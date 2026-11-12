# sdroxide — la traduzione italiana dell'interfaccia. Il riferimento è
# en-US/sdroxide.ftl; ciò che manca qui viene mostrato in inglese.

## The language's own name, as the language picker lists it.
language-name = Italiano

## Values from sdroxide-types, as the screen names them (src/i18n/labels.rs).

layout-mode-auto = Automatico
layout-mode-desktop = Desktop
layout-mode-tablet = Tablet
layout-mode-small = Schermo piccolo
layout-mode-phone = Telefono

ui-theme-default = Predefinito
ui-theme-light = Chiaro
ui-theme-high-contrast = Alto contrasto
ui-theme-green-phosphor = Fosforo verde
ui-theme-amber-phosphor = Fosforo ambra
ui-theme-teal-orange = Verde acqua / arancio
ui-theme-rainbow = Arcobaleno
ui-theme-nord = Nord
ui-theme-nord-dark = Nord scuro
ui-theme-gruvbox = Gruvbox
ui-theme-everforest = Everforest
ui-theme-solarized-dark = Solarized scuro
ui-theme-solarized = Solarized
ui-theme-dracula = Dracula
ui-theme-catppuccin-mocha = Catppuccin mocha
ui-theme-catppuccin-latte = Catppuccin latte
ui-theme-modern-minimalist = Minimalista moderno

chrome-style-angled = Angolare
chrome-style-rectangular = Rettangolare
chrome-style-rounded = Arrotondato
chrome-style-gradient = Sfumato
chrome-style-bevel = Bordo 3D
chrome-style-terminal = Terminale

font-size-small = Piccolo
font-size-medium = Medio
font-size-large = Grande

bandplan-kind-ham = Radioamatori
bandplan-kind-cw = CW
bandplan-kind-digi = Digitale
bandplan-kind-phone = Fonia
bandplan-kind-beacon = Radiofari
bandplan-kind-broadcast = Radiodiffusione
bandplan-kind-am = AM / LW / MW
bandplan-kind-cb = CB

speech-verbosity-terse = Essenziale
speech-verbosity-normal = Normale
speech-verbosity-full = Completo

# The examples in brackets are what the (English) voice says.
speech-freq-style-dial = Sintonia (14 point 074)
speech-freq-style-digits = Ogni cifra
speech-freq-style-full = Sintonia con unità

speech-callsign-style-phonetic = Fonetico (kilo one alpha)
speech-callsign-style-letters = Lettere (kay one ay)
speech-callsign-style-as-is = Come scritto

## Settings window, UI tab (src/app/settings/ui_tab.rs).

settings-ui-display = Schermo
settings-ui-moved-note = L'ampiezza del panadapter, la reazione dello spettro e la velocità di scorrimento della waterfall si impostano con il pulsante SPEC sulla barra del display — con la finestra stretta, il menu DISP.
settings-ui-language = Lingua
    .hint = La lingua dell'interfaccia. Il manuale aperto con F1 e gli annunci vocali restano in inglese, qualunque sia la lingua scelta qui.
settings-ui-language-system = Sistema ({ $language })
settings-ui-layout = Disposizione
settings-ui-theme = Tema
settings-ui-button-style = Stile dei pulsanti
settings-ui-window-style = Stile delle finestre
settings-ui-frame-rate = Frequenza di aggiornamento dello schermo
settings-ui-palette = Palette della waterfall
settings-ui-tuning-buttons = Pulsanti di sintonia
settings-ui-tuning-buttons-show = Mostra su telefono e tablet
    .hint = Una riga di pulsanti abbassa / passo / alza sotto la barra di controllo, su uno schermo a tocco. Non c'è una rotella per scorrere una cifra né una manopola, quindi senza questi pulsanti per spostarsi di un passo noto bisogna digitare l'intera frequenza. Non compaiono mai su desktop.
settings-ui-tuning-step = Passo: { $step }
settings-ui-tuning-snap = Il primo tocco si aggancia al passo
    .hint = Una sintonia che si trova tra due multipli del passo va al multiplo successivo nella direzione premuta — 7 074 300 sale a 7 075 000 con un passo di 1 kHz, scende a 7 074 000. Dopo, i pulsanti si spostano di un passo come sempre. Se disattivato, ogni pressione sposta esattamente di un passo.
settings-ui-smoothing = Levigatura della waterfall
settings-ui-smoothing-interpolate = Interpola
    .hint = Mescola ogni pixel dello schermo con i bin e le righe intorno, così un segnale appare continuo quando il display è più largo della trasformata. Disattivala per una waterfall rettangolare — un blocco per bin, uno per riga — necessaria per leggere la forma di un segnale dall'immagine: un segnale interpolato non si distingue da uno realmente più largo. L'altra metà è una FFT più grande (il chip FFT).
settings-ui-clock = Orologio
settings-ui-clock-show = UTC + ora locale sul panadapter
    .hint = Un orologio in alto a destra del panadapter, in UTC e nell'ora locale.
settings-ui-background = Sfondo dello spettro
settings-ui-background-gradient = Sfumato
settings-ui-background-top = alto
settings-ui-background-bottom = basso
settings-ui-spot-colours = Colori delle etichette spot
    .hint = La tinta di ciascuna sorgente di spot: i riquadri lungo il fondo della waterfall, le etichette nella lista SPOTS e i punti sulla mappa del mondo.
settings-ui-spot-colour = Colore per gli spot { $kind }
settings-ui-spot-colours-reset = Riporta ogni colore degli spot al predefinito
settings-ui-reset = Ripristina
settings-ui-bandplan-colours = Colori del piano di banda
    .hint = La tonalità con cui ogni classe di assegnazione viene dipinta sulla striscia del piano di banda lungo il fondo della waterfall. I blocchi sono disegnati semitrasparenti sopra la waterfall, quindi lì risultano più scuri che nel campione qui.
settings-ui-bandplan-colour = Colore per le assegnazioni { $kind }
settings-ui-bandplan-colours-reset = Riporta ogni colore del piano di banda al predefinito
settings-ui-skimmer-font = Dimensione del carattere dello skimmer
settings-ui-waterfall-font = Dimensione del carattere di waterfall / spettro
settings-ui-interface-font = Dimensione del carattere dell'interfaccia
    .hint = Scala l'intera interfaccia — menu, finestre di dialogo, finestre, le schede della radio, la barra superiore e i suoi pulsanti. Le due dimensioni qui sopra sono relative a questa.
settings-ui-cities = Città sulle mappe
    .hint =
        Disegna le città del mondo — un punto per località, con il nome accanto dove c'è spazio — sulle mappe piane: FT8/WSPR, APRS, ADS-B e AIS.

        Sono la maggior parte di ciò che indica *dove* si trova un punto su una mappa del mondo intero, ed è anche la cosa più affollata. Disattivale se i nomi disturbano le stazioni che stai leggendo. Il globo 3D non cambia — le sue città sono luci del lato notturno, non segnalini, e lì non c'è nulla scritto sopra un contatto.
settings-ui-cities-show = mostra le città e i loro nomi
settings-ui-update-check = Controlla gli aggiornamenti all'avvio
    .hint = Chiede una volta per avvio a sdroxide.com se è stata pubblicata una versione più recente, e lo segnala con un banner chiudibile sopra la waterfall. Non viene inviato nulla oltre alla richiesta stessa.
settings-ui-band-conditions-download = Scarica la previsione delle condizioni di banda
    .hint = Scarica da HAMQSL.com la previsione delle condizioni delle bande HF, al massimo una volta all'ora, per colorare il menu delle bande. Disattivato, il menu delle bande non mostra previsioni a meno che la vista 3D sia aperta. Non viene inviato nulla oltre alla richiesta stessa.
settings-ui-broadcast-schedule-download = Scarica il palinsesto di radiodiffusione
    .hint = Scarica il palinsesto di radiodiffusione in onde corte EiBi (circa 1 MB) da eibispace.de all'inizio di ogni nuova stagione. Disattivato, resta in uso il palinsesto integrato in sdroxide (o l'ultimo scaricato), e Impostazioni → Spot → Scarica il programma ora lo scarica comunque su richiesta.
settings-ui-3d-view = Vista 3D
settings-ui-clouds = Resa delle nuvole
settings-ui-clouds-layered = A strati
settings-ui-clouds-volumetric = Volumetrica
settings-ui-clouds-note = Come il livello NUVOLE nella vista 3D disegna il tempo. A strati sovrappone fette della troposfera ed è l'opzione economica. Volumetrica fa correre un raggio attraverso di esse, così il Sole proietta le sommità delle nuvole sulla superficie sottostante e i fulmini illuminano la tempesta dall'interno, anziché solo schiarirne l'esterno — a un costo per pixel di diverse volte superiore.

## Settings window, UI tab: voice announcements.

speech-title = Annunci vocali
speech-enabled = Comunica le variazioni alla radio
    .hint = Legge ad alta voce ciò che è cambiato, così la radio si può usare senza guardarla
speech-voice = Voce
speech-voice-shipped = Voce di serie
speech-speed = Velocità
    .hint = La voce allunga o comprime la propria cadenza, così il tono non cambia. Oltre circa 2× non diventa più breve.
speech-volume = Volume
speech-output = Uscita
speech-detail = Dettaglio
speech-duck = Abbassa il ricevitore
speech-duck-while = Mentre parla
speech-test = Prova
speech-what = Cosa annunciare
speech-cat-frequency = Frequenza
speech-cat-mode-band = Modo e banda
speech-cat-vfo-split = VFO e split
speech-cat-agc-gain = AGC e guadagno
speech-cat-levels = Pilotaggio, accordo e microfono
speech-cat-ptt = Trasmissione e ricezione
speech-cat-rit-xit = RIT e XIT
speech-cat-memory-scan = Memorie e scansione
speech-cat-band-edge = Uscita da una banda radioamatoriale
speech-cat-notices = Avvisi e messaggi
speech-cat-filters = Filtri, squelch e riduzione del rumore
    .hint = Disattivato di default: cambiano continuamente mentre si insegue un segnale
speech-decoded = Messaggi decodificati
speech-ft8-to-me = Chiamate FT8 per me
speech-ft8-cq = CQ FT8 a cui potrei rispondere
    .hint = Una sera affollata sui venti metri ne produce un centinaio al minuto
speech-js8 = Messaggi JS8 per me
speech-js8-allcall = Anche JS8 @ALLCALL
speech-fsq = Messaggi FSQ per me
speech-include-snr = Includi il rapporto
    .hint = Solo quando il messaggio non ne porta uno proprio — una decodifica che riporta già un numero non riceve anche il nostro
speech-ft8-qso = Il mio scambio FT8
    .hint = Ciò che il sequenziatore sta per inviare, e la chiusura del contatto
speech-reading-text = Lettura ad alta voce del testo decodificato
speech-text-cw = CW
speech-text-rtty-psk = RTTY, PSK, Olivia, THOR, FSQ
speech-text-cw-locked = CW solo quando il decodificatore è agganciato
    .hint = Leggere l'uscita di un decodificatore non agganciato è peggio del silenzio
speech-text-note = Entrambi sono disattivati di default. Un decodificatore produce testo più in fretta di quanto la voce riesca a leggerlo, quindi tutto ciò che resta troppo indietro rispetto all'audio in diretta viene scartato anziché messo in coda.
speech-tuning-up = Accordo in corso
speech-swr-while-tuning = Leggi il ROS mentre TUNE è tenuto premuto
speech-swr-every = Ogni
speech-swr-summary = Riporta la migliore adattazione al rilascio
speech-swr-alarm = Avvisa del ROS alto durante ogni trasmissione
speech-how-read = Come vengono letti i dati
speech-frequencies = Frequenze
speech-callsigns = Nominativi
speech-quiet-on-tx = Resta in silenzio durante la trasmissione
    .hint = La voce va agli altoparlanti e quindi nel microfono. Gli avvisi di ROS alto passano comunque.
speech-device-note = Gli annunci vengono riprodotti sul proprio dispositivo audio, quindi non vengono mai registrati né inviati a chi ascolta da remoto. I tasti per pronunciare lo stato, ripetere l'ultimo annuncio e interrompere a metà frase sono nella scheda Controlli.

## Settings window (src/app/settings/mod.rs)

settings-waiting-station-s-network-configuration = In attesa della configurazione di rete della stazione…
settings-device-reports = come riportato dal dispositivo
settings-transverters = Transverter
settings-one-row-per-box-while = Una riga per ogni casella. Quando la sintonia è dentro la banda di una riga, la radio è sintonizzata su sintonia + offset e si applicano la regola di trasmissione e il limite di pilotaggio di quella riga; fuori da ogni riga la radio opera sulle proprie bande. Le righe vengono provate dall'alto in basso.
settings-name = Nome
settings-band-low = Inizio banda
settings-band-high = Fine banda
settings-offset = Offset
settings-transmit = Trasmissione
settings-max-drive = Pilotaggio massimo
settings-off-takes-transverter-out-line = Disattivato toglie questo transverter dalla linea senza perdere la riga — la banda torna a ciò che la radio raggiunge da sola.
settings-t-2-m = 2 m
settings-what-you-call-shown-log = Come lo chiami. Mostrato nel log quando la sintonia lo seleziona.
settings-bottom-band-transverter-works-dial = Il limite inferiore della banda in cui opera questo transverter, sul dial. 144 per i 2 m.
settings-top-band-dial-148-2 = Il limite superiore della banda, sul dial. 148 per i 2 m nelle Regioni 2 e 3, 146 nella Regione 1.
settings-radio-ends-up-dial-offset = La radio finisce su sintonia + offset, quindi un transverter che porta una banda a una I.F. ha valore negativo: i 2 m in una I.F. di 28 MHz sono -116, perché 144 - 116 = 28. Trascina per correggere di un hertz alla volta un oscillatore leggermente fuori.
settings-off-while-converting-receive-only =
    Disattivato durante la conversione: solo ricezione — la riga è un convertitore, non un transverter, e su questa banda non si trasmette nulla.

    Con lo stesso convertitore: funziona in entrambe le direzioni, e la trasmissione usa lo stesso offset.
settings-ceiling-transmit-drive-while-transverter =
    Un limite massimo al pilotaggio in trasmissione mentre questo transverter è selezionato, come percentuale del massimo. L'ingresso I.F. di un transverter accetta milliwatt, e il pilotaggio giusto per le bande proprie della radio lo distruggerebbe, quindi questo è il campo più importante della riga. 100% significa nessun limite.

    L'impostazione Pilotaggio resta sotto questo valore anziché essere spostata, così il numero che usi in HF torna quando la sintonia lascia la banda.
settings-remove = RIMUOVI
settings-add-transverter = AGGIUNGI TRANSVERTER
settings-takes-effect-apply-reconnect-like = Ha effetto con Applica / riconnetti, come l'offset del convertitore qui sopra.
settings-maximum-transmit-drive = Pilotaggio massimo in trasmissione
settings-hard-limit-drive-tune-controls = Un limite rigido oltre il quale i comandi Pilotaggio e TUNE non possono andare, applicato dopo la calibrazione di banda qui sotto così che nulla possa riportare il pilotaggio al di sopra. Impostalo dove la radio eroga la potenza nominale, e tutto il comando Pilotaggio diventa utilizzabile — su un trasmettitore che modula i propri campioni (HPSDR, LimeSDR, PlutoSDR, HackRF) il pilotaggio pieno significa il trasmettitore spalancato, che con un amplificatore ad alto guadagno supera di molto ciò per cui i finali sono nominali. Disattivato significa che i comandi arrivano al pilotaggio pieno.
settings-limit-drive = Limita il pilotaggio a
settings-per-cent-full-drive-key = Percento del pilotaggio pieno. Metti la radio in trasmissione su un carico fittizio e abbassa questo valore finché il misuratore non legge la potenza per cui l'amplificatore è nominale.
settings-applies-immediately-every-band-tune = Ha effetto subito, su ogni banda e anche su TUNE.
settings-transmit-drive-band = Pilotaggio in trasmissione per banda
settings-every-amplifier-has-different-gain = Ogni amplificatore ha un guadagno diverso su ogni banda, quindi una stessa impostazione di Pilotaggio dà una potenza diversa su ciascuna. Misura l'uscita su ogni banda, poi riduci le bande che risultano alte finché non coincidono tutte: imposta Pilotaggio per la banda che ne ha più bisogno (di solito la più alta), e porta le altre al suo livello. Decibel di potenza in uscita, applicati alla fonia, ai modi digitali e a TUNE allo stesso modo. Zero ovunque — il predefinito — non cambia nulla.
settings-other = Altro
settings-clear-all = CANCELLA TUTTO
settings-back-no-calibration-drive-setting = Torna nessuna calibrazione: l'impostazione Pilotaggio raggiunge ogni banda per intero.
settings-applies-immediately-band-you-would = Ha effetto subito, e sulla banda su cui trasmetteresti — dietro un transverter è la banda sulla sintonia, e il limite di pilotaggio del transverter prevale comunque su qualsiasi cosa impostata qui.
settings-settings = Impostazioni
settings-enter-server-s-address-first = inserisci prima l'indirizzo del server
settings-enter-receiver-s-address-first = inserisci prima l'indirizzo del ricevitore, oppure scegline uno tra gli SDR pubblici
settings-no-radio-selected-press-discover = nessuna radio selezionata — premi Cerca, oppure inserisci un indirizzo
settings-enter-radio-s-address-first = inserisci prima l'indirizzo della radio
settings-general = Generale
settings-radio = Radio
settings-ui = Interfaccia
settings-alerts = Avvisi
settings-controls = Controlli
settings-spots = Spot
settings-freedv = FreeDV
settings-uploads = Invii
settings-winlink = Winlink
settings-servers = Server
settings-t-r-switch = Commutatore T/R
settings-amplifier = Amplificatore
settings-remote = Remoto
settings-tle = TLE
settings-profiles = Profili
settings-station = Stazione
settings-enter-digital-mode-ft8-sstv = Inserisci un modo digitale (FT8 / SSTV / …) una volta per caricare i valori salvati.
settings-callsign = Nominativo
settings-grid-square = Quadrato locatore
settings-your-callsign-grid-shared-across = Il tuo nominativo e il locatore, condivisi tra FT8/FT4/FT2, le intestazioni delle immagini SSTV e il log. Modificabili anche dalla finestra di configurazione FT8 / SSTV.
settings-iaru-region = Regione IARU
settings-which-radio-interface-server-uses = L'interfaccia radio usata dal server si imposta sulla macchina su cui gira.
settings-radio-interface = Interfaccia radio
settings-waiting-machine-radio-attached-interface = In attesa della macchina a cui è collegata la radio. Le sue impostazioni di interfaccia sono qui sotto e nel frattempo si possono cambiare da qui.
settings-converter = Convertitore
settings-manual-type-offset-below = Manuale — scrivi un offset qui sotto
settings-what-box-reads-when-offset = Il valore letto da questa casella quando l'offset non è uno dei preset. Si imposta scrivendo nel campo Offset qui sotto; qui non c'è nulla da selezionare.
settings-frequency-converter-front-receiver-pick = Un convertitore di frequenza davanti al ricevitore. Scegline uno, oppure scrivi accanto un offset.
settings-how-far-converter-moves-signal =
    Di quanto un convertitore sposta il segnale sulla via verso il ricevitore, in Hz — lo stesso numero e lo stesso segno che riporta la documentazione di ogni convertitore e di ogni altro programma SDR. La radio finisce su sintonia + offset: positivo per un upconverter (un Ham It Up è 125000000), negativo per tutto ciò che porta una banda a una I.F. — un LNB satellitare, o un transverter per i 2 m su una I.F. di 28 MHz, che è -116000000 (28 − 144). 0 = nessun convertitore.

    Trascina per correggere di un hertz alla volta, che è ciò che serve a un convertitore il cui oscillatore è leggermente fuori.

    Questa è la via di ricezione. Ciò che è nella linea di trasmissione è la riga qui sotto.

    Ha effetto con Applica.
settings-what-transmit-line-while-converter =
    Ciò che è nella linea di trasmissione mentre un convertitore è impostato.

    Disattivato durante la conversione: non si trasmette nulla — il predefinito sicuro, e giusto per un accessorio solo in ricezione.

    Con lo stesso convertitore: un solo box converte in entrambe le direzioni, come un transverter. La trasmissione usa l'offset qui sopra e lo segue quando viene corretto.

    Offset proprio: la linea di trasmissione è diversa da quella di ricezione. Zero è il caso comune — un convertitore (un LNB, un Ham It Up) in ricezione con il trasmettitore sulla propria antenna, che è la stazione QO-100.

    Il controllo sulle bande radioamatoriali vale comunque, sulla frequenza su cui trasmetti.

    Ha effetto con Applica.
settings-how-far-transmit-line-moves =
    Di quanto la linea di trasmissione sposta il segnale, in Hz, con la stessa regola di segno dell'offset di ricezione: la radio trasmette su sintonia + offset.

    0 = il trasmettitore è collegato alla propria antenna e lavora sulla frequenza indicata dalla sintonia — il collegamento in salita a 2,4 GHz di una stazione QO-100.

    Un convertitore in trasmissione che porta una I.F. *in alto* è un numero negativo: la radio lavora sotto la sintonia.
settings-rx-range = Gamma RX
settings-which-frequencies-radio-receives-mhz =
    Quali frequenze riceve questa radio, in MHz: 144-146, 430-440. Lascia vuoto per usare ciò che il dispositivo riferisce di sé.

    I pulsanti di banda fuori dalla gamma sono grigi e la sintonia non ci va.

    Sono frequenze di sintonia. Con un convertitore impostato, indica la banda che sintonizzi — 144-148 per un transverter sui 2 m — non la I.F. su cui la radio opera davvero.

    Ha effetto con Applica.
settings-tx-range = Gamma TX
settings-which-frequencies-radio-transmits-mhz =
    Su quali frequenze trasmette questa radio, in MHz: 144-146, 430-440. Lascia vuoto per usare ciò che riferisce il dispositivo — e se non riferisce nulla, si dà credito al driver e ogni frequenza è consentita.

    Frequenze di sintonia, come la gamma di ricezione qui sopra.

    È un limite che imposti tu, non una licenza: la trasmissione fuori dalle bande radioamatoriali viene comunque rifiutata, a meno che tu non l'abbia disattivato in config.toml. Né dà un trasmettitore a un dispositivo solo in ricezione.

    Ha effetto con Applica.
settings-antenna = L'antenna è
settings-where-radio-listens-station-antenna =
    Da dove ascolta questa radio.

    Alla stazione: l'antenna è tua, quindi il tuo locatore indica dove si trova. È il predefinito, e va bene per tutto ciò che è nello shack.

    Altrove: un ricevitore online, o un tuo impianto su una collina. Ciò che riceve viene riportato a PSK Reporter, WSPRnet e FreeDV Reporter con il locatore accanto a questo, mai con il tuo — e anche ADS-B colloca gli aerei rispetto a questo.

    Scegliere un ricevitore sotto "SDR pubblici" compila questo campo per te.

    Ha effetto con Applica.
settings-receiver-s-maidenhead-locator-jn88ec =
    Il locatore Maidenhead del ricevitore — JN88ec, DO30db. Quattro o sei caratteri.

    Lascialo vuoto se non sai dove si trova il ricevitore: in quel caso non viene riportato nulla, ed è la sola risposta onesta. Riportare dal proprio quadrato significherebbe mettere la ricezione di qualcun altro nel continente sbagliato.
settings-audio = Audio verso
settings-gain-per-band = Guadagno per banda
settings-remember-receive-gain-per-band = Ricorda il guadagno di ricezione per banda
settings-each-band-keeps-receive-gain = Ogni banda conserva il guadagno di ricezione impostato l'ultima volta, e lo riprende quando la sintonia vi ritorna — un pulsante di banda, un trascinamento oltre il bordo di una banda o la manopola della radio. La copertura generale è divisa in HF, VHF, UHF e oltre. Una banda su cui non hai impostato un guadagno mantiene quello in vigore. Disattivato: un solo guadagno ovunque.
settings-s-meter-calibration = Calibrazione dell'S-meter
settings-added-radio-s-s-meter =
    Aggiunto alla lettura S-meter di questa radio. Il misuratore di un SDR parte leggendo in dBFS — livello rispetto al fondo scala del convertitore — quindi un ricevitore con molto guadagno può mostrare S9 senza alcuna antenna collegata. Imposta questo valore sulla differenza tra ciò che il misuratore mostra e un livello che conosci: un generatore di segnale, oppure una banda silenziosa di cui ti fidi che stia intorno a S2–S3. Ha effetto subito.

    Si somma a cal_offset_db in config.toml, che è un valore unico per tutta la stazione.
settings-t-0-db = 0 dB
settings-antenna-somewhere-else-no-locator = L'antenna è altrove e non è impostato alcun locatore, quindi nulla di ciò che questa radio sente viene riportato a PSK Reporter, WSPRnet o FreeDV Reporter. Scrivi il locatore del ricevitore qui sopra per riportare da lì.
settings-ranges-are-mhz-low-high = Le gamme sono in MHz, da-a, separate da virgole: 144-146, 430-440 — cioè 144000000-146000000 Hz e 430000000-440000000 Hz. Sono frequenze di sintonia: un offset del convertitore non le sposta. L'offset del convertitore qui sopra è il campo in hertz. Lascia vuota una gamma per usare ciò che il dispositivo riferisce di sé; un dispositivo che non riferisce nulla viene preso sulla parola.
settings-careful-rtl-sdr-blog-v4 = Attenzione con un RTL-SDR: il Blog V4 fa un upconversion da solo sotto 28,8 MHz, quindi un offset negativo che porta l'hardware lì sposta il segnale due volte.
settings-transmit-audio = Audio di trasmissione
settings-use = in uso
settings-every-stage-off = tutti gli stadi disattivati
settings-open-tx-audio = Apri Audio TX…
settings-microphone-processing-chain-gate-eq = La catena di elaborazione del microfono — gate, equalizzatore, de-esser, compressore e limitatore — con una registrazione di prova per regolarla a orecchio senza trasmettere. Solo modi vocali. Si apre anche cliccando Mic nel riquadro TX.
settings-pick-radio-interface-above-configuration = Scegli un'interfaccia radio qui sopra (questa configurazione usava la modalità di rilevamento automatico, ormai rimossa).
settings-configuration-used-removed-auto-detect = Questa configurazione usava la modalità di rilevamento automatico, ormai rimossa. Scegli un'interfaccia qui sopra quando la macchina a cui è collegata la radio ha risposto.
settings-radio-has-no-interface-yet = Questa radio non ha ancora un'interfaccia — scegline una qui sopra e premi Applica / riconnetti.
settings-radio-has-no-interface-yet-2 = Questa radio non ha ancora un'interfaccia. Se ne può scegliere una qui sopra quando la macchina a cui è collegata la radio ha risposto.
settings-apply-reconnect = Applica / riconnetti
settings-switch-interface-now-no-restart = Passa subito a questa interfaccia — nessun riavvio necessario
settings-switch-server-s-radio-interface = Passa subito la radio del server a questa interfaccia — nessun riavvio necessario. La vecchia resta attiva se la nuova non può essere aperta.
settings-switches-live-radio-without-restarting = Cambia la radio in funzione senza riavviare.
settings-everything-above-server-s-own = Tutto ciò che è qui sopra è la configurazione propria del server, e le modifiche vengono salvate lì. La maggior parte delle impostazioni ha effetto mentre le cambi; quelle fissate all'apertura del dispositivo — la frequenza di campionamento, un indirizzo, l'interfaccia stessa — richiedono questo pulsante.
settings-dx-cluster-telnet = DX cluster (telnet)
settings-enabled = Attivo
settings-host = Host
settings-port = Porta
settings-login-call = Nominativo di accesso
settings-commands = Comandi
settings-one-per-line-e-g = uno per riga, ad es. SET/FT8
settings-reverse-beacon-network = Reverse Beacon Network
settings-read-world-s-cw-rtty = Legge gli skimmer CW/RTTY del mondo e alimenta con essi la mappa di propagazione. È ciò che fa mostrare alla mappa le bande su cui questa radio non è in ascolto. Attivo di default: non trasmette nulla, non richiede un account e usa il nominativo della scheda Generale. Gli spot RBN non compaiono nella lista degli spot — sono misure, non inviti.
settings-t-7000-cw-rtty-feed-7001 = 7000 è il flusso CW/RTTY, 7001 quello FT8/FT4
settings-one-per-line-e-g-2 = uno per riga, ad es. set/filter cont=eu
settings-sent-after-login-place-narrow = Inviato dopo l'accesso. È il punto in cui restringere il flusso — senza un filtro sono tutti gli skimmer della Terra.
settings-rbn-paths-are-placed-country = I percorsi RBN sono collocati dai centri dei paesi, non dai locatori — precisi per un paese piccolo, lontanissimi per uno grande. Sono un livello separato sulla mappa di propagazione e si possono disattivare lì.
settings-pota-sota-psk-reporter = POTA / SOTA / PSK Reporter
settings-pota-activator-spots = Spot di attivatori POTA
settings-sota-spots = Spot SOTA
settings-psk-reporter-current-band = PSK Reporter (banda corrente)
settings-upload-my-ft8-ft4-ft2 = Invia le mie decodifiche FT8/FT4/FT2/JS8
settings-report-what-station-hears-pskreporter = Riporta a pskreporter.info ciò che questa stazione sente, così compare lì come ricevitore. Usa il nominativo e il locatore della scheda Generale.
settings-antenna-2 = Antenna
settings-collector = Collettore
settings-t-4739-live-collector-14739-test = 4739 è il collettore in funzione, 14739 quello di prova
settings-max-age-s = Età massima (s)
settings-wsprnet = WSPRnet
settings-upload-my-wspr-decodes = Invia le mie decodifiche WSPR
settings-send-every-wspr-reception-wsprnet = Invia ogni ricezione WSPR a wsprnet.org. Attivo di default: non trasmette nulla, e riportare ciò che si sente è ciò che fa di un ricevitore WSPR una parte della rete anziché una curiosità privata. Viene riportato anche un intervallo in cui non è stato decodificato nulla, ed è così che la rete distingue una banda chiusa da un ricevitore spento.
settings-download-who-heard-me = Scarica chi mi ha sentito
settings-ask-wsprnet-org-which-stations = Chiede a wsprnet.org quali stazioni hanno decodificato questa. WSPR non prevede alcuna forma di conferma, quindi è l'unico modo in cui un radiofaro in trasmissione viene a sapere qualcosa della propria portata.
settings-ask-every = Chiedi ogni
settings-looking-back = guardando indietro
settings-apply = {" "}APPLICA{" "}
settings-persist-re-connect-feeds = Salva e (ri)collega i flussi
settings-broadcast-stations = Stazioni di radiodiffusione
settings-winlink-account = Account Winlink
settings-password = Password
settings-winlink-account-password-not-gateway = La password dell'account Winlink, non quella del gateway. Fa distinzione tra maiuscole e minuscole — inseriscila esattamente come ti è stata rilasciata.
settings-locator = Locatore
settings-how-connect = Come collegarsi
settings-route = Percorso
settings-internet = Internet
settings-forward-cms-over-internet = Inoltra con il CMS via internet
settings-radio-packet = Radio (pacchetto)
settings-call-rms-gateway-air-radio = Chiama un gateway RMS via radio. La radio deve essere in PACKET o PACKET-HF.
settings-gateway = Gateway
settings-via = Via
settings-digipeaters-order-separated-spaces-usually = Digipeater, in ordine, separati da spazi. Di solito vuoto — un gateway che senti direttamente è un gateway da chiamare direttamente.
settings-speed = Velocità
settings-most-rms-gateways-answer-1200 = La maggior parte dei gateway RMS risponde a 1200. Chiamare un gateway a 1200 a 9600 suona esattamente come un gateway spento, quindi imposta questo valore in base a ciò che pubblica il titolare del gateway, senza indovinare. 9600 richiede inoltre la porta dati della radio a entrambi i capi. 300 è HF.
settings-frequency = Frequenza
settings-clear = CANCELLA
settings-zero-leaves-dial-alone-which = Zero lascia in pace la sintonia, che è ciò che vuoi quando resti su un solo canale. Qualsiasi altro valore sintonizza la radio all'avvio della sessione.
settings-my-gateways = I miei gateway
settings-winlink-s-published-gateway-list = L'elenco dei gateway pubblicato da Winlink richiede una chiave API che sdroxide non ha, quindi tieni il tuo. I due o tre gateway raggiungibili da una stessa località si imparano provando, e cambiano di rado.
settings-use-2 = USA
settings-call-one-next-connect = Chiama questo alla prossima connessione
settings-current-dial = sintonia attuale
settings-forget = DIMENTICA
settings-forget-gateway = Dimentica questo gateway
settings-add-gateway = + AGGIUNGI GATEWAY
settings-save-gateway-above-list = Salva il gateway qui sopra nell'elenco
settings-internet-gateway = Gateway internet
settings-cms-address = Indirizzo CMS
settings-client-name = Nome del client
settings-winlink-s-production-servers-only = I server di produzione di Winlink accettano solo nomi di client che conoscono, e rispondono a un nome sconosciuto con "Unknown client types are not allowed on production servers". Finché sdroxide non sarà registrato presso il Winlink Development Team, per collegarsi serve un nome che lo riconoscano.
settings-automatic-connection = Connessione automatica
settings-connect-timer = Collega a intervalli
settings-every = Ogni
settings-persist-winlink-account = Salva l'account Winlink
settings-callsign-lookup = Ricerca nominativi
settings-provider = Fornitore
settings-auto-fill-name-qth-grid = Compila automaticamente nome/QTH/locatore al clic su uno spot e per il QSO
settings-choose-provider-enter-login = Scegli un fornitore per inserire il suo accesso.
settings-qrz-user = Utente QRZ
settings-qrz-pass = Password QRZ
settings-hamqth-user = Utente HamQTH
settings-hamqth-pass = Password HamQTH
settings-one-hamqth-account-does-both = Un solo account HamQTH fa entrambe le cose — questo accesso è quello usato dalla scheda di invio HamQTH.
settings-upload = Invio
settings-auto-upload-each-new-qso = Invia automaticamente ogni nuovo QSO
settings-master-switch-which-services-qso = L'interruttore generale. I servizi a cui va un QSO si impostano in ciascuna scheda qui sotto.
settings-turn-auto-upload-each-new = Attiva prima "Invia automaticamente ogni nuovo QSO" qui sopra — un servizio selezionato qui non viene inviato finché non lo è.
settings-auto-upload-off-above-so = L'invio automatico è disattivato qui sopra, quindi le selezioni dei servizi sono disabilitate e per ora non si invia nulla in automatico — il pulsante UP per singolo QSO nel log funziona comunque.
settings-qrz-log-key = Chiave del log QRZ
settings-qrz-logbook-api-key-your = La chiave API del logbook QRZ, dalle impostazioni del tuo logbook QRZ — non l'accesso per la ricerca XML.
settings-eqsl-user = Utente eQSL
settings-eqsl-pass = Password eQSL
settings-eqsl-qth = QTH eQSL
settings-qth-nickname-eqsl-profile-upload = Il soprannome QTH del profilo eQSL su cui inviare e da cui scaricare — serve solo quando l'account ne possiede più di uno; altrimenti lascialo vuoto.
settings-same-login-hamqth-callsign-lookup = Lo stesso accesso della ricerca nominativi HamQTH — compilarne uno compila entrambi.
settings-club-log-email = Email di Club Log
settings-club-log-pass = Password di Club Log
settings-club-log-key = Chiave di Club Log
settings-wrl-api-key = Chiave API WRL
settings-developer-api-key-world-radio = La chiave API per sviluppatori di World Radio League → Integrations → Developer API. Viene mostrata una sola volta quando la generi, quindi copiala subito. I contatti vanno al tuo logbook predefinito — impostane uno in WRL se ne hai più di uno.
settings-confirmations-download = Conferme (download)
settings-lotw-user = Utente LoTW
settings-lotw-pass = Password LoTW
settings-lotw-upload-uses-tqsl-export = L'invio a LoTW usa TQSL — esporta l'ADIF dal logbook e firmalo. Le conferme LoTW/eQSL vengono scaricate qui per distinguere i collegamenti lavorati da quelli confermati.
settings-sync-confirmations = SINCRONIZZA CONFERME
settings-fetching-subscriptions = Recupero degli abbonamenti…
settings-spectrum-waterfall-everything-read-off =
    Il suo spettro, la sua waterfall e tutto ciò che si legge da esse stanno nella scheda di quella radio, ed è per questo che non ne ha una propria. Le impostazioni d'interfaccia qui sopra restano di questa radio e continuano ad applicarsi — dopo averle cambiate premi Applica / riconnetti e le due radio ripartono insieme.

    Per usarla di nuovo da sola, svuota la casella Ricevitore del panadattatore nella pagina di quella radio.
settings-panadapter = Panadapter
settings-give-radio-wideband-panadapter-borrowing = Dai a questa radio un panadapter a banda larga prendendo in prestito il ricevitore di un'altra radio — un SDR sulla stessa antenna, oppure alimentato dall'uscita I.F. di questa radio. Lo spettro, la waterfall, il sub-ricevitore, i modi digitali e lo skimmer vengono tutti da quel ricevitore; sintonia, modo, filtro e trasmettitore restano qui.
settings-receiver = Ricevitore
settings-none = Nessuno
settings-which-radio-receives-one-leaves =
    Quale radio riceve per questa. Mentre è prestata esce dalla barra delle schede — il suo front-end appartiene al motore di questa radio — e ritorna subito quando questa viene impostata su Nessuno.

    Ha effetto con Applica.
settings-connected = Collegato a
settings-how-far-above-radio-s =
    Di quanto il ricevitore si trova sopra la sintonia di questa radio, in hertz — 0 per un ricevitore sulla stessa antenna. Per una derivazione I.F., la frequenza intermedia su cui è fermo il ricevitore: 8215000 per il KXV3B di un K3, 9000000 per una I.F. a 9 MHz.

    Questo non è il campo Convertitore qui sopra, che sintonizza di nuovo la radio, né l'offset del centro I/Q della scheda CAT, che riguarda la scheda audio della radio stessa. Ciò che viene scritto qui non va mai inviato a nessuna delle due radio.

    Ha effetto con Applica.
settings-audio-2 = Audio da
settings-follow-dial = Segui la sintonia
settings-keep-receiver-radio-s-dial = Mantieni il ricevitore sulla sintonia di questa radio, e sposta la sintonia quando sintonizzi sul panadapter. Disattivato, il ricevitore resta dov'è — un ascolto fisso su un segmento mentre la radio va altrove.
settings-invert-spectrum = Inverti lo spettro
settings-mirror-receiver-s-span-about = Specchia l'ampiezza del ricevitore rispetto al suo centro, per una derivazione il cui oscillatore sta sopra il segnale e che consegna la banda al contrario. Lascialo disattivato a meno che la waterfall non si riempia di segnali tutti dalla parte sbagliata della sintonia.
settings-mute-transmit = Muto in trasmissione
settings-silence-receive-audio-while-radio = Silenzia l'audio in ricezione mentre questa radio trasmette. Attivo di default: con il ricevitore sulla stessa antenna, o sulla I.F. di questa radio, ciò che si sente durante una trasmissione è il proprio trasmettitore. Anche le trasmissioni avviate sulla radio stessa — il PTT del microfono, un pedale, MOX — vengono contate, appena il collegamento CAT segnala che l'apparato è passato in trasmissione.
settings-blank-transmit = Oscura in trasmissione
settings-stop-panadapter-waterfall-while-radio = Ferma il panadapter e la waterfall mentre questa radio trasmette. Attivo di default: un trasmettitore che dipinge l'intera ampiezza cancella la banda dietro di sé. Disattivalo per guardare la banda — o il proprio segnale — durante una trasmissione, cosa utile con il ricevitore su un'antenna separata. Vale per le trasmissioni avviate da sdroxide; una avviata sulla radio viene ricevuta fino in fondo, quindi l'immagine continua comunque, qualunque cosa dica questo.
settings-per-mode-offsets-radio-whose = Offset per modo, per una radio la cui I.F. si sposta con il modo. Un modo lasciato vuoto usa l'offset qui sopra.
settings-computer = Su questo computer
settings-add-radio-computer = Aggiungi una radio su questo computer
settings-radio-s-settings-are-below = Le impostazioni di questa radio sono qui sotto. Trascina per spostarla lungo la barra.
settings-switch-radio-dialog-follows-drag = Passa a questa radio (la finestra segue). Trascina per spostarla lungo la barra.
settings-tx = ● TX
settings-station-radio = Stazione:
settings-station-radio-tip = La radio della stazione. Esegue i servizi di rete della stazione (flussi di spot, rotore, amplificatore, commutatore TX/RX) e riceve le impostazioni della riga di comando, per questo resta nell'elenco. Per metterla a riposo, spegnila con COLLEGAMENTO; per usare altro hardware, scegli un'altra interfaccia qui sotto.
settings-mute-radio-s-audio = Silenzia l'audio di questa radio
settings-link = COLLEGAMENTO
settings-close-radio-configuration-kept = Chiudi questa radio (la sua configurazione viene conservata)
settings-close-tab-close-radio-station = Chiudi questa scheda, oppure chiudi la radio sulla sua stazione
settings-close-tab = Chiudi questa scheda
settings-transverters-full = { $max } è il massimo che questa tabella può contenere
settings-dialling = Composizione { $url }…
settings-region-note = Dove si trova la stazione. Imposta ogni piano di banda: i limiti delle bande, i sotto-segmenti CW / dati / fonia sulla striscia della waterfall, dove ascoltano gli skimmer, le frequenze di chiamata proposte e ciò che conta come fuori banda in trasmissione. Nella Regione { $region } questo fa sì che i 70 cm { $region ->
        [1] siano 430–440 MHz — quindi 446 è fuori banda, e i 40 m si fermano a 7.200
        [2] siano 420–450 MHz, e i 40 m arrivano a 7.300
       *[3] siano 430–450 MHz, e gli 80 m si fermano a 3.900
    }. Ha effetto subito e vale per ogni radio di questa stazione.
settings-rx-grid-reported = Le ricezioni attraverso questa radio vengono riportate dal locatore { $grid }, non dal tuo.
settings-rx-grid-invalid = { $grid } non è un locatore Maidenhead, quindi non viene riportato nulla di ciò che sente questa radio. Quattro o sei caratteri: due lettere, due cifre e, facoltativamente, altre due lettere.
settings-bypassed = esclusa
# $state is one of settings-bypassed, settings-use or settings-every-stage-off.
settings-tx-audio-chain = Catena { $state }.
settings-gateway-via = { " " }via { $via }
settings-auto-upload-to = Invia automaticamente ogni nuovo QSO a { $service }
settings-tle-updated = Aggiornat{ $n ->
        [one] o
       *[other] i
    } { $n } { $n ->
        [one] abbonamento
       *[other] abbonamenti
    }: { $total } satelliti.
settings-tle-updated-failed = Aggiornati { $ok } su { $n }; { $failed } non riusciti — vedi le righe qui sopra.
settings-panadapter-of = Questa radio è il panadapter di { $radio }
settings-radio-n = radio { $n }
settings-on-station = Su { $station }
settings-add-radio-on-station = Aggiungi una radio su { $station } — la stazione la serve subito
settings-panadapter-receiver-for = Ricevitore del panadapter per { $radio }
settings-hang-up-stays-on = Riaggancia. La radio resta su { $station }.
settings-close-on-station = Chiudere { $radio } su { $station }? La sua configurazione resta su quella macchina; la scheda qui si chiude con essa, e con essa quella di tutti gli altri.
settings-close-it-on = Chiudila su { $station }
settings-locator-hint = locatore

## Settings window, Alerts (src/app/settings/alerts.rs)

settings-alerts-audible-alerts = Avvisi sonori
settings-alerts-sound-alarm-when-decode-matters = Suona un allarme quando una decodifica è importante
settings-alerts-plays-over-own-audio-output = Viene riprodotto sulla propria uscita audio, così si sente anche quando la banda è in un altoparlante diverso da questo schermo.
settings-alerts-volume = Volume
settings-alerts-output = Uscita
settings-alerts-test = Prova
settings-alerts-what-sound = Cosa far suonare
settings-alerts-each-station-quiet-while-after = Ogni stazione resta silenziosa per un po' dopo un avviso, così una banda affollata non fa suonare ogni slot. Voce e Tono + voce sono letti dalla voce degli annunci parlati — attivala in Impostazioni → Interfaccia.

## Settings window, Amplifier (src/app/settings/amp.rs)

settings-amp-amplifier = Amplificatore
settings-amp-waiting-station-s-amplifier-configuration = In attesa della configurazione dell'amplificatore della stazione…
settings-amp-hobbypcb-hardrock-500-hardrock-50 = Un HobbyPCB Hardrock-500 o Hardrock-50 sulla sua porta seriale USB. SDRoxide legge i suoi strumenti, ne commuta banda, modo e antenna, e ne controlla il sintonizzatore dalla finestra AMP. L'amplificatore resta comandato dalla linea PTT della radio come prima.
settings-amp-enable = Attiva
settings-amp-model = Modello
settings-amp-untested-panel-has-only-been = Non testato: questo pannello è stato provato solo con un Hardrock-500. I limiti dello strumento per questo modello sono scalati da quelli dell'Hardrock-500 e potrebbero essere sbagliati. Controlla il display dell'amplificatore stesso.
settings-amp-port = Porta
settings-amp-choose = — scegli —
settings-amp-baud = Baud
settings-amp-must-match-amplifier-s-menu = Deve corrispondere al menu dell'amplificatore. Il pannello di controllo di HobbyPCB richiede 115200.
settings-amp-follow-radio-s-band = Segui la banda della radio
settings-amp-sends-transmit-frequency-amplifier-every = Invia la frequenza di trasmissione all'amplificatore a ogni cambio di banda, così i suoi filtri seguono la radio senza un cavo CAT proprio. Mai inviata durante una trasmissione.
settings-amp-show-power-swr-tx-meter = Mostra la sua potenza e il ROS sullo strumento TX
settings-amp-only-where-radio-does-not = Solo dove la radio non le misura da sé, e solo mentre l'amplificatore è in linea.
settings-amp-off = Disattivato
settings-amp-connecting = Connessione…
settings-amp-status = Stato
settings-amp-apply = {" "}APPLICA{" "}
settings-amp-save-re-open-amplifier-s = Salva e (ri)apri la porta dell'amplificatore

## Settings window, Controls (src/app/settings/controls.rs)

settings-controls-keyboard = Tastiera
settings-controls-click-shortcut-rebind-then-press = Clicca una scorciatoia per riassegnarla, poi premi la combinazione di tasti (Esc annulla). Le assegnazioni vengono ignorate mentre scrivi in un campo di testo.
settings-controls-shortcut = Scorciatoia
settings-controls-does = Fa
settings-controls-step-mode = Passo / modo
settings-controls-accel = Accel
settings-controls-on = Attivo
settings-controls-press-key = premi un tasto…
settings-controls-remove-binding = Rimuovi questa assegnazione
settings-controls-add-shortcut = + Aggiungi scorciatoia
settings-controls-restore-defaults = Ripristina i predefiniti
settings-controls-bind-hold-talk-space = Assegna il parla-a-tenere alla barra spaziatrice
settings-controls-hold-space-transmit-releasing-losing = Tieni premuta la barra spaziatrice per trasmettere; rilasciandola — o perdendo il fuoco della finestra — si smette di trasmettere
settings-controls-unkey-held-ptt-after = Interrompi un PTT tenuto premuto dopo
settings-controls-backstop-against-stuck-key-controller = Protezione contro un tasto bloccato o un controller che smette di inviare dati. 0 disattiva.
settings-controls-panadapter-mouse = Mouse sul panadapter
settings-controls-wheel = Rotella
settings-controls-wheel-shift = Rotella + Maiusc
settings-controls-tune-step = Passo di sintonia
settings-controls-zoom-rate = Velocità di zoom
settings-controls-click-tune-rounding = Arrotondamento della sintonia al clic
settings-controls-invert-wheel-direction = Inverti la direzione della rotella
settings-controls-left-drag-tunes-well-pans = Il trascinamento sinistro sintonizza oltre a spostare la vista
settings-controls-off-makes-left-drag-pan = Disattivato, il trascinamento sinistro sposta solo la vista, come quello destro.
settings-controls-scroll-digit-frequency-readout-tune = Scorri una cifra sulla lettura di frequenza per sintonizzarla
settings-controls-both-wheel-actions-are-tune = Entrambe le azioni della rotella sono Sintonia — non resta alcun modo per zoomare.
settings-controls-restore-mouse-defaults = Ripristina i predefiniti del mouse
settings-controls-mouse-buttons = Pulsanti del mouse
settings-controls-left-right-buttons-are-reserved = I pulsanti sinistro e destro sono riservati alla sintonia e allo spostamento; il pulsante centrale e quelli extra sono liberi. Un pulsante laterale tenuto premuto per il PTT si comporta come un pedale.
settings-controls-add-mouse-button = + Aggiungi pulsante del mouse
settings-controls-f1-always-opens-manual-even = F1 apre sempre questo manuale, anche mentre scrivi, quindi non è riassegnabile.
settings-controls-midi-controller = Controller MIDI
settings-controls-midi-controllers-need-native-app = I controller MIDI richiedono l'app nativa — il client nel browser non ha accesso al MIDI.
settings-controls-any-class-compliant-midi-surface = Funziona qualsiasi superficie MIDI conforme alla classe: la rotella jog di un controller DJ fa una buona manopola del VFO, i suoi pad fanno PTT e pulsanti di banda, i suoi fader fanno controlli di guadagno.
settings-controls-enable = Attiva
settings-controls-controller = Controller
settings-controls-feedback = Retroazione verso
settings-controls-rescan-ports = Riesamina le porte
settings-controls-not-connected = Non collegato.
settings-controls-move-control-see-here = Muovi un comando per vederlo qui.
settings-controls-control = Comando
settings-controls-reads = Legge come
settings-controls-led = LED
settings-controls-move = muovilo…
settings-controls-click-then-move-control-you = Clicca, poi muovi il comando che vuoi assegnare
settings-controls-speed-sensitivity-spin-faster-tune = Sensibilità di velocità: gira più in fretta per sintonizzare più in fretta
settings-controls-send-current-value-back-light = Rimanda il valore attuale, per accendere un LED o muovere un fader
settings-controls-add-midi-control = + Aggiungi comando MIDI
settings-controls-clear-all = Cancella tutto
settings-controls-endless-jog-encoders-send-relative = Gli encoder infiniti (jog) inviano un passo relativo anziché una posizione, in una di tre codifiche che da piccoli movimenti sembrano uguali. IMPARA indovina da una rotazione in senso orario; se poi la manopola sintonizza nel verso sbagliato, spunta “rev”.
settings-controls-none = — nessuno —

## Settings window, General (src/app/settings/general.rs)

settings-general-system-default = Predefinito di sistema
settings-general-remote-access = Accesso remoto
settings-general-signed-in-station = Stazione con accesso effettuato
settings-general-sign-out = {" "}ESCI{" "}
settings-general-sign-out-forgets-device = Dimentica l'accesso di questo dispositivo. La stazione lo chiederà di nuovo alla prossima connessione; questa resta attiva finché non la chiudi.
settings-general-password-kept-no-cookie = In questo browser viene ricordata una password per { $stations }, perché non è stato fornito alcun cookie di accesso. Le stazioni più vecchie fanno così; non è la stessa cosa.
settings-general-what-remote-client-browser-page = Ciò che un client remoto — la pagina nel browser, oppure un altro sdroxide avviato con --connect — deve fornire prima che questa stazione lo lasci operare. Vale in modalità server (--server); il prossimo accesso riprende una modifica, senza riavvio.
settings-general-username = Nome utente
settings-general-password = Password
settings-general-clients-must-give-password-leaving = I client devono fornire la password. Lasciare vuoto il nome utente va bene.
settings-general-clients-must-sign = I client devono accedere.
settings-general-empty-anyone-who-can-reach = ⚠ Vuoto: chiunque possa raggiungere la porta del server può operare questa radio, con il tuo nominativo. Imposta una password prima di inoltrare la porta.
settings-general-stored-clear-config-toml-like = Salvata in chiaro in config.toml, come le altre password che sdroxide conserva.
settings-general-receive-audio-gain = Guadagno audio in ricezione
settings-general-t-0-db = 0 dB
settings-general-extra-gain-everything-radio-sends =
    Guadagno extra su tutto ciò che questa radio invia agli altoparlanti, oltre al controllo del volume. Lascialo a 0 dB a meno che la radio non sia debole a volume pieno: il fondo della scala del volume è l'audio così come arriva, quindi può abbassare una radio ma mai alzarla, e l'uscita audio USB di alcuni ricetrasmettitori sta ben sotto il fondo scala.

    Sali di 6 dB alla volta. Troppo satura — l'audio viene limitato al fondo scala anziché lasciato girare su sé stesso, quindi esagerare suona aspro, non forte. Le registrazioni vengono prese prima di questo e non ne sono toccate.
settings-general-reload-band-plan = RICARICA PIANO DI BANDA
settings-general-re-read-bandplan-json-machine = Rilegge bandplan.json sulla macchina a cui è collegata la radio e lo applica — limiti di banda, sotto-segmenti e finestre degli skimmer — senza riavviare.
settings-general-running-station-s-own-band = In funzione con il piano di banda proprio della stazione.
settings-general-running-built-iaru-tables = In funzione con le tabelle IARU integrate.
settings-general-settings-file = File delle impostazioni
settings-general-settings-are-machine-radio-attached = Le impostazioni si trovano sulla macchina a cui è collegata la radio. Esportale da lì.
settings-general-export = ESPORTA…
settings-general-write-every-setting-station-radios = Scrive ogni impostazione di questa stazione — le radio, i modi, i server, le memorie, il piano di banda — in un unico file che puoi portare su un'altra installazione. Il tuo logbook e le eventuali password dei server salvate restano qui.
settings-general-import = IMPORTA…
settings-general-replace-station-s-settings-ones = Sostituisce le impostazioni di questa stazione con quelle di un file esportato da un'altra installazione. Riavvia sdroxide dopo.
settings-general-import-overwrites-what-here-file = Un'importazione sovrascrive ciò che c'è qui, file per file, e ha effetto la prossima volta che sdroxide si avvia — altrimenti le impostazioni già in memoria le riscriverebbero sopra. Ciò che il file non menziona resta com'è, quindi un pacchetto di una stazione con una sola radio non rimuove una seconda radio qui. Il tuo logbook non è mai nel file: se vuoi spostare anche quello, esportalo come ADIF dalla finestra LOG.
settings-general-sdroxide-settings = Impostazioni di sdroxide
settings-general-swr-guard = Protezione ROS
settings-general-stop-transmitting-high-swr = Interrompi la trasmissione con ROS alto
settings-general-trip = Scatta a
settings-general-stops-transmission-when-radio-reports =
    Interrompe la trasmissione quando la radio riporta un ROS pari o superiore a questo valore, e blocca la trasmissione finché non lo confermi. Intercetta un'antenna scollegata, una linea di alimentazione guasta o un commutatore lasciato sulla porta sbagliata.

    La sintonia è trattata diversamente, perché alimentare un disadattamento è proprio il suo scopo: un accordatore d'antenna ha il doppio del limite e circa cinque secondi prima che la protezione si applichi del tutto. Un accordatore manuale che impiega più di così richiede di disattivare la protezione per la sessione.

    Serve un apparato che riporti il ROS via CAT. Ignora il primo quinto di secondo di ogni trasmissione e non attende la potenza piena.
settings-general-per-mode-settings = Impostazioni per modo
settings-general-reset-every-mode = AZZERA OGNI MODO
settings-general-forget-every-mode-s-remembered = Dimentica l'AGC, lo squelch, la riduzione del rumore, il notch e gli interruttori stereo memorizzati per ogni modo, e ripristina i valori predefiniti di ciascun modo.
settings-general-selecting-mode-lays-mode-s = Selezionare un modo imposta sul ricevitore i valori iniziali propri di quel modo per AGC, squelch, riduzione del rumore, notch e interruttori stereo, e modificarne uno lo memorizza solo per quel modo. Il pulsante a freccia circolare in fondo alla riga filtro/rumore del ricevitore elenca ciò che è stato cambiato e riporta il modo attuale.
settings-general-your-audio-speakers-microphone = Il tuo audio (altoparlanti / microfono)
settings-general-output = Uscita
settings-general-channels = Canali
settings-general-which-speaker-channel-s-received = A quale/i canale/i dell'altoparlante arriva l'audio ricevuto. Sinistra o destra porta tutto, mixato in mono, solo su quel lato.
settings-general-input = Ingresso

## Settings window, network services (src/app/settings/net.rs)

settings-net-operator = Operatore
settings-net-callsign-grid-are-set-general = Il nominativo e il locatore si impostano nella scheda Generale.
settings-net-set-your-callsign-grid-general = ⚠ Imposta il tuo nominativo e il locatore nella scheda Generale.
settings-net-freedv-reporter = FreeDV Reporter
settings-net-enable = Attiva
settings-net-connects-whenever-enabled-your-station = Si collega ogni volta che è attivo. La tua stazione è mostrata agli altri solo mentre la radio è in RADE — in qualunque altro modo resti collegato ma nascosto.
settings-net-station = Stazione
settings-net-message = Messaggio
settings-net-receive-only-i-cannot-transmit = Solo ricezione (non posso trasmettere)
settings-net-server = Server
settings-net-host = Host
settings-net-port = Porta
settings-net-tls-wss = TLS (wss://)
settings-net-not-yet-implemented-freedv-gui = Non ancora implementato — anche l'interfaccia FreeDV usa ws:// semplice.
settings-net-reporting = Segnalazione
settings-net-report-stations-i-decode = Segnala le stazioni che decodifico
settings-net-sends-rx-report-each-callsign = Invia un rx_report per ogni nominativo recuperato da un frame End-of-Over di RADE.
settings-net-show-other-reporter-stations-spots = Mostra le altre stazioni reporter come spot
settings-net-adds-them-panadapter-overlay-world = Le aggiunge al livello del panadapter, alla mappa del mondo e alla finestra SPOTS, sotto il filtro FREEDV.
settings-net-set-your-callsign-grid-general-2 = ⚠ Imposta il tuo nominativo e il locatore nella scheda Generale. Senza entrambi, il collegamento è solo in visualizzazione: vedi le altre stazioni ma tu non compari.
settings-net-apply = {" "}APPLICA{" "}
settings-net-persist-re-connect = Salva e (ri)collega
settings-net-longwave-shortwave-stations-labelled-waterfall = Le stazioni in onde lunghe e corte etichettate sulla waterfall. Il programma delle onde corte viene scaricato da eibispace.de per la stagione di trasmissione in corso e riscaricato quando la stagione cambia.
settings-net-season = Stagione
settings-net-downloading = download in corso…
settings-net-using-built-copy = uso la copia integrata
settings-net-your-own-stations-corrections-go = Le tue stazioni e le correzioni vanno in questo file. sdroxide non lo scrive mai, e una voce qui sostituisce una voce programmata con lo stesso nome e la stessa frequenza.
settings-net-reload = Ricarica
settings-net-re-read-your-station-file = Rilegge il file delle tue stazioni dopo averlo modificato
settings-net-download-schedule-now = Scarica il programma ora
settings-net-fetch-season-s-schedule-again = Riscarica il programma di questa stagione, sostituendo la copia in cache
settings-net-broadcast-stations-labelled-waterfall-come = Le stazioni di radiodiffusione etichettate sulla waterfall provengono dall'elenco integrato in questa build. Per modificarle serve l'app desktop.
settings-net-ask-service-whether-these-credentials = Chiede al servizio se queste credenziali funzionano. Applica prima le impostazioni qui sopra, e non registra nulla: si limita a leggere.
settings-net-checking = verifica in corso…

## Settings window: messages with values filled in.

settings-amp-connected-to = Collegato al { $model }
settings-controls-last-midi = Ultimo messaggio: { $message }  valore { $value }
settings-general-band-plan-file = Ogni limite di banda e sotto-segmento proviene da { $path }, scritto dalle tabelle IARU integrate la prima volta e da allora modificabile da te — restringi una banda alla tua licenza e sdroxide rifiuterà di trasmettere al di fuori di essa. Le frequenze sono in MHz; elimina il file per ottenere una copia nuova dei predefiniti. Questa è l'assegnazione regionale, non la tua licenza: le tue condizioni personali possono essere più ristrette, e i piani nazionali differiscono all'interno di una regione.
settings-general-exported = Esportato { $note } — scegli dove salvarlo.
settings-general-export-failed = Esportazione non riuscita: { $error }
settings-general-import-failed = Importazione non riuscita: { $error }
settings-general-imported = { $summary } — riavvia sdroxide per usarli.
settings-general-import-skipped = Saltato { $path }: { $why }
settings-general-swr-tripped = ⚠ Attualmente scattata a { $swr }:1 — la trasmissione è bloccata.
settings-net-call-grid-set-on-general = { $call } / { $grid } — impostati nella scheda Generale
settings-net-reporting-as = Segnalazione come { $call } / { $grid } — SDRoxide { $version }
settings-net-schedule-download-failed = ⚠ Download non riuscito: { $error }. Uso il programma integrato.
settings-net-test-service = Prova { $service }

## Settings window (src/app/settings/mod.rs), continued

settings-transmit-off-while-converter-set = La trasmissione è disattivata mentre un convertitore è impostato — indica ciò che c'è nella linea di trasmissione qui sopra per riattivarla.
settings-transmit-goes-through-same-converter = La trasmissione passa dallo stesso convertitore, quindi la radio lavora con lo stesso offset rispetto alla sintonia in entrambe le direzioni.
settings-transmit-not-converted-radio-transmits = La trasmissione non è convertita: la radio trasmette sulla frequenza indicata dalla sintonia, mentre la ricezione passa dal convertitore.
settings-transmit-takes-own-offset-separate = La trasmissione usa un proprio offset, separato da quello di ricezione.
settings-no-enabled-subscriptions-update = Nessun abbonamento attivo da aggiornare.
settings-add-radio-here-station-you = Aggiungi una radio — qui, oppure su una stazione a cui sei collegato

## Settings window, General, continued

settings-general-every-band-edge-sub-segment = Ogni limite di banda e sotto-segmento proviene da bandplan.json sulla macchina a cui è collegata la radio. Questa è l'assegnazione regionale, non la tua licenza: le tue condizioni personali possono essere più ristrette, e i piani nazionali differiscono all'interno di una regione.

## Settings window, picture upload (src/app/settings/picup.rs)

settings-picup-received-pictures-ftp = Immagini ricevute → FTP
settings-picup-uploads-every-received-sstv-picture = Invia ogni immagine SSTV ricevuta (e, se selezionate, ogni carta meteo) appena viene salvata, disposta come la fa QSSTV, così le pagine web scritte per QSSTV funzionano senza modifiche.
settings-picup-upload-received-pictures = Invia le immagini ricevute
settings-picup-server = Server
settings-picup-port = Porta
settings-picup-security = Sicurezza
settings-picup-user = Utente
settings-picup-password = Password
settings-picup-certificate = Certificato
settings-picup-trusted-when-computer-s-operating = Considerato attendibile quando il sistema operativo di questo computer lo considera tale, come farebbe un browser qui. Si può aggiungere sotto una CA privata (PEM).
settings-picup-extra-ca = CA aggiuntiva
settings-picup-only-certificate-fingerprint-accepted-way = Viene accettato solo il certificato con questa impronta — il modo per fidarsi di un server autofirmato. TEST mostra quella del server.
settings-picup-any-certificate-accepted-link-encrypted = ⚠ Viene accettato qualsiasi certificato. Il collegamento è cifrato, ma chiunque sul percorso può farsi passare per il server e leggere la password.
settings-picup-plain-ftp-sends-password-pictures = ⚠ L'FTP semplice invia la password e le immagini senza cifratura.
settings-picup-sstv = SSTV
settings-picup-sstv-dir = Cartella SSTV
settings-picup-weather-fax = Fax meteo
settings-picup-wefax-dir = Cartella WEFAX
settings-picup-empty-login-directory = Vuoto significa la cartella di accesso.
settings-picup-file-names = Nomi dei file
settings-picup-format = Formato
settings-picup-add-extension = aggiungi estensione
settings-picup-image1-png-rather-than-qsstv = image1.png anziché il semplice image1 di QSSTV
settings-picup-caption-bar = Barra didascalia
settings-picup-time-frequency-mode-sender-s = Ora, frequenza, modo e il FSK ID del mittente in cima, come lo disegna QSSTV. Viene contrassegnata solo la copia inviata.
settings-picup-fit-960-768 = Adatta a 960 × 768
settings-picup-qsstv-s-heartbeat-tiny-picture = Il battito di QSSTV: un'immagine minuscola la cui altezza è il minuto, inviata accanto a ciascuna, e resa larga due pixel alla chiusura del programma.
settings-picup-nat-workaround = Aggiramento NAT
settings-picup-ignore-private-address-server-s = Ignora un indirizzo privato nella risposta in modalità passiva del server e usa quello del server stesso — per un server dietro NAT che non conosce il proprio indirizzo pubblico.
settings-picup-apply = {" "}APPLICA{" "}
settings-picup-test = PROVA
settings-picup-connect-settings-above-applied-not = Collegati con le impostazioni qui sopra — applicate o no — accedi, e scrivi ed elimina un piccolo file in ciascuna cartella di invio.
settings-picup-checking = verifica in corso…
settings-picup-trust-certificate = Considera attendibile questo certificato
settings-picup-pin-switch-pinned-fingerprint-one = Fissalo: passa a "Impronta fissata" con questa. Verificala prima sul server — questo è ciò che il server ha detto, non una prova di chi sia. Poi APPLICA.
settings-picup-last-upload = Ultimo invio

## Settings window, Profiles (src/app/settings/profiles.rs)

settings-profiles-profiles-station-s-saved-setups = Profili: le configurazioni salvate della stazione
settings-profiles-profile-captures-whole-working-setup = Un profilo cattura l'intera configurazione di lavoro — sintonie e VFO, modo e filtri, guadagni, pilotaggio e antenne, il tuo nominativo e i modelli di messaggio, e i registri per banda — e la riapplica con un clic. L'hardware (back end, dispositivi audio, convertitori) viene deliberatamente lasciato stare: un profilo è il modo in cui lavori con la stazione, non la radio su cui gira.
settings-profiles-save-current-setup = Salva la configurazione attuale
settings-profiles-profile-name = nome del profilo
settings-profiles-save = Salva
settings-profiles-applies-whatever-sdroxide-attached-dials = Si applica a ciò a cui sdroxide è collegato: sintonie e VFO, modo e filtri, livelli, guadagni, pilotaggio, antenne, l'identità digitale e le pile di banda.
settings-profiles-no-profiles-saved-yet-name = Nessun profilo salvato — dagli un nome sopra e premi Salva.
settings-profiles-saved-profiles = Profili salvati
settings-profiles-apply = Applica

## Settings window, T/R switch (src/app/settings/relay.rs)

settings-relay-transmit-receive-switch = Commutatore trasmissione/ricezione
settings-relay-waiting-station-s-t-r = In attesa della configurazione del commutatore T/R della stazione…
settings-relay-closes-contact-while-station-transmits = Chiude un contatto mentre questa stazione trasmette — per scollegare e mettere a massa l'ingresso antenna dell'SDR, e per comandare in sequenza un amplificatore o un relè T/R esterno. Funziona con le schede relè USB economiche (LCUS, KMtronic, Numato), con una linea seriale RTS/DTR verso qualunque interfaccia che voglia una chiusura PTT, e con qualsiasi altra cosa tramite un hook di comando.
settings-relay-hardware = Hardware
settings-relay-board = Scheda
settings-relay-lcus-four-byte-a0-command = LCUS è la scheda a comandi A0 da quattro byte venduta con una dozzina di nomi; anche KMtronic e Numato possono essere interrogati su come sono impostati
settings-relay-port = Porta
settings-relay-choose = — scegli —
settings-relay-device = Dispositivo
settings-relay-gpio-pin = Pin GPIO
settings-relay-pin-3-every-homebrew-plan = Il pin 3 su ogni piano fai-da-te e su ogni scheda commerciale — è quello in fondo al pacchetto a cui si può saldare un filo. Se il relè non scatta, la scheda è un clone che non implementa affatto questi pin.
settings-relay-gpio-chip = Chip GPIO
settings-relay-linux-only-gpiodetect-lists-chips = Solo Linux. `gpiodetect` elenca i chip della macchina.
settings-relay-transmit = In trasmissione
settings-relay-receive = In ricezione
settings-relay-nothing-switched = Non si commuta nulla.
settings-relay-starting-program-costs-unpredictable-amount = Avviare un programma costa una quantità di tempo imprevedibile, e nulla di esso viene conteggiato nell'anticipo — quindi lascia un margine più ampio qui di quanto faresti per una scheda relè. I numeri della tabella dei contatti non significano nulla per questo collegamento: i tuoi comandi vengono eseguiti una volta alla pressione e una volta al rilascio.
settings-relay-contacts = Contatti
settings-relay-contact-1-rts-contact-2 = Il contatto 1 è RTS e il contatto 2 è DTR.
settings-relay-no = N.
settings-relay-gpio = GPIO
settings-relay-name = Nome
settings-relay-job = Funzione
settings-relay-tx-closes = Chiude in TX
settings-relay-lead = Anticipo
settings-relay-hold = Tenuta
settings-relay-line-s-offset-chip-bcm = Lo scostamento della linea su questo chip — il numero BCM su un connettore di Raspberry Pi, che non è il numero del pin fisico
settings-relay-sdr-antenna = Antenna SDR
settings-relay-decides-default-timings-what-log = Decide i tempi predefiniti e il nome con cui lo registra il log — e, per un decodificatore di banda, che il contatto segue la tabella delle bande qui sotto
settings-relay-transmitting-energises-coil-off-transmitting =
    Attivo: trasmettere eccita la bobina. Disattivo: trasmettere la rilascia.

    Questa è una scelta di cablaggio prima che un'impostazione — vedi la nota qui sotto.
settings-relay-how-long-before-rf-contact = Quanto prima dell'RF questo contatto passa dalla parola RX della sua banda alla parola TX. Un cambio di sintonia lo sposta subito; questo serve per il key-down, quando un filtro solo-trasmissione deve essere inserito prima che arrivi la potenza.
settings-relay-closed-long-before-rf-small = Chiuso con questo anticipo rispetto all'RF. Un piccolo relè coassiale scatta in 5–15 ms. La trasmissione attende il più lungo di questi, quindi un valore alto è un vuoto udibile nell'audio in ricezione; oltre 250 ms viene ignorato.
settings-relay-how-long-after-rf-stops = Quanto dopo che l'RF si ferma questo contatto torna dalla sua parola TX alla sua parola RX.
settings-relay-opened-long-after-rf-stops = Si apre questo tempo dopo che l'RF si ferma. Più lungo dell'anticipo di proposito: lasciare tornare l'antenna con un attimo di ritardo non costa nulla, mentre farlo in anticipo costa uno stadio d'ingresso. Zero è una risposta valida, ed è quella giusta per la linea di comando di un amplificatore.
settings-relay-test = {" "}PROVA{" "}
settings-relay-not-while-station-air = Non mentre la stazione è in trasmissione
settings-relay-close-contact-half-second-so = Chiudi questo contatto per mezzo secondo, così puoi sentire il relè e controllare il cablaggio con il trasmettitore spento
settings-relay-remove = Rimuovi
settings-relay-add-contact = + aggiungi un contatto
settings-relay-if-will-not-answer = Se non risponde
settings-relay-switch-exists-protect-receiver-not = Un commutatore che esiste per proteggere un ricevitore non vale nulla se smette in silenzio di proteggerlo — quindi il rifiuto è l'impostazione predefinita. Scegli l'altra opzione dove un cavo USB allentato che interrompe la gara è l'esito peggiore.
settings-relay-order-timings-longest-lead-closes = L'ordine è dato dai tempi: l'anticipo più lungo chiude per primo, e la tenuta più lunga apre per ultima. Quindi dai al relè dell'antenna l'anticipo e la tenuta più lunghi, e all'amplificatore quelli più corti — così l'antenna viene sempre commutata prima che l'amplificatore sia comandato, e l'amplificatore si disattiva sempre prima che l'antenna torni.
settings-relay-transmit-sense-input = Ingresso di rilevamento trasmissione
settings-relay-when-transceiver-keys-itself-own = Quando un ricetrasmettitore si attiva da solo — il proprio tasto del microfono, un pedale, VOX o il keyer — sdroxide lo scopre chiedendolo via CAT, e quella risposta arriva qualche centinaio di millisecondi dopo l'inizio della trasmissione. Collega la linea SEND dell'apparato, tramite un optoisolatore, a un ingresso di handshake su questa stessa porta, e viene rilevato in pochi millisecondi. Con questo il misuratore e l'interblocco di trasmissione diventano più rapidi.
settings-relay-line = Linea
settings-relay-transmitting = In trasmissione è
settings-relay-high-line = una linea alta
settings-relay-most-opto-isolated-interfaces-pull = La maggior parte delle interfacce optoisolate porta la linea a massa quando l'apparato trasmette, quindi di solito questa è disattivata
settings-relay-belongs-radio = Appartiene alla radio
settings-relay-which-radio-tab-sensed-transceiver = A quale scheda radio appartiene il ricetrasmettitore rilevato. In una stazione con una sola radio è 1 e nulla dipende da questo.
settings-relay-needs-serial-port-handshake-lines = Richiede una porta seriale — su di essa ci sono le linee di handshake.
settings-relay-wiring-only-fail-safe-survives = Il cablaggio è l'unica protezione di sicurezza che sopravvive a questo programma.
settings-relay-choose-tx-closes-so-de = Scegli “Chiude in TX” così che il contatto *diseccitato* sia lo stato che vuoi quando la scheda viene staccata, il computer è spento o sdroxide non è in funzione. Cablato in modo che diseccitato significhi a massa, un relè guasto lascia l'SDR sordo — fastidioso, ma sicuro. Cablato al contrario, un relè guasto più una trasmissione significa uno stadio d'ingresso morto. Con un vero amplificatore a valle, scegli la prima.
settings-relay-cannot-protect-first-instant-over = Questo non può proteggere il primo istante di una trasmissione che avvii dalla radio. Se comandi l'apparato da sdroxide, i contatti precedono sempre l'RF; con l'ingresso di rilevamento collegato come sopra, la seguono in pochi millisecondi. Per uno stadio d'ingresso davvero da proteggere, usa anche un commutatore T/R hardware sensibile all'RF — nessun programma su un computer può essere la soluzione completa.
settings-relay-contacts-transmit = contatti in TRASMISSIONE
settings-relay-not-open = Non aperto.
settings-relay-status-unknown-press-apply = Stato sconosciuto — premi APPLICA.
settings-relay-station-air-apply-will-take = La stazione è in trasmissione — APPLICA avrà effetto dopo questa trasmissione.
settings-relay-band-decoder-outputs = Uscite del decodificatore di banda
settings-relay-one-control-word-per-band = Una parola di controllo per banda: il bit 0 è il contatto 1, il bit 1 il contatto 2 e così via, come nella numerazione qui sopra. RX segue la banda della sintonia di ricezione; TX la banda di qualunque radio abbia avviato la trasmissione, inserita sull'anticipo di ciascun contatto prima dell'RF e rilasciata dopo la sua tenuta — dai lo stesso valore a entrambi per un filtro a cui non interessa in quale direzione vada l'RF, e valori diversi per un bypass di preamplificatore solo in ricezione o un banco di filtri passa-basso solo in trasmissione. Le bande lasciate a 0 non attivano nulla. Ha effetto con Applica / riconnetti.
settings-relay-band = Banda
settings-relay-rx = RX
settings-relay-tx = TX
settings-relay-outputs-asserted = Uscite attive
settings-relay-other = Altro
settings-relay-everywhere-outside-amateur-bands-short = Ovunque fuori dalle bande radioamatoriali — ascolto in onde corte, e tutto ciò su cui cade la sintonia di un transverter che nessuna banda copre.
settings-relay-apply = {" "}APPLICA{" "}
settings-relay-persist-re-open-switch = Salva e (ri)apri il commutatore

## Settings window, Remote (src/app/settings/remote.rs)

settings-remote-connect-server = Collegati a un server
settings-remote-drive-sdroxide-running-somewhere-else = Comanda via rete un sdroxide in funzione altrove — la macchina dello shack, un sito remoto. Avvialo lì con --server, e inserisci qui il suo indirizzo. La radio, i decodificatori e il logbook restano su quella macchina; questo schermo riceve la waterfall e l'audio, e usa il proprio altoparlante e microfono.
settings-remote-address = Indirizzo
settings-remote-host-name-ip = nome host o IP
settings-remote-machine-running-sdroxide-server-host = La macchina che esegue sdroxide --server: un nome host, un indirizzo IPv4 o IPv6, oppure un URL ws:// (o wss://) completo se il server si trova dietro un reverse proxy — un URL viene usato esattamente come scritto e ignora la porta e l'interruttore qui sotto.
settings-remote-port = Porta
settings-remote-port-server-listens-server-port = La porta su cui ascolta quel server — server_port nel suo config.toml. 4950 se non gli è stata data --port.
settings-remote-secure = Sicuro
settings-remote-wss-tls = wss:// (TLS)
settings-remote-tick-where-server-sits-behind = Seleziona questa opzione quando il server sta dietro qualcosa che termina HTTPS per lui — un reverse proxy sulla porta 443, che è il modo abituale per raggiungere una stazione attraverso internet aperta. Il server di sdroxide parla ws:// semplice e non la vuole.
settings-remote-connect = {" "}COLLEGA{" "}
settings-remote-open-station-radio-tab-own = Apri questa stazione come scheda radio a sé
settings-remote-client-cannot-hold-second-connection = Questo client non può mantenere una seconda connessione.
settings-remote-station-appears-tab-beside-radios =
    La stazione compare come scheda accanto alle radio già aperte qui, e la barra delle schede in cima alla finestra passa da una all'altra (⊞ le mette due affiancate). Chiudila dall'elenco in cima alla scheda Radio — questo riaggancia; nulla sul server viene modificato.

    Se il server chiede un nome utente e una password, la sua schermata di accesso compare nella nuova scheda. Un collegamento ws:// semplice non trasporta nulla di tutto ciò cifrato, quindi attraverso internet aperta usa una VPN o un tunnel SSH, oppure termina HTTPS davanti al server e seleziona Sicuro qui sopra.

## Settings window, Servers (src/app/settings/servers.rs)

settings-servers-wsjt-x-udp-broadcast = Trasmissione UDP WSJT-X
settings-servers-waiting-station-s-wsjt-x = In attesa della configurazione di trasmissione WSJT-X della stazione…
settings-servers-sends-decodes-station-status-logged = Invia decodifiche, stato della stazione e QSO registrati nel modo di WSJT-X, così GridTracker, JTAlert, N1MM+ e Log4OM funzionano con sdroxide senza modifiche. Solo in uscita — nulla su questo socket può toccare la radio.
settings-servers-enable = Attiva
settings-servers-send = Invia a
settings-servers-t-127-0-0-1-reaches = 127.0.0.1 raggiunge i client su questa macchina; un indirizzo LAN o un gruppo multicast (224.0.0.1) raggiunge gli altri
settings-servers-port = Porta
settings-servers-t-2237-port-every-client-defaults = 2237 è la porta predefinita di ogni client
settings-servers-identify = Identificati come
settings-servers-name-clients-see-some-loggers = Il nome che vedono i client. Alcuni logger accettano solo traffico che si dichiara WSJT-X.
settings-servers-n1mm-contactinfo-broadcast = Trasmissione contactinfo N1MM+
settings-servers-same-news-n1mm-s-words = Le stesse notizie con le parole di N1MM: un datagramma XML per ogni contatto registrato, per i logger che ascoltano N1MM anziché WSJT-X — il ponte della World Radio League accetta entrambi. Solo contatti; in questo protocollo non c'è un flusso di decodifiche. Ha un proprio interruttore e una propria porta, perché una stazione può volerli entrambi.
settings-servers-t-127-0-0-1-reaches-2 = 127.0.0.1 raggiunge i client su questa macchina. Il consiglio di N1MM stesso per una rete di gara è l'indirizzo di broadcast di questa sottorete — 192.168.1.255 per una rete 192.168.1.n — che raggiunge tutte le postazioni insieme.
settings-servers-t-12060-port-n1mm-s-documentation = 12060 è la porta che raccomanda la documentazione di N1MM
settings-servers-station-name = Nome della stazione
settings-servers-what-n1mm-calls-stationname-name = Ciò che N1MM chiama StationName: il nome del computer che ha inviato il pacchetto. I logger lo mostrano per distinguere una postazione operativa dall'altra.
settings-servers-decoded-cw-over-udp = CW decodificato via UDP
settings-servers-cw-panel-s-copy-plain = La copia del pannello CW come testo semplice, un datagramma UDP per ogni pezzo non appena si stabilizza — per un controller di stazione, la finestra CW di un logger o uno script. Nessun protocollo: un ascoltatore accoda semplicemente ciò che arriva (prova nc -ul 9999). Viene inviato solo testo stabilizzato, mai la coda che il decodificatore sta ancora rivedendo. Ogni radio ha il proprio interruttore.
settings-servers-t-127-0-0-1-reaches-3 = 127.0.0.1 raggiunge i programmi su questa macchina; un indirizzo LAN o l'indirizzo di broadcast di questa sottorete raggiunge gli altri.
settings-servers-t-9999-default = 9999 per impostazione predefinita
settings-servers-apply = {" "}APPLICA{" "}
settings-servers-persist-re-open-broadcast-sockets = Salva e (ri)apri i socket di trasmissione
settings-servers-hamlib-rigctld-server = Server rigctld di Hamlib
settings-servers-waiting-station-s-rigctld-configuration = In attesa della configurazione rigctld della stazione…
settings-servers-listen = Ascolta su
settings-servers-t-127-0-0-1-machine = 127.0.0.1 (questa macchina)
settings-servers-t-0-0-0-0-whole = 0.0.0.0 (intera rete)
settings-servers-t-4532-port-every-rigctld-client = 4532 è la porta che presuppone ogni client rigctld
settings-servers-rig-name = Nome dell'apparato
settings-servers-reported-clients = riportato ai client
settings-servers-max-clients = Client massimi
settings-servers-allow-clients-transmit = Consenti ai client di trasmettere
settings-servers-off-refuses-every-key-request = Disattivato rifiuta ogni richiesta di trasmissione e smette di pubblicizzare un intervallo di trasmissione, così Hamlib stessa rifiuta di attivare il tasto.
settings-servers-rigctld-has-no-authentication-0 = ⚠ rigctld non ha autenticazione. Su 0.0.0.0, chiunque possa raggiungere questa porta può sintonizzare e attivare il tuo trasmettitore.
settings-servers-t-1-client = 1 client
settings-servers-not-running = Non in funzione.
settings-servers-status-unknown-press-apply = Stato sconosciuto — premi APPLICA.
settings-servers-persist-re-bind-server = Salva e (ri)associa il server
settings-servers-lets-any-hamlib-capable-program = Consente a qualunque programma compatibile con Hamlib di comandare questa radio: frequenza, modo, PTT, split, RIT e potenza. In WSJT-X, fldigi o CQRLOG scegli l'apparato “Hamlib NET rigctl” (modello 2) e puntalo a questo indirizzo; in GPredict e N1MM inserisci direttamente host e porta. A differenza del server TCI, porta solo controllo — niente audio, niente IQ.
settings-servers-antenna-rotator = Rotore d'antenna
settings-servers-waiting-station-s-rotator-configuration = In attesa della configurazione del rotore della stazione…
settings-servers-points-motorized-antenna-satellite-you = Punta un'antenna motorizzata verso il satellite su cui sei agganciato, tramite un demone rotctld di Hamlib (avvialo ad es. con “rotctld -m 603 -r /dev/ttyUSB0” per un rotore GS-232B, oppure “rotctld -m 1” per provarlo senza hardware) — un solo demone copre ogni rotore pilotato da Hamlib — oppure tramite PSTRotator, con UDP Control attivo nel suo Setup.
settings-servers-program = Programma
settings-servers-host = Host
settings-servers-t-4533-rotctld-s-default-pstrotator = 4533 è il predefinito di rotctld. La porta di controllo UDP di PSTRotator è 12000 di default, e risponde alle richieste di posizione sulla porta successiva, 12001 — che su questa macchina deve essere libera.
settings-servers-min-elevation = Elevazione minima
settings-servers-below-rotator-parks-instead-tracking = Sotto questo valore il rotore si mette in posizione di riposo anziché seguire — impostalo sul tuo orizzonte locale o sul profilo dei tetti
settings-servers-azimuth-offset = Offset di azimut
settings-servers-added-every-command-rotator-whose = Aggiunto a ogni comando, per un rotore il cui nord è sfasato
settings-servers-min-movement = Movimento minimo
settings-servers-steps-smaller-than-are-not = I passi più piccoli di questo non vengono inviati — inseguire decimi di grado consuma i motori senza motivo
settings-servers-park-position = Posizione di riposo
settings-servers-drive-position-when-idle = porta in una posizione quando è inattivo
settings-servers-off-leaves-antenna-wherever-last = Disattivato lascia l'antenna dove è finita l'ultima passata
settings-servers-not-connected = Non collegato.
settings-servers-persist-re-connect-daemon = Salva e (ri)collega al demone
settings-servers-waiting-station-s-tci-server = In attesa della configurazione del server TCI della stazione…
settings-servers-built-tci-server = Server TCI integrato
settings-servers-device-name = Nome del dispositivo
settings-servers-off-leaves-control-receive-streams = Disattivato lascia funzionanti controllo e flussi di ricezione, ma ogni richiesta di trasmissione viene rifiutata.
settings-servers-tci-has-no-authentication-0 = ⚠ TCI non ha autenticazione. Su 0.0.0.0, chiunque possa raggiungere questa porta può sintonizzare e attivare il tuo trasmettitore.
settings-servers-tci-client = Client TCI
settings-servers-when-you-tune-send-radio = Quando sintonizzi, invia il VFO di questa radio a un altro server TCI, così quell'apparato segue. Solo controllo — niente audio, niente IQ, e non attiva il trasmettitore.
settings-servers-server = Server
settings-servers-same-address-server-above-client = È lo stesso indirizzo del server qui sopra. Il client si collega solo quando quel server non è quello in ascolto — lascialo disattivato, oppure su un'altra porta, quando 590Ctrl usa questa porta.
settings-servers-tci-client-connected = ● Client TCI collegato
settings-servers-tci-client-connecting = Client TCI in collegamento…
settings-servers-tci-client-press-apply = Client TCI — premi APPLICA.
settings-servers-persist-re-bind-server-connect = Salva e (ri)associa il server, e collega il client TCI
settings-servers-server-lets-tci-capable-programs = Il server consente ai programmi compatibili con TCI di comandare questa radio: controllo di frequenza e modo, IQ a banda larga, audio di ricezione e audio di trasmissione. In WSJT-X scegli il tipo di apparato SunSDR (TCI), puntalo a questo indirizzo e imposta il PTT su TCI. Il client TCI va nell'altra direzione: la sintonia di sdroxide viene inviata come comando VFO a 590Ctrl (o a qualunque server TCI) all'indirizzo qui sopra. Non trasporta audio.

## Settings window, TLE (src/app/settings/tle.rs)

settings-tle-satellites-element-sets-frequencies = Satelliti: elementi orbitali e frequenze
settings-tle-waiting-station-s-satellite-configuration = In attesa della configurazione satellitare della stazione…
settings-tle-tracker-already-fetches-celestrak-s = Il tracker scarica già da solo il gruppo radioamatoriale di CelesTrak. Questo serve per tutto il resto: i satelliti meteo NOAA, un cubesat troppo recente per essere nel gruppo, o un insieme di elementi più aggiornato di quello arrivato. Gli elenchi vengono scaricati — e conservati — dove gira il motore radio, quindi è lo stesso insieme di satelliti ovunque sia aperta l'app.
settings-tle-subscriptions = Abbonamenti
settings-tle-listings-fetched-kept-current-same = Elenchi scaricati e mantenuti aggiornati, con la stessa cadenza di sei ore del gruppo radioamatoriale. Vengono aggiornati mentre la finestra solare è aperta, e con AGGIORNA ORA qui.
settings-tle-fetch-track-listing = Scarica e traccia questo elenco
settings-tle-remove-subscription = Rimuovi questo abbonamento
settings-tle-orbits = Orbite
settings-tle-which-satellites-listing-get-orbit = Quali satelliti di questo elenco ottengono un anello orbitale e un'etichetta. Un gruppo intero vuole “selezionati”: novanta anelli insieme sono illeggibili, e nessuno lascia novanta puntini anonimi.
settings-tle-nothing-listing-sdroxide-s-curated = Nulla in questo elenco è nell'elenco selezionato di sdroxide — quell'elenco contiene dieci satelliti radioamatoriali, quindi questo si comporterebbe esattamente come “nessuno”.
settings-tle-all-satellites = tutti i satelliti
settings-tle-catalogue-numbers-keep-comma-separated = Numeri di catalogo da conservare, separati da virgole. Vuoto traccia tutto ciò che contiene l'elenco.
settings-tle-not-fetched-yet = non ancora scaricato
settings-tle-subscription = + Abbonamento
settings-tle-update-now = {" "}AGGIORNA ORA{" "}
settings-tle-ask-radio-engine-fetch-every = Chiedi al motore radio di scaricare ora ogni abbonamento attivo
settings-tle-celestrak-groups = Gruppi CelesTrak
settings-tle-pasted-element-sets = Elementi orbitali incollati
settings-tle-one-off-these-do-not = Per un'una tantum. Questi non si aggiornano da soli, e SGP4 smette di propagare un insieme di elementi una volta che è passata una quindicina di giorni dalla sua epoca — per tutto ciò che intendi conservare, abbonati.
settings-tle-track-one = Traccia questo
settings-tle-show-two-element-lines = Mostra le due righe degli elementi
settings-tle-remove = Rimuovi
settings-tle-paste-two-three-line-element = Incolla qui insiemi di elementi a due o tre righe
settings-tle-add-pasted = + Aggiungi incollati
settings-tle-nothing-paste-box-looked-like = Nulla nella casella di incolla somigliava a un insieme di elementi.
settings-tle-clear-box = Svuota la casella
settings-tle-frequencies = Frequenze
settings-tle-shown-under-pass-table-solar = Mostrate sotto la tabella dei passaggi nella finestra solare. Una voce qui sostituisce del tutto quella integrata per quel numero di catalogo, quindi parti da una copia di essa a meno che tu non voglia eliminare le altre.
settings-tle-remove-satellite-s-entry = Rimuovi la voce di questo satellite
settings-tle-link = COLLEGAMENTO
settings-tle-downlink = DISCESA
settings-tle-uplink = SALITA
settings-tle-mode = MODO
settings-tle-note = NOTA
settings-tle-fm-repeater = Ripetitore FM
settings-tle-fm = FM
settings-tle-ctcss-67-0-hz = CTCSS 67,0 Hz
settings-tle-link-2 = + Collegamento
settings-tle-copy-built = Copia integrata
settings-tle-frequency-either-one-number-145 = Una frequenza è un solo numero (145.800) oppure una banda passante del transponder scritta 145.950-145.970. Lascia vuota una direzione per un radiofaro.
settings-tle-norad = NORAD
settings-tle-satellite = + Satellite
settings-tle-frequency-entry-needs-satellite-s = Una voce di frequenza richiede il numero di catalogo NORAD del satellite.

## Settings window, Wavelog (src/app/settings/wavelog.rs)

settings-wavelog-address = Indirizzo
settings-wavelog-api-key = Chiave API
settings-wavelog-your-wavelog-s-front-page = L'indirizzo della pagina principale del tuo Wavelog — https://log.example.org oppure http://192.168.1.10/wavelog — e una chiave dalla sua pagina API. L'invio richiede una chiave in lettura/scrittura.
settings-wavelog-plain-http-address-internet-api = http:// semplice verso un indirizzo su internet: la chiave API viaggia non cifrata. Va bene nella tua rete; attraverso internet usa https://.
settings-wavelog-trusted-cert = Certificato attendibile
settings-wavelog-forget-certificate = Dimentica questo certificato
settings-wavelog-go-back-ordinary-checking-against = Torna al normale controllo sulle autorità di certificazione pubbliche — per un server che ora ha un certificato regolare.
settings-wavelog-server-presented-certificate-sha-256 = Il server ha presentato questo certificato (SHA-256):
settings-wavelog-compare-your-server-s-e = Confrontalo con quello del tuo server — ad es. `openssl x509 -noout -fingerprint -sha256 -in cert.pem` — e consideralo attendibile solo se coincidono. Da quel momento viene accettato questo esatto certificato e ogni altro viene rifiutato.
settings-wavelog-trust-certificate = Considera attendibile questo certificato
settings-wavelog-station-location = Ubicazione della stazione
settings-wavelog-none = — nessuna —
settings-wavelog-press-test-above-list-station = Premi Prova qui sopra per elencare le ubicazioni della stazione definite nel tuo Wavelog.
settings-wavelog-default = Predefinita
settings-wavelog-where-qsos-go-when-operating = Dove vanno i QSO quando il profilo operativo non indica un'ubicazione.
settings-wavelog-profile = Questo profilo
settings-wavelog-use-default = Usa la predefinita
settings-wavelog-not-set = non impostata
settings-wavelog-no-station-location-chosen-uploads = Nessuna ubicazione della stazione scelta — gli invii falliranno finché non ne viene scelta una.
settings-wavelog-qsos-not-wavelog = QSO non presenti in Wavelog
settings-wavelog-every-qso-logbook-wavelog = Ogni QSO del logbook è in Wavelog.
settings-wavelog-everything-logbook-wavelog-has-not = Tutto ciò che il logbook contiene e Wavelog non ha confermato — inclusi i contatti precedenti all'invio automatico. Quelli che ha già tornano come duplicati e vengono semplicemente segnati come inviati.
settings-wavelog-set-address-key-station-location = Imposta prima l'indirizzo, la chiave e un'ubicazione della stazione.
settings-wavelog-yes-send-them = Sì, inviali
settings-wavelog-cancel = Annulla
settings-wavelog-download-into-logbook = Scarica nel logbook
settings-wavelog-downloading-runs-where-radio-attached = Lo scaricamento viene eseguito dove è collegata la radio, nel logbook di quella macchina — configuralo lì.
settings-wavelog-download-qsos-start-every = Scarica i QSO all'avvio e ogni
settings-wavelog-downloaded-qsos-join-logbook-so = I QSO scaricati entrano nel logbook, così FT8 e le liste degli spot sanno cosa hai già lavorato da altri programmi e apparati. Ogni scaricamento prende solo ciò che è nuovo.
settings-wavelog-press-test-above-choose-which = Premi Prova qui sopra per scegliere quali ubicazioni della stazione scaricare.
settings-wavelog-sync-now = Sincronizza ora
settings-wavelog-fetch-what-wavelog-has-added = Scarica ciò che Wavelog ha aggiunto dall'ultimo scaricamento. Applica prima le impostazioni qui sopra.
settings-wavelog-full-resync = Risincronizzazione completa…
settings-wavelog-download-ticked-locations-whole-compare = Scarica interamente le ubicazioni selezionate e confrontale con il log: i QSO modificati o cancellati in Wavelog, oppure provenienti da un'ubicazione non più selezionata, vengono riportati in linea. Vedi le modifiche prima che accada qualcosa.
settings-wavelog-full-resync-changes-make = Risincronizzazione completa — modifiche da fare
settings-wavelog-qsos-logged-here-imported-file = I QSO registrati qui o importati da un file non vengono mai modificati o rimossi — solo quelli scaricati da Wavelog.
settings-wavelog-apply = Applica
settings-wavelog-discard = Scarta
settings-wavelog-live-radio-status = Stato radio in tempo reale
settings-wavelog-send-frequency-mode-wavelog-they = Invia frequenza e modo a Wavelog man mano che cambiano
settings-wavelog-radio-name = Nome della radio
settings-wavelog-pick-radio-wavelog-s-live = Scegli questa radio nel logging in tempo reale di Wavelog e il suo modulo segue la sintonia. Richiede una chiave in lettura/scrittura. Solo dalla radio principale della stazione.

## Settings window, picture upload: messages with values

settings-picup-server-fingerprint = Impronta SHA-256 del certificato del server: { $fingerprint }

## Settings window, Profiles: messages with values

settings-profiles-apply-hint = Riporta la stazione su “{ $name }”: sintonie, VFO, modo, filtri, guadagni, pilotaggio, antenne, identità e pile di banda.
settings-profiles-delete = Elimina “{ $name }”

## Settings window, T/R switch: messages with values

settings-relay-not-usable = Non ancora utilizzabile: { $why }
settings-relay-receiving = in ricezione
settings-relay-rx-tx-words = { $rx } / TX { $tx }

## Settings window, Servers: messages with values

settings-servers-n-clients = { $n ->
        [one] { $n } client
       *[other] { $n } client
    }
settings-servers-listening = ● In ascolto su { $addr } — { $clients }
settings-servers-failed = Errore: { $error }
settings-servers-rotator-at = ● Collegato — antenna a { $az }° az, { $el }° el
settings-servers-tci-client-error = Client TCI: { $error }

## Settings window, TLE: messages with values

settings-tle-subscription-age = { $count } satelliti · { $age } di età
settings-tle-subscribed = Abbonato a { $name }. Premi AGGIORNA ORA per scaricarlo.
settings-tle-norad-too-stale = NORAD { $norad } · { $age } di età — troppo vecchio per propagare
settings-tle-norad-age = NORAD { $norad } · { $age } di età
settings-tle-norad-n = NORAD { $norad }
settings-tle-added = Aggiunt{ $n ->
        [one] o { $n } insieme di elementi
       *[other] i { $n } insiemi di elementi
    }.
settings-tle-added-refreshed = Aggiunt{ $added ->
        [one] o { $added }
       *[other] i { $added }
    } e aggiornat{ $r ->
        [one] o { $r } insieme di elementi
       *[other] i { $r } insiemi di elementi
    }.
settings-tle-links = { $n } { $n ->
        [one] collegamento
       *[other] collegamenti
    }
settings-tle-replace-links = Sostituisci questi collegamenti con quelli integrati per { $name }

## Settings window, Wavelog: messages with values

settings-wavelog-call-mismatch = L'ubicazione "{ $location }" è { $station_call }, ma tu registri come { $call } — Wavelog rifiuterà questi QSO.
settings-wavelog-grid-mismatch = L'ubicazione "{ $location }" si trova su { $station_grid }, ma il tuo locatore è { $grid } — Wavelog rifiuterà questi QSO.
settings-wavelog-location-n = ubicazione #{ $id }
settings-wavelog-saved-with-callsign = Salvata con il nominativo ({ $call }) in Impostazioni → Profili, così un profilo portatile può registrare su un'ubicazione portatile.
settings-wavelog-owed = { $n } { $n ->
        [one] QSO
       *[other] QSO
    } registrat{ $n ->
        [one] o
       *[other] i
    } dall'attivazione dell'invio automatico ancora da inviare — nuovo tentativo ogni 5 minuti.
settings-wavelog-send-unsent = Invia { $n } { $n ->
        [one] QSO
       *[other] QSO
    } non ancora presenti in Wavelog…
settings-wavelog-send-confirm = Inviare { $n } QSO a { $location }? Le impostazioni vengono applicate prima.
settings-wavelog-plan-add = • { $n } QSO presenti in Wavelog da aggiungere al log
settings-wavelog-plan-update = • { $n } QSO scaricati modificati in Wavelog da aggiornare
settings-wavelog-plan-remove = • { $n } QSO scaricati non più presenti in Wavelog (o provenienti da un'ubicazione non selezionata) da rimuovere
settings-wavelog-plan-mark = • { $n } dei tuoi QSO da segnare come presenti in Wavelog

## Settings window, Radio tab (src/app/settings/radio.rs)

settings-radio-antenna = Antenna
settings-radio-which-socket-back-radio-using =
    Quale presa sul retro usa la radio — il suo comando ANT, la stessa impostazione del pulsante ANT sul pannello frontale.

    Ha effetto subito, ed è l'impostazione della radio stessa anziché una copia conservata qui: la presa su cui si trova la radio viene riletta all'apertura della sessione.

    La scelta viene ricordata per banda. Passa all'antenna direttiva sui 2 m e al filo sui 40, e ciascuna tornerà la prossima volta che la sintonia entra in quella banda — e la prossima volta che sdroxide si avvia. Una memoria salvata qui porta con sé anche la sua presa.
settings-radio-receive-antenna = Antenna di ricezione
settings-radio-radio-s-separate-receiving-antenna =
    L'antenna di ricezione separata della radio — RX ANT sul retro — inserita nel percorso di ricezione o esclusa da esso. È un'impostazione propria, la stessa del pulsante RX ANT della radio.

    L'antenna sulla presa principale resta in trasmissione in ogni caso: questo è un ingresso in più per la ricezione, un loop o una beverage, non una scelta di presa.

    A differenza dell'antenna qui sopra, questo non viene ricordato qui. La radio lo conserva per banda da sola, quindi sdroxide lo rilegge dopo ogni cambio di banda e mostra ciò che dice la radio; spuntare questa casella è l'unica cosa che lo sposta.
settings-radio-receive-path = Nel percorso di ricezione
settings-radio-radio-power = Alimentazione della radio
settings-radio-switch-radio-itself-off-back =
    Spegni la radio stessa, e riaccendila, tramite il collegamento di controllo. È l'unico vero interruttore on/off del programma — non l'interruttore LINK di sdroxide, che chiude la parte di sdroxide e lascia la radio accesa.

    Perché l'accensione torni a raggiungere qualcosa, la parte di controllo della radio deve restare sveglia mentre è spenta. Sulla rete ci pensa Network Control. Su un cavo seriale è la porta CI-V della radio, che resta alimentata dalla rete su un apparato spento dal pannello frontale anziché scollegato; sdroxide invia prima dell'accensione la sequenza di risveglio che la documentazione Icom stessa richiede.

    Lo spegnimento interrompe audio e misuratori — dietro di essi non c'è più la radio — e sdroxide mantiene aperto il collegamento di controllo perché l'accensione abbia dove arrivare.
settings-radio-on = Acceso
settings-radio-off = Spento
settings-radio-waiting-configuration-machine-radio-attached = In attesa della configurazione della macchina a cui è collegata la radio.
settings-radio-sound-format = Formato audio
settings-radio-invert-spectrum = Inverti lo spettro
settings-radio-swap-i-q = Scambia I/Q
settings-radio-mirror-panadapter-about-tuned-frequency =
    Specchia il panadapter rispetto alla frequenza sintonizzata, per un apparato che porta I e Q invertiti sulla sua scheda audio. Il segnale rivelatore è una waterfall piena di segnali convincenti, tutti dalla parte sbagliata della sintonia, con la SSB che esce sulla banda laterale opposta — scambiare i due cavi sulla scheda audio risolverebbe allo stesso modo.

    Solo ricezione: la trasmissione consegna alla radio un unico segnale audio reale, che non ha banda laterale da invertire.
settings-radio-i-q-sample-rate = Frequenza di campionamento I/Q
settings-radio-how-fast-radio-s-i =
    Quanto velocemente viene fatta girare la scheda audio I/Q della radio — e quindi quanta banda mostra il panadapter, perché un flusso in quadratura copre tutta la sua frequenza di campionamento: 48 kHz danno ±24 kHz per lato rispetto alla sintonia, 192 kHz danno ±96.

    La scheda ha l'ultima parola. Una scheda che non sa fare la frequenza scelta qui viene aperta alla più vicina che sa fare, e il panadapter risulta corrispondentemente più stretto — il log lo dice all'avvio quando accade.

    Una scheda più veloce è anche più lavoro: la macchina ha proporzionalmente meno tempo per svuotarla, e una che non tiene il passo perde campioni. Si vede come audio che si spezza mentre la waterfall continua ad apparire perfetta, e anche questo viene registrato nel log. Se lo vedi, torna indietro di un passo.
settings-radio-t-48-khz-24-khz = 48 kHz (±24 kHz)
settings-radio-t-96-khz-48-khz = 96 kHz (±48 kHz)
settings-radio-t-192-khz-96-khz = 192 kHz (±96 kHz)
settings-radio-t-384-khz-192-khz = 384 kHz (±192 kHz)
settings-radio-i-q-centre-offset = Offset del centro I/Q
settings-radio-how-far-radio-s-i =
    Di quanto l'uscita I/Q della radio è centrata sopra la propria sintonia, per un apparato la cui I.F. di ricezione è stata spostata lontano da zero — su un Elecraft, MENU:RX SHFT impostato su 8.0 anziché NOR, che allontana la sintonia dal picco DC del mixer (e impedisce che una stazione SSB/AM forte vicina venga rivelata in AM).

    Non è un convertitore: la radio continua a visualizzare e trasmettere sulla frequenza reale, e qui non viene mai inviato nulla. Questo dice soltanto dove si trovano i campioni sulla scheda audio, e il flusso viene riportato sulla sintonia man mano che arriva.

    Lascia a 0 a meno che tu non abbia attivato una voce di menu del genere. Se i segnali cadono a due volte l'offset di distanza, il segno è invertito.
settings-radio-iq-correction = Correzione IQ
settings-radio-cancel-mirror-image-every-signal =
    Cancella l'immagine speculare di ogni segnale, e il picco DC al centro della waterfall.

    L'uscita I/Q di una radio sono due percorsi analogici che non sono mai del tutto uguali in guadagno né esattamente sfasati di 90°, e ciò che ne risulta è una copia di ogni segnale riflessa rispetto alla frequenza sintonizzata, di solito 30-40 dB più in basso — abbastanza forte da sembrare una stazione che non c'è, e da essere decodificata come tale. Le radio con trimmer di bilanciamento sul pannello frontale regolano esattamente questo; qui viene misurato sul rumore ricevuto e non richiede alcuna impostazione.

    Lascialo attivo. Disattivalo se ascolti AM sintonizzata esattamente sulla portante — la portante è DC, quindi va insieme al picco — o per verificare se un segnale è reale osservando se sopravvive.
settings-radio-cancel-mirror-images = Cancella le immagini speculari
settings-radio-dc-notch = Notch DC
settings-radio-widen-hole-taken-out-middle =
    Allarga il buco ricavato al centro dell'ampiezza, per una radio il cui picco centrale è più largo dell'offset sottostante.

    0 lascia il normale blocco, largo qualche decina di hertz, che elimina l'offset senza toccare altro. Alzandolo, se ne va anche il fondo dell'ampiezza: un passa-alto del primo ordine, attenuato di 3 dB al valore impostato qui e di più sotto di esso.

    È centrato dove è centrato l'I/Q della radio, cioè sulla sintonia a meno che l'offset qui sopra non l'abbia spostato — quindi anche tutto ciò che è sintonizzato lì viene eliminato. Una nota CW a 600 Hz rientra in un'impostazione di 600 Hz.
settings-radio-panadapter-bw = Larghezza di banda del panadapter
settings-radio-cat-family = Famiglia CAT
settings-radio-rigctld-address = Indirizzo rigctld
settings-radio-host-port-running-hamlib-rigctld =
    host:porta di un rigctld di Hamlib in funzione — 127.0.0.1:4532 è il suo predefinito, su questa macchina.

    Avvialo con, ad esempio, `rigctld -m 2028 -r /dev/ttyUSB0 -s 38400`, dove -m è il numero di modello Hamlib della tua radio (`rigctl -l` li elenca).

    Questo è il contenitore generico: raggiunge frequenza, modo, PTT, potenza, S-meter e ROS, e nient'altro. Dove una delle famiglie native qui sopra si adatta alla tua radio, fa di più — comanda in trasmissione dal buffer di testo dell'apparato, il filtro di ricezione e le scale dei misuratori per modello.
settings-radio-flrig-address = Indirizzo flrig
settings-radio-host-port-running-flrig-127 =
    host:porta di un flrig in funzione — 127.0.0.1:12345 è il suo predefinito, su questa macchina. flrig serve XML-RPC ogni volta che è in funzione; la porta si trova in Config → Setup → Server.

    Come l'opzione Hamlib, pilota un demone anziché la radio, ma tramite il driver specifico per modello di flrig — su diversi apparati la gestione di potenza e filtri è la più fedele delle due. Raggiunge frequenza, modo, PTT, potenza di trasmissione (in watt interi), la larghezza di banda in ricezione, lo S-meter, il ROS e la potenza in uscita. Il CW passa dalla porta cwio di flrig, che va configurata in flrig stesso.
settings-radio-serial-port = Porta seriale
settings-radio-select = — seleziona —
settings-radio-baud = Baud
settings-radio-data-bits = Bit di dati
settings-radio-parity = Parità
settings-radio-stop-bits = Bit di stop
settings-radio-force-rts = Forza RTS
settings-radio-force-dtr = Forza DTR
settings-radio-ptt-method = Metodo PTT
settings-radio-how-transmit-keyed = Come viene attivata la trasmissione.
settings-radio-how-transmit-keyed-network-link = Come viene attivata la trasmissione. Un collegamento di rete non ha linee di controllo, quindi DTR e RTS qui non attivano nulla — usa CAT, che chiede al demone di attivare la radio, oppure VOX.
settings-radio-mode-control = Controllo del modo
settings-radio-digimode-mode = Modo per i modi digitali
settings-radio-what-put-radio-mode-sdroxide =
    In quale modo mettere la radio per un modo che sdroxide modula tramite la scheda audio. "DIGI" seleziona la posizione DATA/PKT dell'apparato, che prende l'audio di trasmissione dall'ingresso USB o ACC senza l'elaborazione vocale del percorso microfono; "USB" lo lascia sulla banda laterale semplice, per una radio già impostata a modulare la SSB dal suo ingresso dati o che non ha affatto una posizione DATA.

    Copre ogni modo digitale — FT8, FT4, PSK, RTTY, SSTV e gli altri — e il CW inviato come "Scheda audio", e per essi prevale su Controllo del modo. L'SSTV è quello la cui banda laterale segue la banda, quindi qui significa la posizione DATA su quella banda laterale: USB-D sopra i 40 m, LSB-D a quella frequenza e sotto. SSTV-FM non ne fa parte — una portante FM non ha banda laterale da scegliere.
settings-radio-cw-keying = Manipolazione CW
settings-radio-how-cw-panel-s-keyer =
    Come trasmette il manipolatore del pannello CW. "Manipolatore dell'apparato" mette la radio in CW e le consegna il testo da inviare con il proprio manipolatore. Usa la velocità del manipolatore dell'apparato (impostata dal WPM del pannello), e su Yaesu invia tramite la memoria del manipolatore 1, sovrascrivendo ciò che vi era memorizzato.

    Richiede il break-in attivo sulla radio, che sdroxide attiva da sé con ogni messaggio su Icom, Yaesu e Kenwood — un apparato con il break-in disattivato accetta il messaggio e non trasmette nulla. Resta attivo dopo, e un break-in completo non viene mai riportato a semi.

    "Scheda audio" invia invece il sidetone manipolato come audio (MCW), un tono a sintonia + tono — e poiché un apparato in CW ignorerebbe del tutto la sua scheda audio, selezionando CW si segue l'impostazione Modo per i modi digitali (USB, DATA, o Controllato dalla radio) anziché commutare l'apparato in CW. Scegli "Controllato dalla radio" qui per gli apparati la cui posizione dati non può essere comandata via CAT (la U-D di un Xiegu) e lasciali su di essa a mano, come per FT8.
settings-radio-poll-rate = Frequenza di interrogazione
settings-radio-how-often-radio-asked-what =
    Ogni quanto viene chiesto alla radio cosa sta facendo — la sua sintonia, il suo modo e i suoi misuratori. È la metà del collegamento di controllo che va dalla radio a sdroxide: gira la manopola VFO dell'apparato o cambia il suo modo dal pannello frontale, e la lettura, la banda e il panadapter seguono entro un'interrogazione.

    È anche tutto il traffico di controllo che questa parte genera, e su un Icom moderno quel traffico non è gratuito: la porta CI-V e la scheda audio stanno dietro lo stesso hub USB dentro la radio, quindi i frame richiesti qui sono tempo di bus che l'audio non ottiene — si sente come interruzioni che sembrano un guasto del DSP e non lo sono. Il valore predefinito di 2 Hz è mezzo secondo dietro la manopola e abbastanza silenzioso da non intralciare l'audio. Alzalo dove la porta di controllo è un dispositivo a sé (un adattatore USB-seriale separato, un rigctld in rete); abbassalo se l'audio della radio si spezza.

    Un Icom con CI-V Transceive attivo nel suo menu non ha bisogno di nulla di tutto questo per la sintonia: segnala la manopola nell'istante in cui si muove, e sdroxide sospende l'interrogazione della sintonia appena lo vede farlo — e la riprende se la radio si sposta senza dirlo.

    La frequenza è quella della sintonia. Il modo viaggia con una interrogazione ogni quattro, perché è un'impostazione che cambia alcune volte in una serata anziché una che segue una manopola.
settings-radio-send-command = Comando di invio
settings-radio-which-transceiver-generation-keys-rig =
    Quale generazione di ricetrasmettitori attiva l'apparato, per il metodo PTT "CAT". Le due non concordano su cosa significhi il parametro TX e nulla sul filo le distingue, quindi scegli quella che corrisponde alla tua radio.

    "Stile TS-2000 (TX;)" — TS-480, TS-570, TS-870, TS-2000: il normale invio, sulla banda principale.

    "Stile TS-590 (TX1;)" — TS-590S/SG, TS-890, TS-990: DATA SEND, che attiva con l'ingresso audio ACC2/USB attivo. Su questi apparati il normale invio seleziona invece il microfono e silenzia l'audio che sdroxide trasmette.

    Impostata male, un TS-590 trasmette silenzio — ma un TS-2000 trasmette sulla sotto-banda, che è un'altra banda del tutto.
settings-radio-radio = Radio
settings-radio-k3-k3s-kx3-kx2-k4 = K3 · K3S · KX3 · KX2 · K4
settings-radio-one-profile-covers-family-k3 =
    Un solo profilo copre la famiglia: il set di comandi K3, a cui rispondono KX3, KX2 e K4. Qui non c'è nulla da scegliere — quanti watt copre il cursore Pilotaggio (12 su un KX2 o KX3 nudo, 110 con un KPA3 o un KXPA100) viene letto dalla richiesta dei moduli opzionali dell'apparato stesso all'apertura della porta.

    Nota le velocità di baud qui sopra: K3, K3S, KX3 e KX2 non vanno oltre 38400, e un apparato impostato sotto la velocità scelta qui non risponde affatto.
settings-radio-transmit-input = Ingresso di trasmissione
settings-radio-where-radio-takes-transmit-audio =
    Da dove la radio prende l'audio di trasmissione — il comando TI dell'apparato, che è il menu 32 "TX IN" sul pannello frontale. Viene impostato all'apertura della porta.

    "Audio USB" è ciò che fa funzionare questa interfaccia: il FDM-DUO trasmette ciò che sdroxide mette nella sua scheda audio USB. Una radio lasciata su "Microfono" invia invece l'ambiente, e nulla sullo schermo lo segnala.

    "Automatico" lascia scegliere all'apparato — il microfono per una pressione del PTT sul microfono, la porta USB per un'attivazione CAT o RTS.

    "Lascia come impostato sulla radio" non invia alcun TI, per un operatore che lo imposta dal pannello frontale.
settings-radio-which-two-sockets-back-receiver =
    Quale delle due prese sul retro ascolta il ricevitore — il comando AN dell'apparato, che è il menu 31 "ANTENNAS" sul pannello frontale e l'indicatore "ANT 1 2" sul suo display.

    "RTX" è un'antenna che fa entrambi i compiti, sulla presa di tipo M che porta anche la trasmissione. "Solo RX" sposta la ricezione sulla seconda presa e lascia la trasmissione su RTX — un'antenna di ricezione, un loop o una beverage, con la direttiva ancora sul trasmettitore.

    Ha effetto subito, e viene riletto dalla radio all'apertura della porta: è l'impostazione dell'apparato stesso anziché una copia conservata qui.
settings-radio-one-profile-covers-both-there =
    Un solo profilo copre entrambi. Qui non c'è nulla da scegliere — la radio si identifica da sola all'apertura della porta.

    Questa è solo la parte CAT: pilota sintonia, modo, PTT, S-meter, ROS e potenza di trasmissione sulla porta USB CAT dell'apparato, e prende l'audio dalla sua porta USB Audio. La terza porta USB del FDM-DUO porta l'I/Q a banda larga, che questa interfaccia non legge — per quello seleziona l'interfaccia ELAD qui sopra.

    Nota la velocità di baud: il menu 70 "CAT BAUD" sulla radio deve corrispondere, e di fabbrica è 38400.

    Il CW viene manipolato dal tasto o dal paddle propri della radio. Il FDM-DUO non ha alcun comando che accetti testo, quindi il pannello CW non può manipolarlo via CAT.
settings-radio-one-profile-covers-range-qrp =
    Un solo profilo copre la gamma: il set di comandi proprio di QRP Labs, che è un sottoinsieme di quello del Kenwood TS-480 con parecchie aggiunte. Qui non c'è nulla da scegliere — la radio si identifica con il proprio nome e la versione del firmware all'apertura della porta, e la versione decide se viene richiesta affatto la lettura della protezione ROS.

    Non il profilo Kenwood: su un QMX il comando PC è la lettura di potenza METER anziché il controllo di potenza, quindi una radio pilotata come Kenwood vedrebbe la propria lettura del misuratore scritta come se fosse un'impostazione, e MD8 — un modo Kenwood — qui è il sintonizzatore ROS.

    La potenza di trasmissione si imposta sulla radio. Non esiste un comando CAT per questo, quindi il cursore Pilotaggio raggiunge solo il livello dell'audio che entra nella scheda audio.

    Anche il filtro di ricezione è della radio: riporta la larghezza che il suo modo implica (3,2 kHz in Digi, 300 Hz in CW) e non offre modo di cambiarla.
settings-radio-baud-rate-ignored-port-usb = La velocità di baud viene ignorata — la porta è USB
settings-radio-qmx-serves-own-virtual-com =
    Un QMX offre le proprie porte COM virtuali via USB, quindi la velocità impostata qui non ha effetto su nessuna delle due estremità. Ne offre fino a tre; se la radio non risponde nulla, la causa più probabile è quella sbagliata.

    Una cosa conta davvero sulla porta: un ritorno carrello inviato ad essa porta la radio in modalità terminale per il resto della sessione. sdroxide non ne invia mai — ma un programma terminale lasciato aperto sulla stessa porta sì, e il CAT smette di funzionare nell'istante in cui accade.
settings-radio-i-q-mode-switched-radio = La modalità I/Q è attivata sulla radio
settings-radio-radio-s-sound-card-carries =
    La scheda audio della radio trasporta o l'audio demodulato o l'I/Q grezzo che vede il suo ADC, e sdroxide imposta quello che chiede il Formato audio qui sopra all'apertura della porta (il comando Q9 della radio, la voce di menu "IQ mode").

    L'offset del centro e la frequenza di campionamento qui sopra sono stati impostati su ciò che serve a un QMX: il suo ricevitore è un supereterodina con una I.F. a 12 kHz, quindi il sintetizzatore sta 12 kHz sotto la sintonia e tutto ciò che è sulla scheda sta 12 kHz sopra il centro dell'ampiezza. La scheda gira a 48 kHz, che rende il panadapter largo 48 kHz.

    In CW la radio sposta quella I.F. di altri ~700 Hz, cosa che un unico valore non può seguire — aggiungilo a mano se usi il panadapter con la radio in CW. In Digi, dove di solito si trova una radio usata come front-end I/Q, non c'è nulla da aggiungere.

    QRP Labs avverte che la modalità I/Q non è adatta a WSJT-X e agli altri programmi che si aspettano audio demodulato; qui la demodulazione avviene da questo lato, quindi va bene.
settings-radio-rs-hfiq-5-w-hf = RS-HFIQ (ricetrasmettitore HF da 5 W)
settings-radio-hobbypcb-s-rs-hfiq-there =
    L'RS-HFIQ di HobbyPCB. Qui non c'è nulla da scegliere: il profilo è l'intero set di comandi della radio, e la versione del firmware viene registrata all'apertura della porta.

    Il collegamento di controllo porta due sole cose — dove mettere l'oscillatore, e se trasmettere. Non c'è alcun comando di modo, potenza, filtro, squelch o misuratore, perché la radio non ne ha. Tutto il resto è di sdroxide: ciò che esce dalla scheda audio è banda base complessa centrata sulla sintonia, quindi modo, filtro e modulazione avvengono tutti da questo lato.

    Il comando di frequenza copre 3–30 MHz. Se ne chiedi una fuori da questo intervallo, la radio la rifiuta e resta dov'è; il log lo segnala.

    Il CW viene manipolato come audio attraverso la catena di trasmissione — questa radio non ha un manipolatore che accetti testo. (Ha un generatore CW interno, che sdroxide non usa mai, come consiglia il firmware stesso.)
settings-radio-sound-format-baud-ptt-are = Formato audio, velocità di baud e PTT sono della radio, non impostazioni
settings-radio-all-three-have-been-filled =
    Tutti e tre sono stati compilati e nessuno di essi è una preferenza:

    • Il formato audio è I/Q (stereo) — la scheda trasporta banda base complessa e nient'altro.
    • 57600 baud, 8N1 — l'unica velocità del firmware, senza alcun menu per cambiarla. Qualunque altra è un collegamento muto, quindi viene fissata all'apertura della porta.
    • PTT via CAT — il comando *X è l'unico interruttore di trasmissione che l'interfaccia ha.

    L'offset del centro è impostato a zero: il sintetizzatore lavora a quattro volte la sintonia verso un rivelatore in quadratura, quindi il centro dell'ampiezza è la sintonia. Imposta la frequenza di campionamento qui sopra su quella a cui gira davvero la tua scheda audio; è ciò che rende il panadapter largo come è.
settings-radio-dl2man-pe1nnz-s-pocket-qrp =
    Il ricetrasmettitore QRP tascabile di DL2MAN/PE1NNZ e il firmware aperto uSDX da cui è nato. Emula un Kenwood TS-480, ma il sottoinsieme è scarno: risponde solo a sintonia, modo, PTT e azzeramento RIT/XIT. Nessun S-meter, nessun ROS, nessun controllo di potenza, nessun VFO B, nessuno split e nessun manipolatore CAT, quindi il cursore Pilotaggio e i misuratori non fanno nulla via CAT — il livello audio è l'unico controllo di trasmissione che esiste.

    Il CW viene manipolato come audio (MCW) attraverso la banda laterale, oppure con il tasto o il paddle sulla radio; non esiste alcun comando che accetti testo.
settings-radio-audio = Audio
settings-radio-tr-usdx-has-no-sound =
    Un (tr)uSDX non ha una scheda audio propria e due modi per essere ascoltato, e quale sia quello giusto dipende dalla tua stazione più che dalla radio.

    "Un cavo" porta l'audio dentro il collegamento seriale CAT stesso — il flusso a 8 bit della radio, sullo stesso cavo USB del controllo, senza nient'altro da collegare. Il rovescio è il firmware: non può ricevere un comando CAT mentre il suo flusso è in corso (un comando inviato dentro il flusso non lo mette in pausa, lo fa cadere), quindi questo modo non interroga mai. Sintonia e cambi di modo vengono inviati man mano che li fai, ciascuno mettendo in pausa il flusso per un attimo e riavviandolo, quindi aspettati una breve interruzione a ogni cambio; quando ascolti soltanto non si invia nulla. Anche la sintonia e il modo propri della radio non vengono seguiti — non c'è una lettura con cui seguirli — quindi pilotala da qui.

    "Scheda audio USB" prende l'audio da una scheda audio esterna collegata alla presa 3,5 mm altoparlante/microfono della radio, e si comporta come ogni altro apparato CAT: l'interrogazione della sintonia è attiva, quindi la manopola e il modo della radio vengono seguiti. Scegli la scheda sotto Audio della radio qui sotto.

    Entrambi i modi aprono la porta con DTR alto: il DTR dell'adattatore seriale è collegato al reset della radio, quindi aprire la porta la riavvia e il primo secondo o due è silenzioso mentre si avvia. DTR non viene mai usato per attivare la trasmissione.
settings-radio-radio-model = Modello della radio
settings-radio-which-icom-two-things-ci =
    Quale Icom, per le due cose che il CI-V non fa allo stesso modo su tutti.

    L'indirizzo del ricetrasmettitore qui sotto viene compilato da questa scelta: ogni modello ha un indirizzo diverso, e un frame inviato all'indirizzo sbagliato viene semplicemente ignorato — una radio che non risponde nulla, senza alcun errore che spieghi perché.

    Decide anche se sdroxide può selezionare la modalità DATA. Sul CI-V, USB e USB-DATA hanno lo stesso byte di modo e un comando separato le distingue; senza di esso un collegamento per modi digitali va attraverso l'ingresso microfono, con l'elaborazione vocale e il filtro SSB dell'apparato nel percorso.

    "Altro" lascia l'indirizzo a te e non invia alcun comando di modalità DATA.
settings-radio-radio-id-hex = ID radio (esadecimale)
settings-radio-show-radio-s-spectrum-scope = Mostra lo scope spettrale della radio
settings-radio-scope-span = Ampiezza dello scope
settings-radio-how-wide-sweep-radio-keeps = Quanto ampia è la scansione. La radio mantiene l'ampiezza scelta per ultima sul proprio schermo — spesso qualche kHz. Impostare qui un'ampiezza mette anche lo scope in modalità centro, quindi segue la sintonia. Cambia anche la visualizzazione della radio stessa; "Come impostato sulla radio" la lascia com'è.
settings-radio-press-apply-reconnect-switch-without = Premi "Applica / riconnetti" per cambiare senza riavviare.
settings-radio-radio-audio-sound-card = Audio della radio (scheda audio)
settings-radio-waiting-sound-cards-machine-radio = In attesa delle schede audio sulla macchina a cui è collegata la radio.
settings-radio-radio-rx = Dalla radio (RX)
settings-radio-radio-tx = Alla radio (TX)
settings-radio-apply-reconnect = Applica / riconnetti
settings-radio-reopen-cat-rig-these-sound = Riapri l'apparato CAT con queste schede audio — senza riavviare
settings-radio-reconnects-radio-without-restarting = Ricollega la radio senza riavviare.
settings-radio-receive-radio-pc = Ricezione (radio → PC)
settings-radio-sound-card-radio-s-own = La scheda audio su cui arriva l'audio proprio della radio — la sua presa cuffia verso l'ingresso linea o microfono del computer. Tutto ciò che è nel panadapter e nei decodificatori arriva qui.
settings-radio-transmit-pc-radio-mic = Trasmissione (PC → microfono della radio)
settings-radio-sound-card-carries-audio-radio = La scheda audio che trasporta l'audio che la radio deve trasmettere — l'uscita del computer verso la presa microfono della radio. La radio si attiva da sola, con VOX, nel momento in cui arriva audio qui, quindi questo dispositivo è silenzioso a meno che la radio non stia trasmettendo.
settings-radio-radio-has-no-control-cable = Questa radio non ha un cavo di controllo — si attiva da sola con VOX, quindi non c'è alcun PTT da comandare. La trasmissione è audio verso il microfono della radio.
settings-radio-reopen-radio-these-sound-cards = Riapri la radio con queste schede audio — senza riavviare
settings-radio-devices = Dispositivi
settings-radio-discover = Cerca
settings-radio-no-devices-press-discover = nessun dispositivo — premi Cerca
settings-radio-manual-ip = IP manuale
settings-radio-optional-e-g-192-168 = facoltativo, ad es. 192.168.1.50
settings-radio-sample-rate = Frequenza di campionamento
settings-radio-receiver-ddc = Ricevitore (DDC)
settings-radio-protocol-2-board-carries-several = Una scheda Protocollo 2 porta più ricevitori sintonizzabili in modo indipendente (DDC) su un'unica connessione — fai girare questa radio su DDC1 e un'altra radio, stesso indirizzo, su DDC2. Il trasmettitore appartiene alla radio su DDC1. Le schede Protocollo 1 hanno solo DDC1.
settings-radio-lna-gain = Guadagno LNA
settings-radio-front-end-gain-hermes-lite = Guadagno del front-end di un Hermes-Lite 2. Ha effetto subito — senza riconnettere — e viene ricordato come il livello da cui la radio parte. Troppo alto satura l'ADC e tutta la banda appare distorta; troppo basso e il ricevitore diventa sordo.
settings-radio-overload-protection = Protezione da sovraccarico
settings-radio-back-lna-gain-off-itself =
    Riduce da sé il guadagno LNA mentre la scheda segnala che il suo ADC va in overflow, e lo rialza quando smette.

    Un Hermes-Lite 2 campiona tutto lo 0-38 MHz su un unico convertitore a 12 bit senza mixer né preselettore davanti, quindi una stazione di radiodiffusione a una banda di distanza può portarlo in overflow mentre la banda che guardi non mostra nulla di sbagliato — il rumore di fondo sale, tutto intermodula e i decodificatori si fermano. La scheda lo sa, e questo è ciò che agisce su di essa.

    Disattivato di default: sposta un comando che hai impostato tu. Non succede nulla durante la trasmissione.
settings-radio-wind-gain-back-when-adc = Riduci il guadagno quando l'ADC va in overflow
settings-radio-applies-apply-reconnect-main-window = Ha effetto con Applica / riconnetti. La barra del guadagno della finestra principale mostra il guadagno che la scheda sta davvero usando, quindi segue il ciclo e puoi osservarne l'effetto; il cursore qui sopra è il livello da cui la radio parte, e il ciclo non lo riscrive.
settings-radio-step = {"  "}Passo
settings-radio-how-far-gain-moves-each = Di quanto si sposta il guadagno ogni volta, in dB. Uno è il passo del registro di guadagno proprio della scheda.
settings-radio-attack-decay = {"  "}Attacco / rilascio
settings-radio-how-often-gain-may-come =
    Ogni quanto il guadagno può scendere mentre il convertitore è in overflow, e ogni quanto può risalire una volta che è smesso.

    I due sono volutamente lontani cento volte. Arretra subito: ogni millisecondo di overflow è un ricevitore pieno di intermodulazione. Torna piano: qualunque cosa l'abbia causato — un vicino che trasmette, una radio di diffusione che si accende al tramonto — di solito non è sparita, e un ciclo che si riprendesse veloce quanto arretra passerebbe la serata a oscillare attorno alla soglia. 100 ms e 10 s per decibel sono ciò che hanno usato sia l'Auto S-Att di PowerSDR sia l'HermesIntf di N1GP.
settings-radio-range = {"  "}Intervallo
settings-radio-lowest-highest-gain-loop-may = Il guadagno più basso e più alto che il ciclo può usare, in dB. Il tetto serve a impedire che decida quanto debba essere sensibile il tuo ricevitore; il minimo è dove dichiari che, sotto un certo punto, il sovraccarico è un problema di qualcun altro e la risposta è un filtro, non altri venti decibel.
settings-radio-filter-board = Scheda filtri
settings-radio-accessory-board-hermes-lite-2 = Scheda accessoria sul connettore J16 dell'Hermes-Lite 2 (o sulle uscite open-collector di qualunque altra scheda openHPSDR — anche Protocollo 2). "N2ADR" sceglie un relè per banda; "Codice banda Alex / Hermes" mette il numero della banda sulle uscite 1-4, che è ciò che si aspettano la scheda Alex di un ANAN, uno Zeus SDR, un HiQSDR e Quisk. "Personalizzata" apre una tabella qui sotto in cui indichi tu le sette uscite, banda per banda, in ricezione e in trasmissione — per un commutatore d'antenna, il decodificatore di banda di un amplificatore o qualsiasi altra cosa che nessuno dei due preset copra; uno dei due preset può essere copiato in essa come punto di partenza. Lascia questo su "Nessuna" a meno che non sia davvero montato qualcosa: quei sette pin sono uscite open-collector di uso generale, e gli operatori le collegano anche al PTT di amplificatori, ai relè d'antenna e alla commutazione dei transverter. Pilotarle con i dati di banda avvierebbe qualunque cosa sia collegata. Ha effetto con Applica / riconnetti.
settings-radio-io-board-rx-input = Ingresso RX della scheda IO
settings-radio-where-n2adr-hl2ioboard-takes-receive = Da dove una scheda N2ADR HL2IOBoard prende il segnale di ricezione. La scheda stessa viene trovata automaticamente e non richiede impostazioni; questa esiste solo per chi ha cablato le sue prese SMA. Altrimenti lascia su "Ingresso proprio della radio" — selezionare il J9 della scheda IO senza nulla collegato lascia il ricevitore sordo. Ha effetto con Applica / riconnetti. Impostato su una delle due scelte J9, gli ingressi compaiono anche sul comando ANT e vengono ricordati per banda.
settings-radio-power-amplifier = Amplificatore di potenza
settings-radio-use-hermes-lite-2-s = Usa il PA di bordo dell'Hermes-Lite 2
settings-radio-default-what-you-want-unless = Attivo di default, ed è ciò che vuoi a meno che un amplificatore esterno non venga pilotato dall'uscita RF1 a bassa potenza della scheda. Disattivato, la radio si attiva comunque — il relè T/R scatta e ogni scheda accessoria lo segue — ma la presa d'antenna non produce alcuna potenza, e il relè è volutamente tenuto in ricezione. Ignorato sulle schede diverse da un Hermes-Lite.
settings-radio-mirror-board-s-spectrum-about = Specchia lo spettro della scheda rispetto alla sintonia, in trasmissione così come in ricezione. Attivo di default: un Hermes-Lite 2 ne ha bisogno. Disattivalo solo se i segnali compaiono dalla parte sbagliata della sintonia e nulla viene decodificato — il rivelatore è una waterfall piena di tracce convincenti mentre la SSB cade sulla banda laterale sbagliata e FT8 non restituisce alcuna decodifica.
settings-radio-frequency-correction = Correzione della frequenza
settings-radio-crystal-tcxo-error-ppm-applied = Errore del cristallo/TCXO in ppm, applicato a RX e TX. Ha effetto subito.
settings-radio-ppm = {" "}ppm
settings-radio-transmit-buffer = Buffer di trasmissione
settings-radio-how-far-ahead-real-time = Quanto in anticipo rispetto al tempo reale l'audio di trasmissione viene inviato alla scheda via rete, prima che sdroxide rallenti per adeguarsi. La scheda stessa non conserva alcun buffer di questo tipo — questo amplia soltanto il margine proprio di sdroxide. Alzalo su un collegamento WiFi o una VPN, dove il valore predefinito basso (giusto per una connessione cablata diretta) non dà margine sufficiente contro il jitter e l'audio trasmesso o il PTT si spezzano; un valore più alto costa latenza in trasmissione. Ha effetto con APPLICA, che ricollega la scheda.
settings-radio-puresignal = PureSignal
settings-radio-adaptive-predistortion-linearise-transmitter-sample = Predistorsione adattiva: linearizza il trasmettitore a partire da un campione di ciò che ha effettivamente emesso, per una ventina di decibel in meno di intermodulazione alla stessa potenza. Il ricevitore è il percorso di retroazione — la scheda continua a ricevere durante una trasmissione — quindi un accoppiatore direzionale e un attenuatore devono portare un campione dell'uscita dell'amplificatore su un ingresso che il commutatore T/R non toglie in trasmissione. Su un Hermes-Lite 2 vuol dire la presa PureSignal della scheda IO, con l'ingresso di ricezione qui sopra impostato di conseguenza; su una scheda che non ne ha, la porta RX di un ANAN, oppure un relè esterno che inserisce l'accoppiatore per la durata della trasmissione. È questo ricevitore che viene letto — una scheda con un secondo ADC non può usare RX2 come presa di retroazione, comunque il suo firmware lo instradi. Senza nulla accoppiato, il ciclo non si aggancia mai e il trasmettitore resta esattamente come sarebbe stato. Ha effetto con Applica / riconnetti, e solo sulla radio che possiede il trasmettitore (DDC1).
settings-radio-correct-transmitter-receiver-s-own = Correggi il trasmettitore dalla retroazione del ricevitore stesso
settings-radio-table-steps = {"  "}Passi della tabella
settings-radio-how-finely-amplifier-s-curve = Quanto finemente viene modellata la curva dell'amplificatore. Più passi seguono un ginocchio più netto e impiegano più tempo a riempirsi; 32 è un buon punto di partenza.
settings-radio-adaptation = {"  "}Adattamento
settings-radio-how-fast-correction-follows-what = Quanto velocemente la correzione segue ciò che riporta l'accoppiatore. Lento è giusto — si tratta di mediare la curva di un amplificatore, che non cambia, a partire da un percorso di retroazione che ha del rumore.
settings-radio-hold = Blocca
settings-radio-stop-adapting-keep-correction-where = Smetti di adattarti e mantieni la correzione com'è — per misurare, e per un operatore soddisfatto di ciò che ha appreso.
settings-radio-manual-ip-overrides-discovery-press = Un IP manuale prevale sulla ricerca. Premi "Applica / riconnetti" per cambiare senza riavviare.
settings-radio-open-collector-outputs-band = Uscite open-collector per banda
settings-radio-one-control-word-per-band = Una parola di controllo per banda, come la documentazione del tuo hardware la descrive: il bit 0 è l'uscita 1, il bit 6 l'uscita 7. RX è attiva durante la ricezione su quella banda e TX mentre il trasmettitore è attivato — dai lo stesso valore a entrambi per un filtro, e valori diversi per tutto ciò che appartiene a un solo lato della commutazione (la linea di comando di un amplificatore, il bypass di un preamplificatore di ricezione). Le bande lasciate a 00 non attivano nulla. Ha effetto con Applica / riconnetti.
settings-radio-alex-band-code = CODICE BANDA ALEX
settings-radio-replace-table-what-preset-would = Sostituisce la tabella con ciò che questo preset invierebbe su ogni banda, come punto di partenza da modificare. Nient'altro nella configurazione cambia.
settings-radio-clear = CANCELLA
settings-radio-every-output-off-every-band = Ogni uscita spenta su ogni banda, che è ciò che fa "Nessuna".
settings-radio-band = Banda
settings-radio-rx = RX
settings-radio-tx = TX
settings-radio-outputs-asserted = Uscite attive
settings-radio-other = Altro
settings-radio-everywhere-outside-amateur-bands-short = Ovunque fuori dalle bande radioamatoriali — ascolto in onde corte, e tutto ciò su cui cade la sintonia di un transverter che nessuna banda copre.
settings-radio-dongle = Chiavetta
settings-radio-rescan = Riesamina
settings-radio-re-list-usb-bus-no = Rielenca il bus USB. Nessun dispositivo viene aperto, quindi è sicuro premerlo durante la ricezione.
settings-radio-no-dongles-press-rescan = nessuna chiavetta — premi Riesamina
settings-radio-first-one-found = — la prima trovata —
settings-radio-rtl2832u-s-resampler-reaches-225 = Il ricampionatore dell'RTL2832U arriva a 225–300 kHz e da 900 kHz a 3,2 MHz, niente di mezzo. Ha effetto con Applica.
settings-radio-often-drops-samples = {"  "}(spesso perde campioni)
settings-radio-agc = AGC
settings-radio-manual-setting-measurement-weak-signal = Manuale è l'impostazione per le misure e per i modi digitali a segnale debole. Il sintonizzatore e il demodulatore hanno anelli automatici indipendenti.
settings-radio-tuner-gain = Guadagno del sintonizzatore
settings-radio-applies-immediately-no-reconnect-tuner = Ha effetto subito — senza riconnettere. Il sintonizzatore ha 29 passi discreti, quindi il valore si aggancia al più vicino che può produrre. Ignorato mentre l'AGC del sintonizzatore è attivo.
settings-radio-crystal-error-parts-per-million = Errore del cristallo in parti per milione. Avvia con RUST_LOG=sdroxide_rtlsdr=debug e il log stampa l'errore di clock misurato dopo circa 20 secondi — è il numero da inserire. Ha effetto subito.
settings-radio-hf-reception = Ricezione HF
settings-radio-tuner-itself-starts-24-mhz =
    Il sintonizzatore stesso parte da 24 MHz. Un RTL-SDR Blog V4 converte verso l'alto sotto quella frequenza via hardware; le altre chiavette raggiungono le HF solo campionando direttamente l'ADC, attraverso la porta HF del V3. Automatico affida all'ADC tutto ciò che è sotto i 24 MHz e al sintonizzatore tutto ciò che è sopra.

    Il campionamento diretto copre ogni banda HF, compresi i 17 m e i 15 m — arrivano nella seconda zona di Nyquist dell'ADC, con il verso giusto. Però nulla filtra l'ingresso dell'ADC, quindi insieme a loro arriva qualsiasi cosa si trovi a 28,8 MHz meno la sintonia: 10,7 MHz sotto i 17 m, 7,726 MHz sotto i 15 m.

    Cambiare modo interrompe brevemente il flusso.
settings-radio-removes-dongle-s-own-dc =
    Elimina il picco DC proprio della chiavetta dal centro dell'ampiezza, e l'immagine speculare che ogni segnale lascia riflessa rispetto ad esso, misurando lo squilibrio nei campioni stessi — nessuna calibrazione, e ha effetto subito. Il sintonizzatore non ha un modo di sintonia a offset, quindi questo è l'unico modo per liberare il centro.

    Anche una portante AM sintonizzata esattamente sulla frequenza sta in DC, quindi va insieme al picco: sintonizzati a un chilohertz di distanza, oppure disattiva questo.
settings-radio-remove-centre-spike-mirror-image = Rimuovi il picco centrale e l'immagine speculare
settings-radio-bias-tee = Bias tee
settings-radio-feed-4-5-v-dc = Invia circa 4,5 V DC lungo il cavo coassiale
settings-radio-bias-tee-never-connect-transceiver = Il bias tee è ATTIVO. Non collegare mai un ricetrasmettitore, un'antenna a massa o un preamplificatore alimentato altrove mentre questo è attivo — la corrente continua scende direttamente lungo la linea di alimentazione.
settings-radio-receive-only-dongle-sample-rate = Solo ricezione. La chiavetta e la frequenza di campionamento hanno effetto con Applica; tutto il resto ha effetto mentre lo modifichi.
settings-radio-server-address = Indirizzo del server
settings-radio-where-rtl-tcp-listening-address =
    Dove è in ascolto rtl_tcp: un indirizzo, oppure un indirizzo e una porta. La porta predefinita è 1234, che è quella di rtl_tcp.

    Sull'altro capo, avvialo come `rtl_tcp -a 0.0.0.0` — legato a 127.0.0.1, come fa senza -a, accetta connessioni solo dalla stessa macchina.

    Ha effetto con Applica.
settings-radio-host-host-port-e-g = host o host:porta, ad es. raspberrypi.local:1234
settings-radio-requested-server-same-resampler-limits =
    Richiesto al server, con gli stessi limiti del ricampionatore di una chiavetta locale — è lo stesso silicio dall'altro capo.

    La cifra accanto a ogni frequenza è ciò che costa sul collegamento: i campioni vengono inviati non compressi, e una frequenza che la rete non regge fa sì che rtl_tcp chiuda la connessione anziché degradarsi. Ha effetto con Applica.
settings-radio-wired-link = {"  "}(collegamento cablato)
settings-radio-runs-server-s-dongle-manual = Gira sulla chiavetta del server. Manuale è l'impostazione per le misure e per i modi digitali a segnale debole.
settings-radio-applies-immediately-no-reconnect-sent = Ha effetto subito — senza riconnettere. Inviato in decimi di dB e agganciato dal server a un passo che il suo sintonizzatore possiede; il protocollo non prevede risposte, quindi non si può leggere su cosa si è stabilizzato, e questo cursore continua a mostrare ciò che è stato richiesto. Ignorato mentre l'AGC del sintonizzatore è attivo.
settings-radio-crystal-error-server-s-dongle =
    Errore del cristallo della chiavetta del *server*, in parti per milione — una caratteristica di quell'hardware, quindi si imposta qui e non sulle chiavette di questa macchina. Ha effetto subito.

    L'errore di clock misurato che stampa l'interfaccia USB non è disponibile qui: in rete ciò che quella misura vede è il buffering, non il cristallo, ed è sbagliato di migliaia di ppm. Calibra la chiavetta via USB una volta e riporta il numero, oppure sintonizza una stazione di radiodiffusione di frequenza nota e regola finché non cade sulla sintonia.
settings-radio-tuner-starts-24-mhz-below =
    Il sintonizzatore parte da 24 MHz; sotto quella frequenza il capo remoto ha bisogno di aiuto. Un Blog V4 converte verso l'alto da solo e Automatico non lo tocca — che è l'unica cosa che può fare, perché il protocollo riporta il chip del sintonizzatore e nient'altro, e un V4 è indistinguibile da un R828D normale sul filo.

    Su un V3 o qualunque altra chiavetta, Automatico fa passare il server al campionamento diretto sotto la soglia di 24 MHz del sintonizzatore — il che copre ogni banda HF, con 17 m e 15 m che arrivano nella seconda zona di Nyquist dell'ADC con 28,8 MHz meno la sintonia ripiegati sopra. Scegli esplicitamente Campionamento diretto per un R828D normale che non sente nulla in HF. Il cambio interrompe brevemente il flusso.
settings-radio-removes-dongle-s-dc-spike =
    Elimina qui il picco DC della chiavetta e la sua immagine speculare, dai campioni man mano che arrivano — sono artefatti dell'hardware, quindi viaggiano in rete con tutto il resto. Ha effetto subito.

    Anche una portante AM sintonizzata esattamente sulla frequenza sta in DC, quindi va insieme al picco: sintonizzati a un chilohertz di distanza, oppure disattiva questo.
settings-radio-powers-preamp-far-end-s = Alimenta un preamplificatore dalla chiavetta del capo remoto. I server più vecchi non implementano il comando e lo ignorano in silenzio — il protocollo non ha modo di dire di no — quindi un bias tee che non si accende non è necessariamente colpa di questo capo.
settings-radio-feed-4-5-v-dc-2 = Invia circa 4,5 V DC lungo il cavo coassiale remoto
settings-radio-bias-tee-hardware-somewhere-else = Il bias tee è ATTIVO, su un hardware che si trova altrove. Ciò che sta sull'altro capo di quella linea di alimentazione — un ricetrasmettitore, un'antenna a massa, un preamplificatore già alimentato — non è davanti a te per controllarlo.
settings-radio-sdrplay-server-rsp-tcp = Server SDRplay (rsp_tcp)
settings-radio-sdrplay-server-greets-exactly-like = Un server SDRplay si presenta esattamente come una chiavetta, quindi questi campi sono mostrati sempre anziché solo quando ne viene rilevato uno. Con un normale server rtl_tcp vengono semplicemente ignorati — il protocollo non ha risposte e scarta i comandi che non conosce. La scheda Dispositivo indica il server come rsp_tcp non appena si è identificato.
settings-radio-input = Ingresso A
settings-radio-input-b = Ingresso B
settings-radio-hi-z = Hi-Z
settings-radio-lna-state = Stato LNA
settings-radio-step-index-not-db-figure = Un indice di passo, non un valore in dB: quanto vale ogni passo dipende dal modello RSP e dalla banda. 0 è il guadagno massimo.
settings-radio-if-gain-reduction = Riduzione del guadagno IF
settings-radio-reduction-so-more-less-signal = Una riduzione, quindi di più significa meno segnale. Obbedita solo con l'AGC dell'RSP disattivato.
settings-radio-enable = Attiva
settings-radio-dbfs = {" "}dBfs
settings-radio-level-rsp-s-agc-aims = Il livello che l'AGC dell'RSP punta a mantenere.
settings-radio-notches = Notch
settings-radio-am = AM
settings-radio-fm = FM
settings-radio-dab = DAB
settings-radio-rf = RF
settings-radio-reference-out = Uscita di riferimento
settings-radio-t-24-mhz-clock-out = Uscita di clock a 24 MHz
settings-radio-rsp2-rspduo-only = Solo RSP2 e RSPduo.
settings-radio-receive-only-address-sample-rate = Solo ricezione. L'indirizzo e la frequenza di campionamento hanno effetto con Applica; tutto il resto ha effetto mentre lo modifichi. Una connessione caduta viene ritentata da sola, quindi un server riavviato torna senza toccare nulla qui.
settings-radio-where-spyserver-listening-address-address =
    Dove è in ascolto spyserver: un indirizzo, oppure un indirizzo e una porta. La porta predefinita è 5555, che è quella di spyserver.

    Sull'altro capo, verifica che il suo file di configurazione si leghi a un indirizzo raggiungibile dalle altre macchine — legato a 127.0.0.1 accetta connessioni solo dalla stessa macchina.

    Ha effetto con Applica.
settings-radio-host-host-port-e-g-2 = host o host:porta, ad es. raspberrypi.local:5555
settings-radio-i-q-bandwidth = Larghezza di banda I/Q
settings-radio-how-wide-slice-band-arrives =
    Quanto è larga la porzione di banda che arriva come I/Q — e quindi quanto del collegamento usa. Questa è la finestra *demodulata*: segue la sintonia, e tutto ciò che è più largo è la FFT del server nella striscia sopra il panadapter.

    Automatico punta a circa 96 kHz, che porta ogni modo qui incluso l'FM larga e sta ancora in un collegamento in salita cellulare. Ha effetto con Applica.
settings-radio-which-stage-server-s-own =
    Quale stadio della scala di frequenze del server chiedere. Ogni ricevitore ha una scala diversa — è la sua frequenza massima dimezzata stadio per stadio — quindi questo viene memorizzato come stadio anziché come cifra in hertz, e la stessa impostazione continua ad avere senso puntata su un altro server.

    Automatico punta a circa 1 Msps. Premi Prova connessione per vedere quali valori danno gli stadi su questo server. Ha effetto con Applica.
settings-radio-automatic = Automatico
settings-radio-sample-format = Formato dei campioni
settings-radio-bits-per-component-wire-so =
    Bit per componente sul filo, e quindi quanto costa sul collegamento: 16 bit sono il doppio di 8 bit, e il float a 32 bit è quattro volte tanto per non più informazione di quella che aveva l'ADC del ricevitore.

    8 bit è ciò che fa funzionare un ricevitore remoto su una linea domestica, ed è giusto per quasi tutto. Un server può imporne un altro — alcuni sono configurati per pretendere un formato — e lo segnala nel log quando lo fa. Ha effetto con Applica.
settings-radio-gain = Guadagno
settings-radio-server-s-gain-stage-index =
    Lo stadio di guadagno del server, come indice — non come numero di decibel. Ciò che ogni indice significa appartiene al ricevitore dall'altro capo e cambia con la banda, e il protocollo non lo dice mai, quindi nulla qui può convertirlo in dB senza inventare una cifra.

    L'intervallo reale è quello del server ed è noto solo una volta connessi; un indice oltre di esso viene limitato. Ha effetto subito — senza riconnettere.
settings-radio-index = indice{" "}
settings-radio-digital-gain = Guadagno digitale
settings-radio-how-far-server-scales-samples =
    Di quanto il server scala i propri campioni verso l'alto prima di quantizzarli per il filo. Automatico osserva ciò che arriva davvero e mantiene il picco a metà del fondo scala, che è l'unica cosa che può andare bene sia su una banda morta sia su una affollata — le due distano trenta decibel, e nessuna cifra fissa serve a entrambe.

    Conta solo a 8 bit, dove un segnale ben sotto il fondo scala perde i bit bassi nel quantizzatore e uno sopra viene tagliato piatto. I formati più ampi hanno margine e ricevono la cifra che calcola ogni altro client, a partire dall'indice di guadagno e dallo stadio di decimazione. Ha effetto subito.
settings-radio-full-band-strip = Striscia a banda piena
settings-radio-server-s-own-fft-whole =
    La FFT propria del server su tutta la banda, disegnata nella striscia sopra il panadapter. In questa interfaccia è l'unica vista di banda che esiste — il panadapter stesso è largo solo quanto l'I/Q ricevuto.

    Disattivandola, un ricevitore non ha più modo di vedere nulla su cui non sia già sintonizzato. Ha effetto subito, con una breve interruzione dell'audio mentre il server cambia ciò che sta inviando.
settings-radio-ask-server-low-rate-fft =
    Chiedi al server anche una FFT a bassa frequenza di tutta la banda oltre all'I/Q, e disegnala nella striscia sopra il panadapter.

    Costa quasi nulla — un paio di kilobyte per fotogramma, una dozzina di volte al secondo — e mostra l'intero ricevitore anziché la porzione che viene demodulata. Vale la pena disattivarla solo su un collegamento dove ogni byte conta. Ha effetto subito, con una breve interruzione dell'audio.
settings-radio-show = Mostra
settings-radio-whole-band = intera banda
settings-radio-how-much-receiver-strip-covers = Quanto del ricevitore copre la striscia. L'intera banda è la vista più larga che esiste; restringerla mette lo stesso numero di bin su meno spettro, cioè un dettaglio più fine su un tratto più piccolo.
settings-radio-strip-db-window = Finestra dB della striscia
settings-radio-server-quantises-fft-into-window =
    Il server quantizza la sua FFT in questa finestra prima di inviarla, un byte per bin — quindi queste decidono quanto finemente viene misurata la striscia, non solo come viene disegnata. Il pavimento e il soffitto con cui la striscia viene *visualizzata* sono impostati dal livellamento automatico del motore e sono un'altra cosa.

    Il valore predefinito di 150 dB è l'intero intervallo del protocollo e non richiede attenzione. Restringerlo attorno al rumore di fondo guadagna risoluzione su un ricevitore i cui segnali stanno tutti in una piccola parte della scala. Ha effetto subito.
settings-radio-top = alto{" "}
settings-radio-range-2 = intervallo{" "}
settings-radio-i-q-correction = Correzione I/Q
settings-radio-remove-dc-spike-mirror-image = Rimuovi il picco DC e l'immagine speculare nel DSP, da questo lato. Se il ricevitore dall'altro capo ne abbia bisogno dipende da cosa sia — un Airspy HF+ no, un RTL-SDR sì — e il protocollo non dice con quale stia parlando. Ha effetto subito.
settings-radio-enabled = Attivo
settings-radio-test-connection = Prova connessione
settings-radio-connect-read-what-server-says = Collegati, leggi cosa dice il server sul suo ricevitore, e disconnettiti di nuovo — senza avviare un flusso, quindi è sicuro da premere su un server che qualcun altro sta usando.
settings-radio-receive-only-panadapter-narrow-i = Solo ricezione. Il panadapter è la finestra I/Q stretta, che segue la sintonia; la vista di banda è la FFT del server nella striscia sopra di esso. Premi "Applica / riconnetti" per cambiare senza riavviare.
settings-radio-receive-only-wideband-i-q = Solo ricezione. I/Q a banda larga, come ogni SDR locale. Premi "Applica / riconnetti" per cambiare senza riavviare.
settings-radio-receiver-address = Indirizzo del ricevitore
settings-radio-where-receiver-serves-web-page =
    Dove il ricevitore serve la sua pagina web: un indirizzo, oppure un indirizzo e una porta. La porta predefinita è 8073, quella di un KiwiSDR.

    Tieni presente che un ricevitore raggiunto tramite il proxy del progetto (qualcosa.proxy.kiwisdr.com — quasi la metà di quelli pubblici) risponde invece sulla porta 80, quindi indicala esplicitamente. Scegliere un ricevitore da "SDR pubblici" compila questo campo correttamente.

    Ha effetto con Applica.
settings-radio-host-host-port-e-g-3 = host o host:porta, ad es. kiwi.example.org:8073
settings-radio-password = Password
settings-radio-receiver-s-user-password-where =
    La password *utente* del ricevitore, quando il suo operatore ne ha impostata una. Vuota su quasi tutti i ricevitori pubblici. Non la password di amministratore.

    Ha effetto con Applica.
settings-radio-announce = Annuncia come
settings-radio-name-connection-shows-up-under =
    Il nome con cui compare questa connessione, visibile al proprietario del ricevitore e a tutti gli altri che lo ascoltano.

    Lasciato vuoto è il tuo nominativo di stazione, che è la convenzione della rete — alcuni operatori bloccano i client che non si identificano. Ha effetto con Applica.
settings-radio-blank-station-callsign = vuoto = nominativo di stazione
settings-radio-band-view = Vista di banda
settings-radio-ask-receiver-own-waterfall-well =
    Chiedi al ricevitore la sua waterfall oltre al suo I/Q, e disegnala nella striscia sopra il panadapter.

    Vale la pena: l'I/Q è largo circa 12 kHz, quindi senza questo non c'è nulla *con cui* sintonizzarsi. È una seconda connessione che trasporta circa 20 kB/s a pieno regime, contro i 44 kB/s dell'I/Q.

    Ha effetto con Applica.
settings-radio-show-receiver-s-0-30 = mostra la waterfall 0-30 MHz del ricevitore
settings-radio-band-view-speed = Velocità della vista di banda
settings-radio-how-often-receiver-sends-waterfall =
    Ogni quanto il ricevitore invia una riga di waterfall, da 1 (più lento) a 4. È l'unica impostazione qui che cambia il costo del collegamento durante il funzionamento, quindi abbassala su una connessione a consumo misurato.

    Ha effetto subito.
settings-radio-band-view-span = Ampiezza della vista di banda
settings-radio-how-much-band-receiver-s =
    Quanta banda copre la waterfall del ricevitore. Tutta la sua 0-30 MHz a sinistra del cursore, dimezzandosi a ogni passo verso destra.

    La waterfall è sempre larga 1024 bin, qualunque sia l'ampiezza della finestra, quindi questo è in realtà un controllo di risoluzione: l'intera banda sta a 29 kHz per bin, il che è una *mappa* di banda anziché un'immagine di qualcosa, mentre poche centinaia di kilohertz mostrano le singole stazioni in una banda di radiodiffusione. Con lo zoom, la finestra segue la tua sintonia.

    L'intera banda è il predefinito ed è ciò che rende la striscia uno strumento per sintonizzarsi; il panadapter sotto di essa è l'I/Q di circa 12 kHz. Ha effetto subito.
settings-radio-receiver-agc = AGC del ricevitore
settings-radio-receiver-s-agc-which-far =
    L'AGC del *ricevitore*, che sta dall'altro lato del collegamento e prima dell'I/Q — quindi agisce prima di qualsiasi cosa faccia sdroxide.

    Attivo di default, a differenza di ogni SDR locale qui, perché l'alternativa misurata era peggiore: su un ricevitore in funzione il guadagno manuale non era monotono lungo la sua gamma e la sua estremità alta tagliava l'I/Q a fondo scala, mentre l'AGC teneva circa -24 dBFS con 7 dB di margine.

    Significa che l'ampiezza dei campioni non è un livello di segnale, ed è per questo che lo S-meter viene letto dalla cifra del ricevitore stesso. Ha effetto subito.
settings-radio-let-receiver-ride-own-gain = lascia che il ricevitore gestisca il proprio guadagno
settings-radio-manual-gain = Guadagno manuale
settings-radio-fixed-gain-when-receiver-s = Guadagno fisso quando l'AGC del ricevitore è disattivato, su una scala propria da 0 a 90. Non sono decibel di nulla di dichiarato — il protocollo lo chiama manGain e non dice altro, ed è stato misurato come non monotono, quindi trattalo come una manopola per trovare un buon punto e non come una calibrazione. Ha effetto subito.
settings-radio-read-receiver-s-status-page = Leggi la pagina di stato del ricevitore: cos'è, cosa copre, quanti canali sono liberi, e se il suo operatore consente connessioni da app diverse dal browser. Non occupa nessuno dei suoi canali.
settings-radio-host-port-e-g-127 = host:porta, ad es. 127.0.0.1:50001
settings-radio-iq-sample-rate = Frequenza di campionamento IQ
settings-radio-receiver = Ricevitore
settings-radio-rig-two-receivers-sunsdr2dx-can = Un apparato con due ricevitori (SunSDR2DX) può servire due schede radio da un'unica connessione — fai girare questa radio su RX1 e un'altra su RX2. Il trasmettitore appartiene alla radio su RX1.
settings-radio-stream-delay = Ritardo del flusso
settings-radio-how-long-rig-s-iq = Quanto tempo impiega l'I/Q dell'apparato per arrivare su un nuovo centro dopo che sdroxide lo sposta. È la pipeline DSP dell'apparato, non la rete — anche un server su questa macchina ne ha una — e mentre il panadapter viene trascinato con zoom massimo in fuori, sbagliarsi di questo tanto sposta lateralmente le righe più recenti della waterfall rispetto alla storia, di tanto quanto è l'errore per la velocità del trascinamento. Un valore troppo alto le sposta esattamente quanto uno troppo basso, nel verso opposto. Misuralo con `cargo run --release -p sdroxide-tci --example retune_latency`; il predefinito è un SunSDR2DX su ExpertSDR3 a 192 kHz.
settings-radio-wideband-iq-receive-audio-transmit = I/Q a banda larga in ricezione, audio in trasmissione. Premi "Applica / riconnetti" per cambiare senza riavviare.
settings-radio-radio-address = Indirizzo della radio
settings-radio-address-shown-radio-under-set = L'indirizzo mostrato sulla radio sotto SET > Network. Lì deve essere attivo Network Control, e la radio deve avere un nome utente e una password di rete impostati.
settings-radio-host-ip-e-g-192 = host o IP, ad es. 192.168.1.50
settings-radio-control-port = Porta di controllo
settings-radio-t-50001-unless-has-been-changed = 50001 a meno che non sia stata cambiata sulla radio.
settings-radio-network-user = Utente di rete
settings-radio-stored-clear-radio-json-protocol = Salvata in chiaro in radio.json. Il protocollo la offusca in modo reversibile sul filo, quindi nulla qui la renderebbe un segreto.
settings-radio-receive = Ricevi da
settings-radio-af-radio-demodulates-sdroxide-shows = AF: la radio demodula e sdroxide mostra la banda audio. 12 kHz IF: la radio invia invece la sua frequenza intermedia DRM e sdroxide demodula, portando in gioco i propri filtri, la riduzione del rumore e i decodificatori su circa ±12 kHz. In entrambi i casi la waterfall larga è lo scope proprio della radio — nessun Icom invia I/Q. Una preferenza IF usa temporaneamente AF per modi più larghi come l'WFM, poi ripristina IF per i modi compatibili. Una preferenza AF resta AF. HD Radio non è disponibile su questa connessione.
settings-radio-audio-sample-rate = Frequenza di campionamento audio
settings-radio-t-12-khz-if-needs-48000 = Una IF a 12 kHz richiede il flusso a 48000 Hz — a questa frequenza viene usato l'audio demodulato della radio.
settings-radio-if-spectrum = Spettro IF
settings-radio-which-way-round-radio-s = In che verso gira la IF a 12 kHz della radio. "Automatico" usa ciò che il modello è noto per fare — specchiata su un IC-7760, normale su ogni altro Icom. Impostala a mano se la SSB esce sulla banda laterale sbagliata: il segnale è dover selezionare USB dove la banda va in LSB, mentre il display di modo della radio concorda con sdroxide per tutto il tempo.
settings-radio-normal = Normale
settings-radio-mirrored = Specchiata
settings-radio-displayed-bandwidth = Larghezza di banda visualizzata
settings-radio-width-audio-band-panadapter-cat = Larghezza del panadapter della banda audio, come per un apparato CAT. Usata nei modi digitali e dove lo scope della radio è spento; altrimenti il panadapter è lo scope, e Ampiezza dello scope ne imposta la larghezza.
settings-radio-how-cw-panel-s-keyer-2 = Come trasmette il manipolatore del pannello CW. "Manipolatore dell'apparato" mette la radio in CW e le consegna il testo da inviare con il proprio manipolatore (CI-V 0x17). "Scheda audio" invia invece il sidetone manipolato sulla LAN come audio (MCW), un tono a sintonia + tono — e poiché un apparato in CW ignorerebbe il suo ingresso modulatore, selezionando CW la radio resta su USB semplice, lo stesso modo su cui viaggiano i modi digitali.
settings-radio-how-much-audio-radio-holds = Quanto audio trattiene la radio prima di modulare. Di più sopravvive a una rete peggiore, al costo della latenza in trasmissione.
settings-radio-streams-radio-s-own-475 = Trasmette lo sweep proprio della radio di 475 bin. Su AF è il panadapter — l'audio che la radio invia è ciò che è passato dal suo filtro, non un'immagine della banda — e sulla IF a 12 kHz è la waterfall a banda piena sopra uno di essi. In entrambi i casi è l'immagine della radio, non il DSP di sdroxide: non c'è I/Q da cui calcolarne una.
settings-radio-how-wide-sweep-only-wide = Quanto ampio è lo sweep. È l'unica vista ampia che un Icom abbia, e la radio mantiene l'ampiezza scelta per ultima sul proprio schermo — spesso qualche kHz, motivo per cui la striscia può comparire non più larga del panadapter sotto di essa. Impostare qui un'ampiezza mette anche lo scope in modalità centro, quindi segue la sintonia. Cambia anche la visualizzazione della radio stessa; "Come impostato sulla radio" la lascia com'è.
settings-radio-switch-modulation-input-lan = Passa l'ingresso di modulazione a LAN
settings-radio-transmit-audio-only-heard-when = L'audio di trasmissione si sente solo quando l'ingresso MOD della radio è impostato su LAN. sdroxide può scriverlo su un modello di cui conosce la numerazione dei menu; su un altro lo dice e lascia il menu com'è. È un prestito: ciò che la radio aveva lì viene letto per primo e rimesso al suo posto alla fine della sessione, così un apparato usato da solo in seguito sente ancora il proprio microfono.
settings-radio-copy-diagnostic-report = Copia il rapporto diagnostico
settings-radio-copies-radio-s-last-session = Copia l'ultima sessione di questa radio — il suo handshake e la traccia CI-V — negli appunti, per una segnalazione di bug. Di questa radio, non della stazione: con due Icom sulla LAN ogni scheda risponde per il proprio indirizzo.
settings-radio-control-audio-radio-s-own = Controllo, audio e lo scope proprio della radio su un'unica connessione di rete — nessun cavo seriale e nessuna scheda audio. Premi "Applica / riconnetti" per cambiare senza riavviare.
settings-radio-testing = Prova in corso…
settings-radio-was-only-check-press-apply = Era solo una verifica — premi Applica / riconnetti qui sotto per iniziare a usare questa radio.
settings-radio-radios = Radio
settings-radio-asks-network-iio-devices-also = Chiede alla rete i dispositivi IIO, e prova anche direttamente 192.168.2.1 — un Pluto alla fine di un cavo USB spesso non è raggiungibile via multicast anche se l'indirizzo funziona.
settings-radio-no-radios-press-discover = nessuna radio — premi Cerca
settings-radio-address = Indirizzo
settings-radio-overrides-selection-above-usb-cable = Prevale sulla selezione qui sopra. Il cavo USB presenta il Pluto come scheda di rete, non come porta seriale, quindi è un indirizzo IP anche quando la radio è sulla tua scrivania.
settings-radio-pluto-revision-c-pluto-unlocked = Un Pluto+ o un Pluto di revisione C sbloccato a 2R2T può servire due schede radio da una sola unità — questa radio su RX1 e un'altra su RX2, ciascuna sulla propria antenna. Le due catene condividono un unico oscillatore, quindi sintonizzare una delle due radio sposta entrambe; ciò che RX2 offre è una seconda antenna sullo stesso spettro (diversità), non una seconda banda. Il trasmettitore appartiene alla radio su RX1. Un Pluto 1R1T di serie rifiuta RX2 al collegamento.
settings-radio-width-spectrum-sdroxide-receives-ad9361 = Ampiezza dello spettro che sdroxide riceve. L'AD9361 arriva a 61,44 Msps; il collegamento di rete USB no, ed è a questo che è scalato l'elenco. Le frequenze più basse richiedono una configurazione del filtro caricata nell'AD9361, cosa che sdroxide non fa — un Pluto di serie le fa funzionare a circa 2,084 Msps e lo dice al collegamento. Ha effetto con Applica.
settings-radio-more-than-usb-2-will = {"  "}(più di quanto il USB 2 possa portare)
settings-radio-stock-pluto-runs-2-084 = {"  "}(un Pluto di serie gira a 2,084)
settings-radio-analog-filter = Filtro analogico
settings-radio-ad9361-s-baseband-filter-leave = Il filtro di banda base dell'AD9361. Lascia a 0 per automatico, che lo apre fino a nove decimi della frequenza di campionamento — largo di proposito, perché il ricevitore sposta il proprio oscillatore di un quarto di ampiezza lontano dalla sintonia per tenere i segnali lontani dal picco DC, e un filtro stretto taglierebbe proprio la parte in cui li ha spostati. Ha effetto con Applica.
settings-radio-ad9361-has-four-modes-not = L'AD9361 ha quattro modi, non un interruttore on/off. Attacco lento va bene per SSB e CW; attacco veloce per i segnali a raffica; manuale è l'impostazione per le misure e per i modi digitali a segnale debole. Ha effetto subito.
settings-radio-rx-gain = Guadagno RX
settings-radio-applies-immediately-no-reconnect-ignored = Ha effetto subito — senza riconnettere. Ignorato a meno che l'AGC non sia impostato su manuale, che è il comportamento dell'AD9361 stesso, non di sdroxide.
settings-radio-tx-gain = Guadagno TX
settings-radio-negative-because-ad9361-expresses-transmit = Negativo perché l'AD9361 esprime il livello di trasmissione come attenuazione: 0 dB è l'uscita piena. Ha effetto subito. Il trasmettitore viene portato al suo livello più basso prima che questo valore venga applicato alla connessione, così nulla lasciato dal programma precedente può essere attivo.
settings-radio-reference-error-parts-per-million = Errore di riferimento in parti per milione. Avvia con RUST_LOG=sdroxide_pluto=debug e il log stampa l'errore di clock misurato dopo circa 20 secondi — è il numero da inserire. Applicato da sdroxide, non scritto nella correzione persistente della radio. Ha effetto subito.
settings-radio-full-duplex = Full duplex
settings-radio-keep-receiving-while-transmitting = Continua a ricevere durante la trasmissione
settings-radio-leave-off-pluto-reached-over =
    Lascialo disattivato su un Pluto raggiunto tramite il suo cavo USB. Quel collegamento non può portare un megasample al secondo in entrambe le direzioni contemporaneamente, e una trasmissione che affama il buffer di trasmissione esce a scatti — quindi per impostazione predefinita la ricezione si ferma per la durata di una trasmissione.

    Attivalo per una scheda su Ethernet vero (un LibreSDR, un Pluto su un adattatore gigabit), dove c'è spazio per entrambi: sentirai così il ricevitore durante la tua stessa trasmissione, che è il modo in cui una stazione QO-100 ascolta il proprio downlink.

    Una trasmissione richiede il doppio della frequenza di campionamento in banda di collegamento — 2,5 Msps sono 10 MB/s in ciascuna direzione — quindi abbassa la frequenza se il log comincia a dire che il collegamento non la regge. La panoramica mostra ancora il segnale trasmesso durante una trasmissione; è l'audio a continuare ad arrivare.

    Ha effetto con Applica.
settings-radio-duplex = Duplex
settings-radio-whether-ad9361-runs-both-directions =
    Se l'AD9361 funziona in entrambe le direzioni contemporaneamente (FDD, che è come un Pluto si avvia e ciò in cui sdroxide lo ha sempre lasciato) o una alla volta (TDD).

    Lascia questo su FDD a meno che tu non voglia i pin PTT qui sotto — TDD è ciò da cui quelli attivano, e esclude il Full duplex qui sopra. Ha effetto con Applica.
settings-radio-ptt-pins = Pin PTT
settings-radio-pluto-s-four-gpo-test =
    I quattro punti di test GPO del Pluto possono comandare da soli un amplificatore di potenza esterno, un LNA o un commutatore trasmissione-ricezione. Scegli una coppia e un pin è alto per tutto il tempo in cui la radio riceve, l'altro per tutto il tempo in cui trasmette — nessun software sul computer nel mezzo e nessuna linea PTT seriale da cablare.

    Questo porta la radio in TDD qualunque cosa dica la riga Duplex, perché i pin seguono le linee di abilitazione dell'AD9361 e in FDD entrambe sono attive per tutta la sessione. Il Full duplex si disattiva con esso, quindi lascia entrambi ai valori predefiniti se lavori con i satelliti e ti serve sentire il tuo stesso downlink.

    Analog Devices stessa indica un LNA esterno su GPO0/GPO1, quindi usa GPO2/GPO3 se la tua scheda è cablata così. I pin sono a circa 1,3 V con pochi milliampere: pilota un transistor o un optoisolatore con essi, mai una bobina di relè.

    Ha effetto con Applica.
settings-radio-buffer-size = Dimensione del buffer
settings-radio-how-much-device-holds-before =
    Quanto trattiene il dispositivo prima di ogni trasferimento, in campioni complessi. Il valore predefinito di 32768 sono circa 16 ms a 2 Msps: abbastanza lungo perché il tempo di andata e ritorno di ogni trasferimento non sia il collo di bottiglia, abbastanza corto perché un cambio di sintonia non sia visibilmente in ritardo.

    Dimezzalo se il log segnala che il socket di ricezione viene sostituito. Un collegamento marginale — un Pluto raggiunto via cavo USB a una frequenza di campionamento alta è il caso tipico — si blocca a metà di un trasferimento, e un trasferimento più piccolo è sia meno probabile che venga colto a metà sia più rapido da ripristinare dopo. Alzalo per scambiare latenza di sintonia con meno andate e ritorni.

    Ha effetto con Applica.
settings-radio-samples = {" "}campioni
settings-radio-rx-tx-port = Porta RX / TX
settings-radio-ad9361-s-rf-port-select = L'rf_port_select dell'AD9361. Un Pluto di serie collega uno per tipo, quindi lascia vuoti questi campi a meno che tu non abbia una scheda che non lo fa. Ha effetto con Applica.
settings-radio-opens-radio-reads-what-says = Apre la radio, legge cosa dice di sé, e riporta l'intervallo di sintonia che questa particolare scheda ha. Non avvia alcun flusso.
settings-radio-copies-last-session-s-protocol = Copia la traccia del protocollo dell'ultima sessione negli appunti, per una segnalazione di bug.
settings-radio-wideband-iq-receive-transmit-half = I/Q a banda larga in ricezione e trasmissione. Semi-duplex — la ricezione si ferma per la durata di una trasmissione, perché il collegamento di rete USB non regge entrambe insieme. Non servono SoapySDR né libiio.
settings-radio-not-yet-verified-against-real =Non ancora verificato su hardware reale. Se si comporta male, invia il rapporto diagnostico — contiene ogni comando scambiato con la radio e i primi byte del flusso di campioni.
settings-radio-flexradio-announces-itself-local-network = Una FlexRadio si annuncia sulla rete locale circa una volta al secondo. Una radio raggiunta tramite un router o una VPN non trasmette mai verso di te — inseriscine l'indirizzo qui sotto.
settings-radio-another-gui-client-has-radio = Un altro client grafico ha questa radio e multiFLEX è disattivato. Disconnetti quel client, oppure attiva multiFLEX sulla radio.
settings-radio-overrides-selection-above-use-radio = Prevale sulla selezione qui sopra. Usalo per una radio su un'altra sottorete, dietro una VPN, o su una porta non standard.
settings-radio-width-spectrum-sdroxide-receives-192 = Ampiezza dello spettro che sdroxide riceve. 192 kHz è il massimo della radio per un flusso DAX IQ, quindi l'ampiezza più larga che questa interfaccia può mostrare.
settings-radio-t-192-khz-widest-dax-iq = 192 kHz è il flusso DAX IQ più largo che una FLEX invia, e la radio lega a esso l'ampiezza del panadapter — quindi quella è l'intera ampiezza, e nessuna impostazione qui la allarga.
settings-radio-dax-iq-channel = Canale DAX IQ
settings-radio-radio-has-four-change-only = La radio ne ha quattro. Cambialo solo se qualcos'altro sulla rete sta già usando il canale 1 — la radio rifiuta un canale due volte.
settings-radio-station-name = Nome della stazione
settings-radio-shown-against-session-radio-s = Mostrato accanto a questa sessione nell'elenco client della radio. La radio riconosce anche un client tramite esso, quindi rinominarlo fa sì che la radio tratti sdroxide come uno nuovo.
settings-radio-gui-client-id = ID client grafico
settings-radio-identity-radio-restores-session-s = L'identità con cui la radio ripristina i slice e i panadapter di questa sessione. Lasciato vuoto viene derivato dal nome della stazione — stabile, ma uguale su ogni sdroxide che ha mantenuto il nome predefinito, e una radio risolve un duplicato scacciando il client precedente. sdroxide lo rileva sul filo e ricade su un'identità di una sola sessione, che costa il ripristino; inserisci qui un UUID tuo per mantenerlo.
settings-radio-optional-e-g-uuid-your = facoltativo, ad es. un UUID tuo
settings-radio-mirror-radio-s-i-q =
    Specchia l'I/Q della radio rispetto al centro del panadapter. Disattivato di default, che è come è stata verificata una FLEX-6600.

    Provalo se l'audio in ricezione è incomprensibile sia in USB *sia* in LSB e non si decodifica nulla, mentre la waterfall appare del tutto convincente — è così che si presenta un flusso specchiato, ed è l'unico guasto senza altri sintomi (segnalazione #368). Se non è questo, attivarlo lo rende evidente anziché sottile. Ha effetto con Applica / riconnetti.
settings-radio-network-mtu = MTU di rete
settings-radio-largest-datagram-radio-may-send = Il datagramma più grande che la radio può inviare. 1450 è ciò che chiede lo stesso SmartSDR. Abbassalo su un percorso con MTU più piccolo — una VPN o un tunnel — dove i frammenti vanno persi e non arriva affatto alcuno spettro.
settings-radio-b = {" "}B
settings-radio-checks-radio-answers-without-registering = Verifica che la radio risponda, senza registrarsi come client grafico — quindi non disturba una sessione di SmartSDR già in corso.
settings-radio-wideband-iq-receive-over-dax = I/Q a banda larga in ricezione tramite DAX, audio in trasmissione che la radio modula. Premi "Applica / riconnetti" per cambiare senza riavviare.
settings-radio-not-yet-verified-against-real-2 = Non ancora verificato su hardware reale. Se si comporta male, invia il rapporto diagnostico — contiene ogni riga di protocollo scambiata con la radio.
settings-radio-devices-soapysdr-can-see = Dispositivi che SoapySDR può vedere
settings-radio-waiting-machine-radio-attached-where = In attesa della macchina a cui è collegata la radio, dove sono installati i moduli. Quale dispositivo viene aperto è `device_args` nel config.toml di quella macchina, oppure il suo --device.
settings-radio-ask-every-installed-soapysdr-module = Chiedi a ogni modulo SoapySDR installato di cercare. Non si apre nulla, quindi è sicuro durante la ricezione — ma può richiedere un momento.
settings-radio-not-enumerated-yet-press-rescan = Non ancora enumerato — premi Riesamina.
settings-radio-no-soapysdr-devices-found-check = Nessun dispositivo SoapySDR trovato. Verifica che il modulo per la tua radio sia installato e che tu possa accedere al dispositivo.
settings-radio-not-radio = non è una radio
settings-radio-sound-card-listed-above-if = Una scheda audio è elencata sopra come se fosse un SDR. Accetta qualsiasi frequenza e la ignora, quindi aprirla mostra l'ingresso della scheda audio invece della banda. sdroxide non le sceglie automaticamente — ma una riga device_args che ne indica una viene comunque obbedita.
settings-radio-which-one-opens-set-device = Quale viene aperto è impostato da --device o da device_args nel config.toml; senza nessuno dei due vince la prima radio elencata.
settings-radio-no-device = nessun dispositivo
settings-radio-rig-has-no-software-adjustable = Questo apparato non ha guadagni regolabili via software.
settings-radio-rx-gains = Guadagni RX
settings-radio-tx-gains = Guadagni TX
settings-radio-antennas = Antenne
settings-radio-remembered-next-start = Ricordato per il prossimo avvio.
settings-radio-these-controls-appear-once-soapysdr = Questi comandi compaiono quando un dispositivo SoapySDR è aperto — sono quelli propri del dispositivo, e va interrogato.
settings-radio-stream = Flusso
settings-radio-follow-app-setting = Segui l'impostazione dell'app
settings-radio-msps = oppure, in Msps
settings-radio-baseband-filter = Filtro di banda base
settings-radio-let-driver-choose = Lascia scegliere al driver
settings-radio-changing-either-reopens-radio = Cambiarne uno riapre la radio.
settings-radio-driver-publishes-no-settings-own = Questo driver non pubblica impostazioni proprie.
settings-radio-applied-once-remembered-next-start = Applicato subito, e ricordato per il prossimo avvio.
settings-radio-no-rx-888-found-press = Nessun RX-888 trovato — premi Riesamina
settings-radio-adc-clock = Clock dell'ADC
settings-radio-t-129-6-msps-needs-superspeed = 129,6 Msps richiede un collegamento SuperSpeed e un host veloce; 64,8 è il predefinito sicuro. Cambiarlo riapre automaticamente il ricevitore, cosa che richiede un momento ma non un riavvio.
settings-radio-panadapter-width = Larghezza del panadapter
settings-radio-hf-only = {"  "}· solo HF
settings-radio-usable-both-front-ends = Utilizzabile su entrambi i front-end.
settings-radio-below-vhf-crossover-ordinary-width = Sotto il punto di incrocio VHF questa è una larghezza normale. Sopra di esso la IF da 8 MHz del sintonizzatore non può essere centrata in una finestra così larga, quindi lo spettro in tempo reale sta spostato da un lato del panadapter con nulla accanto.
settings-radio-dial-above-crossover-now-so = La sintonia ora è sopra l'incrocio, quindi questa larghezza mostra l'8 MHz del sintonizzatore fuori centro con spettro morto accanto. Restringi la larghezza per riempire il panadapter.
settings-radio-vga-gain = Guadagno VGA
settings-radio-ad8370-variable-gain-amplifier-ahead = Amplificatore a guadagno variabile AD8370 davanti all'ADC.
settings-radio-attenuator = Attenuatore
settings-radio-pe4304-step-attenuator-0-5 = Attenuatore a passi PE4304, a passi di 0,5 dB.
settings-radio-adc-range = Gamma dell'ADC
settings-radio-wide-2-25-vp-p = Ampia (2,25 Vp-p)
settings-radio-selects-adc-s-wider-input = Seleziona la gamma d'ingresso più ampia dell'ADC: più margine per i segnali forti di radiodiffusione, meno conteggi per quelli deboli. Disattivato seleziona la gamma più sensibile da 1,5 Vp-p.
settings-radio-dither = Dither
settings-radio-adds-small-dither-signal-ahead = Aggiunge un piccolo segnale di dither davanti all'ADC: costa un po' di rumore di fondo, e in cambio dà più gamma dinamica libera da spurie.
settings-radio-randomiser = Randomizzatore
settings-radio-adc-scrambles-output-so-digital = L'ADC mescola la sua uscita così il bus digitale smette di irradiare nel front-end; il driver la rimette in ordine. Lascialo attivo a meno che non stai facendo debug. Ha effetto alla riconnessione.
settings-radio-dc-hf-antenna-port = DC sulla porta d'antenna HF
settings-radio-powers-active-antenna-preamp-down = Alimenta verso il basso un'antenna attiva o un preamplificatore lungo il cavo coassiale.
settings-radio-vhf-tuner-gain = Guadagno del sintonizzatore VHF
settings-radio-r828d-rf-gain-used-above = Guadagno RF dell'R828D, usato sopra l'incrocio automatico HF/VHF. 29 passi discreti; si usa il più vicino.
settings-radio-auto = Automatico
settings-radio-let-tuner-run-own-lna = Lascia che il sintonizzatore faccia girare i propri anelli LNA e mixer.
settings-radio-bias-tee-vhf = Bias tee (VHF)
settings-radio-dc-vhf-antenna-port = DC sulla porta d'antenna VHF
settings-radio-clock-trim = Correzione del clock
settings-radio-corrects-reference-oscillator-applied-when = Corregge l'oscillatore di riferimento. Applicato quando rilasci il valore — riaprire la radio a ogni pixel di un trascinamento la riavvierebbe centinaia di volte.
settings-radio-receive-only-below-adc-s = Solo ricezione. Sotto il limite di Nyquist dell'ADC l'antenna viene campionata direttamente e la risintonia è istantanea, perché non c'è un convertitore hardware di discesa — tutto il flusso dell'ADC viene convertito in banda base dall'host. Sopra di esso il ricevitore passa automaticamente al suo sintonizzatore R828D e alla SMA VHF, quindi entrambe le antenne devono essere collegate. Il VHF richiede un clock ADC di 32,4 Msps o più perché la IF del sintonizzatore possa stare dentro, e a clock sotto 48 Msps resta un vuoto tra le due gamme che nulla può raggiungere. Ogni impostazione qui ha effetto subito — non c'è un pulsante Applica da premere.
settings-radio-device = Dispositivo
settings-radio-no-devices-press-rescan = nessun dispositivo — premi Riesamina
settings-radio-six-rates-are-six-different =
    Le sei frequenze sono sei immagini FPGA diverse, non un registro — ed è per questo che nulla nel protocollo di ELAD seleziona tra di esse, e perché ciò che questa impostazione significa dipende dal modello.

    FDM-S1 / FDM-S2: un comando. Il loro FPGA viene caricato dal computer a ogni accensione, e sdroxide esegue il caricatore elad-firmware di ELAD per inserirvi l'immagine di questa frequenza. Senza quel caricatore installato un campionatore non invia alcun campione, e sdroxide lo dice sullo schermo.

    FDM-DUO: non un comando. La radio avvia il proprio FPGA e arriva alla frequenza in cui si è accesa o in cui l'ha lasciata FDM-SW2 l'ultima volta (192 kHz su una nuova), e questa impostazione dice soltanto come viene LETTO il flusso. Se sbagliata, il panadapter ha semplicemente la larghezza sbagliata, con ogni frequenza al suo interno scalata di conseguenza.

    La frequenza reale del flusso viene misurata un paio di secondi dopo l'avvio, e un'incoerenza viene segnalata sullo schermo. Ha effetto con Applica.
settings-radio-fdm-s-fpga-image-loaded = FDM-S: l'immagine FPGA caricata — FDM-DUO: come viene letto il flusso
settings-radio-receiver-s-input-pad-one = L'attenuatore d'ingresso del ricevitore — un passo, in più o in meno. Lo stesso comando del cursore Guadagno della finestra principale.
settings-radio-pre-selection-filters = Filtri di preselezione
settings-radio-low-pass-bank-front-adc = Il banco di passa-basso davanti all'ADC. Escluderlo dà la vista più ampia e il comportamento peggiore vicino a segnali forti fuori banda — un trasmettitore di radiodiffusione a qualche miglio di distanza metterà immagini su tutta la banda. Lascialo attivo a meno che tu non stia deliberatamente ascoltando fuori dalla gamma filtrata.
settings-radio-filters-circuit = Filtri nel circuito
settings-radio-rig-control-fdm-duo-only = Controllo dell'apparato — solo FDM-DUO
settings-radio-transceiver-s-cat-usb-port = La porta USB CAT del ricetrasmettitore, che è un cavo separato da quello di ricezione. Lascia la porta non impostata su un FDM-S1 o FDM-S2, che non ne hanno — e su un FDM-DUO preferiresti guidare solo attraverso il suo cavo di ricezione, dove può ancora essere sintonizzato e manipolato ma non si può leggere alcun misuratore.
settings-radio-port-set-radio-s-display = Con la porta impostata, il display della radio e questa sintonia sono lo stesso numero in entrambe le direzioni: la finestra di ricezione dell'FDM-DUO è costruita attorno al suo VFO, quindi sintonizzare qui muove il pannello frontale e girare la manopola del pannello frontale muove la lettura qui. Il panadapter si ricentra sulla sintonia man mano che va. Senza porta — o con una porta a cui la radio non risponde — non c'è alcun VFO da comandare, e la sintonia resta dentro la finestra che la radio sta già inviando.
settings-radio-none-usb-control-only = — nessuna (solo controllo USB) —
settings-radio-must-match-menu-70-cat = Deve corrispondere al menu 70 "CAT BAUD" della radio, che di fabbrica è 38400. La radio ha queste quattro velocità e nessun'altra, e una porta aperta a qualsiasi altra è muta in entrambe le direzioni — la radio ignora la sintonia e rifiuta di attivarsi, senza nulla che spieghi perché.
settings-radio-how-transmit-keyed-cat-sends = Come viene attivata la trasmissione. "CAT" invia il comando TX proprio della radio e non richiede nulla da impostare sull'apparato; RTS richiede il menu 54 "PTT" impostato su PTT+RTS.
settings-radio-where-radio-takes-transmit-audio-2 =
    Da dove la radio prende l'audio di trasmissione — il comando TI dell'apparato, che è il menu 32 "TX IN" sul pannello frontale. Viene impostato all'apertura della porta.

    "Audio USB" è ciò che fa funzionare la trasmissione qui: l'FDM-DUO invia ciò che sdroxide mette nella sua scheda audio USB. Una radio lasciata su "Microfono" invia invece l'ambiente, e nulla sullo schermo lo segnala.
settings-radio-which-two-sockets-back-receiver-2 =
    Quale delle due prese sul retro ascolta il ricevitore — il comando AN dell'apparato, che è il menu 31 "ANTENNAS" sul pannello frontale e l'indicatore "ANT 1 2" sul suo display.

    "RTX" è un'antenna che fa entrambi i compiti, sulla presa di tipo M che porta anche la trasmissione. "Solo RX" sposta la ricezione sulla seconda presa e lascia la trasmissione su RTX — un'antenna di ricezione, un loop o una beverage, con la direttiva ancora sul trasmettitore.

    Sposta l'intero ricevitore: il panadapter, i demodulatori e l'audio proprio dell'apparato arrivano tutti dalla presa selezionata qui.

    Ha effetto subito, e viene riletto dalla radio all'apertura della porta di controllo — è l'impostazione dell'apparato stesso, non una copia conservata qui.
settings-radio-how-often-radio-asked-what-2 = Ogni quanto viene chiesto alla radio cosa sta facendo — la sua sintonia, il suo modo e i suoi misuratori. Gira la manopola VFO dell'apparato e la lettura segue entro un'interrogazione. È anche tutto il traffico di controllo che questa parte genera, quindi abbassalo se l'audio della radio si spezza.
settings-radio-transmit-audio = Audio di trasmissione
settings-radio-system-default = — predefinito di sistema —
settings-radio-fdm-duo-s-own-usb = La porta USB Audio propria dell'FDM-DUO. Lasciata sul predefinito di sistema, quasi mai è la radio — sono gli altoparlanti della macchina.
settings-radio-press-rescan-list-sound-cards = premi Riesamina per elencare le schede audio
settings-radio-copies-last-session-s-trace = Copia la traccia dell'ultima sessione negli appunti, per una segnalazione di bug: ogni comando scambiato con il dispositivo e i primi byte del flusso di campioni.
settings-radio-only-receiving-fdm-duo-has = Finora è stata eseguita su hardware reale solo la ricezione con un FDM-DUO. Se qualcosa si comporta male, allega il rapporto diagnostico a una segnalazione di bug.
settings-radio-cw-keyed-radio-s-own = Il CW viene manipolato dal tasto o dal paddle propri della radio: l'FDM-DUO non ha un comando che accetti testo, quindi il pannello CW non può manipolarlo.
settings-radio-press-apply-reopen-radio = Premi Applica per riaprire la radio.
settings-radio-no-receivers-press-rescan = nessun ricevitore — premi Riesamina
settings-radio-which-rates-receiver-has-depends = Quali frequenze ha un ricevitore dipende dal modello e dal firmware. Ha effetto con Applica.
settings-radio-every-rate-any-hf-model = ogni frequenza offerta da qualunque modello HF+ — collegane uno per vedere la sua
settings-radio-receiver-s-own-gain-control = Il controllo del guadagno proprio del ricevitore. Lascialo attivo per l'ascolto generico; disattivalo per impostare l'attenuatore a mano per le misure.
settings-radio-agc-threshold = Soglia AGC
settings-radio-high = Alta
settings-radio-high-trades-little-sensitivity-headroom = Alta scambia un po' di sensibilità per margine contro i vicini forti — l'impostazione giusta su una banda affollata di notte.
settings-radio-front-end-attenuation-gain-0 = Attenuazione del front-end, come guadagno — 0 dB significa nessuna. Obbedita solo con l'AGC disattivato.
settings-radio-preamp = Preamplificatore
settings-radio-hf-low-noise-amplifier-buys = L'amplificatore HF a basso rumore. Compra sensibilità al prezzo dell'intermodulazione, quindi è disattivato di default — il che di solito va bene su un'antenna vera.
settings-radio-lna = LNA attivo
settings-radio-frequency-calibration = Calibrazione della frequenza
settings-radio-parts-per-billion-receiver-s = Parti per miliardo — l'unità propria di questo ricevitore, mille volte più fine dei ppm usati da un RTL-SDR. Nulla qui viene mai scritto nella flash del ricevitore: questo sostituisce il valore memorizzato solo per la sessione.
settings-radio-use-receiver-s-stored-value = Usa il valore memorizzato nel ricevitore
settings-radio-ppb = {" "}ppb
settings-radio-feed-dc-up-coax = Invia corrente continua lungo il coassiale
settings-radio-not-every-hf-has-one = Non tutti gli HF+ ne hanno uno; su un ricevitore che non lo ha, non fa nulla.
settings-radio-host-dsp = DSP dell'host
settings-radio-image-balancer-zero-if-offset = Il bilanciatore d'immagine, l'offset zero-IF e l'oscillatore di sintonia fine. Disattivali solo per vedere l'uscita grezza dell'hardware — con essi disattivati, l'immagine speculare compare sulle frequenze zero-IF e la sintonia è precisa solo al kilohertz più vicino.
settings-radio-correct-image-fine-tune = Correggi l'immagine e la sintonia fine
settings-radio-copies-last-session-s-trace-2 = Copia la traccia dell'ultima sessione negli appunti, per una segnalazione di bug: ogni comando scambiato con il ricevitore e i primi byte del flusso di campioni.
settings-radio-receive-only-no-soapysdr-no = Solo ricezione. Non servono SoapySDR né libairspyhf. Sotto il limite del sintetizzatore è l'oscillatore dell'host a fare la sintonia, ed è così che questo ricevitore raggiunge le VLF. Il ricevitore e la frequenza di campionamento hanno effetto con Applica; tutto il resto ha effetto mentre lo modifichi.
settings-radio-not-yet-verified-against-real-3 = Non ancora verificato su hardware reale. Se si comporta male, invia il rapporto diagnostico — contiene ogni comando scambiato con il ricevitore e i primi byte del flusso di campioni decodificati come coppie I/Q.
settings-radio-no-airspy-r2-mini-found = Nessun Airspy R2 o Mini trovato — premi Riesamina
settings-radio-read-receiver = leggi dal ricevitore
settings-radio-r2-mini-offer-different-rates = un R2 e un Mini offrono frequenze diverse e non si possono distinguere finché uno non è aperto
settings-radio-rate-you-get-receiver-s = Questa è la frequenza che ottieni. L'ADC del ricevitore gira al doppio — digitalizza un segnale reale e sdroxide ne ricava banda base complessa sull'host.
settings-radio-gain-curve = Curva del guadagno
settings-radio-step-along-curve-above-not = Un passo lungo la curva qui sopra, non un valore in dB — LNA, mixer e VGA del sintonizzatore si muovono insieme, e quanto vale ogni passo dipende dalla curva e dalla banda. 0 è l'estremo silenzioso.
settings-radio-tuner-agc = AGC del sintonizzatore
settings-radio-lna-2 = LNA
settings-radio-hands-lna-tuner-s-own = Consegna l'LNA all'anello proprio del sintonizzatore, sostituendo la curva.
settings-radio-mixer = Mixer
settings-radio-same-mixer-stage = Lo stesso per lo stadio mixer.
settings-radio-loop-running-gain-slider-no = Con un anello attivo, il cursore del guadagno non imposta più lo stadio che possiede — l'anello lo sovrascrive.
settings-radio-dc-antenna-port = DC sulla porta d'antenna
settings-radio-t-12-bit-packing = Compattamento a 12 bit
settings-radio-third-less-usb-traffic-leave = Un terzo di traffico USB in meno. Lascialo attivo: è un dispositivo USB 2.0 e la frequenza massima è 30 MB/s compattata contro 40 non compattata. Ha effetto alla riconnessione.
settings-radio-dc-removal = Rimozione DC
settings-radio-remove-adc-s-offset = Rimuovi l'offset dell'ADC
settings-radio-turn-off-see-raw-hardware = Disattivalo per vedere l'uscita grezza dell'hardware. Vale la pena sapere dove finisce lo spurio: l'offset cade al bordo dell'ampiezza, non al centro, perché il segnale viene traslato di un quarto della frequenza di campionamento lungo il percorso.
settings-radio-every-command-exchanged-receiver-sample = Ogni comando scambiato con il ricevitore, l'aritmetica delle frequenze di campionamento, e i primi campioni decodificati come I/Q.
settings-radio-receive-only-24-1800-mhz = Solo ricezione, 24–1800 MHz. Non servono SoapySDR né libairspy. Il ricevitore, la frequenza di campionamento e il compattamento hanno effetto con Applica; tutto il resto ha effetto mentre lo modifichi.
settings-radio-not-yet-verified-against-real-4 = Non ancora verificato su hardware reale. Se si comporta male, invia il rapporto diagnostico — contiene ogni comando scambiato con il ricevitore, l'aritmetica delle frequenze e i primi campioni decodificati come coppie I/Q.
settings-radio-no-hydrasdr-rfone-found-press = Nessun HydraSDR RFOne trovato — premi Riesamina
settings-radio-these-are-rates-receiver-turned = queste sono le frequenze che questo ricevitore si è rivelato avere
settings-radio-receiver-only-reports-three-these = il ricevitore ne riporta solo tre; le altre sono nella tabella alternativa del firmware e una build più vecchia potrebbe non includerle
settings-radio-rf-input = Ingresso RF
settings-radio-antenna-sma-only-socket-bias = La SMA d'antenna — l'unica presa con il bias tee dietro di essa.
settings-radio-cable-socket-no-bias-tee = Una presa per cavo. Qui non c'è bias tee; l'hardware lo mette sulla sola ANT.
settings-radio-powers-active-antenna-preamp-down-2 = Alimenta verso il basso un'antenna attiva o un preamplificatore lungo il coassiale. Solo la presa ANT ce l'ha — le due porte per cavo sono ingressi semplici.
settings-radio-third-less-usb-traffic-leave-2 = Un terzo di traffico USB in meno. Lascialo attivo: è un dispositivo USB 2.0 e la frequenza massima è 36 MB/s compattata contro 48 non compattata. Ha effetto alla riconnessione.
settings-radio-one-these-boards-1d50-60a1 = Una di queste schede è su 1d50:60a1, l'ID USB che i prototipi di HydraSDR condividono con l'Airspy R2 e il Mini. sdroxide controlla il firmware dopo l'apertura e lo segnala se è stata scelta l'interfaccia sbagliata, in entrambi i sensi.
settings-radio-receive-only-24-1800-mhz-2 = Solo ricezione, 24–1800 MHz. Non servono SoapySDR né libhydrasdr. Il ricevitore, la frequenza di campionamento e il compattamento hanno effetto con Applica; tutto il resto ha effetto mentre lo modifichi.
settings-radio-re-list-fobos-sdrs-no = Rielenca gli SDR Fobos. Nessun dispositivo viene aperto, quindi è sicuro premerlo durante la ricezione.
settings-radio-no-fobos-sdr-found-press = Nessun SDR Fobos trovato — premi Riesamina
settings-radio-input-2 = Ingresso
settings-radio-through-tuner-lna-vga-mixer = Attraverso il sintonizzatore: LNA, VGA e l'intervallo di frequenza proprio del mixer.
settings-radio-direct-sampling-hf1-antenna-input = Campionamento diretto sull'ingresso d'antenna HF1 — nessun sintonizzatore, nessun guadagno LNA/VGA, sintonizzato interamente via software.
settings-radio-direct-sampling-hf2-antenna-input = Campionamento diretto sull'ingresso d'antenna HF2, come HF1.
settings-radio-both-real-adc-channels-once = Entrambi i canali ADC reali insieme, combinati dal filtro di diversità qui sotto — una sorgente di rumore annullata, o due percorsi con evanescenza combinati.
settings-radio-direct-sampling-tuner-so-lna = Campionamento diretto — il sintonizzatore (e quindi il guadagno LNA/VGA) non è nel percorso. La frequenza qui sotto è un obiettivo per il convertitore a discesa software, non un'impostazione hardware, quindi la frequenza effettivamente ottenuta può differire da essa.
settings-radio-receiver-s-own-reported-rates = le frequenze che questo ricevitore riporta da sé
settings-radio-measured-one-real-unit-connect = misurate su un'unità reale — collega questo ricevitore e premi Riesamina per vedere il suo elenco
settings-radio-clock-source = Sorgente di clock
settings-radio-external-reference = Riferimento esterno
settings-radio-diversity-taps = Prese di diversità
settings-radio-taps = {" "}prese
settings-radio-one-tap-gain-phase-null =Una presa è un guadagno e una fase — un nullo a una frequenza che peggiora da entrambi i lati, che è tutto ciò che un fasatore analogico può fare. Ogni presa in più compra un periodo di campionamento della differenza di percorso tra HF1 e HF2 che il filtro può equalizzare, ed è ciò che trasforma quel notch in una banda silenziosa per tutta la sua estensione.
settings-radio-receive-only-rf-port-tuner = Solo ricezione. Porta RF (sintonizzatore), HF1/HF2 (campionamento diretto, decodificato via software) e HF1+HF2 (entrambi insieme, combinati dal filtro di diversità) funzionano tutti. Il ricevitore, l'ingresso e la frequenza di campionamento hanno effetto con Applica; guadagno, sorgente di clock e prese hanno effetto mentre li modifichi — la modalità, la frequenza di adattamento e il blocco del filtro di diversità stanno nel box DIV della finestra principale una volta selezionata questa radio.
settings-radio-no-hackrf-found-press-rescan = Nessun HackRF trovato — premi Riesamina
settings-radio-front-end-amplifier-8-db = Amplificatore del front-end, a passi di 8 dB. È lo stadio che cambia la sensibilità — e lo stadio che va in sovraccarico per primo su un'antenna vera.
settings-radio-baseband-amplifier-after-mixer-2 = Amplificatore di banda base dopo il mixer, a passi di 2 dB. Alzalo per un segnale debole prima di ricorrere all'LNA.
settings-radio-rf-amp = Amplificatore RF
settings-radio-one-switch-shared-transmit-setting = Un solo interruttore, condiviso con l'impostazione di trasmissione qui sotto — la radio applica quello che appartiene alla direzione in cui sta entrando. Disattivato di solito va bene su un'antenna vera.
settings-radio-hackrf-pro-chooses-itself-three = Un HackRF Pro sceglie questo da sé — tre quarti della frequenza di campionamento, filtrata nell'FPGA — e ignora tutto ciò che l'host chiede.
settings-radio-leave-automatic-choosing-one-too = Lascialo su Automatico. Sceglierne uno troppo stretto non ammorbidisce soltanto i bordi della banda — ritira l'offset dell'oscillatore locale che tiene il picco DC lontano dal tuo segnale.
settings-radio-about-3-v-50-ma = Circa 3 V a 50 mA lungo il coassiale, per un'antenna attiva o un preamplificatore. Solo HackRF One o Pro; il Jawbreaker e il rad1o non hanno un simile circuito.
settings-radio-remove-dc-mirror-image = Rimuovi DC e l'immagine speculare
settings-radio-zero-if-radio-so-own = Questa è una radio zero-IF, quindi la propria fuga dell'oscillatore locale sta a metà ampiezza e l'errore di quadratura del mixer vi mette sopra un'immagine speculare. Disattivarlo mostra l'uscita grezza dell'hardware, che è il modo rapido per distinguere un problema del driver da uno del DSP.
settings-radio-corrects-reference-oscillator = Corregge l'oscillatore di riferimento.
settings-radio-transmit = Trasmissione
settings-radio-transmitter = Trasmettitore
settings-radio-off-default-while-off-backend = Disattivato di default. Finché è spento, il backend non pubblica alcun canale di trasmissione, quindi nulla può attivare la radio. Ha effetto alla riconnessione.
settings-radio-tx-vga-gain = Guadagno VGA in trasmissione
settings-radio-transmit-driver-amplifier-drive-applied = L'amplificatore pilota della trasmissione. Il pilotaggio viene applicato in digitale prima di questo stadio, quindi lascia il pilotaggio alto e imposta qui il livello di uscita — abbassare il pilotaggio fa invece lavorare il DAC a una frazione del fondo scala e aumenta l'intermodulazione.
settings-radio-same-switch-receive-setting-above = Lo stesso interruttore dell'impostazione di ricezione qui sopra, applicato quando la radio cambia direzione.
settings-radio-transmit-armed-into-dummy-load = La trasmissione è armata. Su un carico fittizio finché non l'hai misurata: le armoniche di un HackRF sono abbastanza forti da richiedere un filtro passa-basso esterno per la banda su cui sei, ed è semi-duplex, quindi la ricezione si ferma per la durata di ogni trasmissione.
settings-radio-every-command-exchanged-radio-session = Ogni comando scambiato con la radio in questa sessione, in ordine, compresa la sequenza attorno a ogni attivazione.
settings-radio-dc-7-25-ghz-half = DC – 7,25 GHz, semi-duplex. Non servono SoapySDR, libusb né libhackrf. La radio e la frequenza di campionamento hanno effetto con Applica; tutto il resto ha effetto mentre lo modifichi. I limiti della banda sono quelli del firmware: un HackRF è specificato per 1 MHz – 6 GHz (100 kHz su un Pro) ed è fortemente attenuato fuori di esso, ma lì si sintonizza.
settings-radio-ask-sdrplay-api-service-device = Chiedi al servizio SDRplay API l'elenco dei dispositivi. Nulla viene aperto, quindi è sicuro premerlo durante la ricezione.
settings-radio-no-rsps-press-rescan-needs = nessun RSP — premi Riesamina (richiede il servizio SDRplay API)
settings-radio-both-tuners-running-api-fixes = Con entrambi i sintonizzatori attivi, l'API fissa l'ADC a 6 MHz e restituisce 2 Msps da una IF bassa — quindi 2 Msps è l'ampiezza più larga, e quelle più strette sono questa ridotta. Ha effetto con Applica.
settings-radio-rates-below-2-msps-run = Le frequenze sotto 2 Msps fanno girare l'ADC a 2 Msps e riducono nel servizio. Ha effetto con Applica.
settings-radio-reduced-adc-resolution = {"  "}(risoluzione ADC ridotta)
settings-radio-if-bandwidth = Larghezza di banda IF
settings-radio-tuner-s-analog-filter-auto = Il filtro analogico del sintonizzatore. Automatico sceglie il più largo che entra nella frequenza di campionamento. Ha effetto con Applica.
settings-radio-rsp-s-own-if-gain = L'anello di guadagno IF proprio dell'RSP, eseguito dal servizio API. Disattivato restituisce il cursore del guadagno IF a te — l'impostazione per le misure e per i modi digitali a segnale debole.
settings-radio-agc-set-point = Punto di riferimento dell'AGC
settings-radio-signal-level-loop-holds-adc = Livello del segnale che l'anello mantiene sull'ADC. Più basso lascia più margine per i segnali fuori canale. Quanto può scendere dipende dalla frequenza di campionamento — sopra 8,064 Msps il convertitore ha meno margine da offrire, e sopra 9,216 Msps ancora meno.
settings-radio-rsp-s-native-gain-unit = L'unità di guadagno nativa dell'RSP: 20 dB è il guadagno massimo (0 con la gamma estesa qui sotto), 59 dB il minimo. Ha effetto subito. Ignorato mentre l'AGC è attivo — allora il valore è dell'anello, e lo S-meter mostra su cosa si è stabilizzato.
settings-radio-extended-if-range = Gamma IF estesa
settings-radio-lets-if-gain-reduction-go = Lascia che la riduzione del guadagno IF scenda sotto 20 dB, fino a 0 — gli ultimi 20 dB di guadagno che il ricevitore ha. Disattivato è il predefinito dell'API e quello giusto per l'ascolto normale, perché il fondo della gamma è dove un RSP è più facile da sovraccaricare. Vale la pena attivarlo per i segnali deboli con l'LNA già a 0. Permette anche al punto di riferimento dell'AGC di salire fino a 0 dBFS. Ha effetto subito.
settings-radio-allow-if-gain-reduction-below = Consenti la riduzione del guadagno IF sotto 20 dB
settings-radio-front-end-attenuation-steps-0 = Attenuazione del front-end a passi: 0 è il guadagno massimo, ogni passo in più inserisce più attenuazione. Alcune bande hanno meno passi — il driver li limita e conserva la tua scelta per quando risintonizzi, e il cursore qui finisce dove arriva la banda su cui è la radio. Una scelta più alta fatta su un'altra banda viene conservata e mostrata accanto. Ha effetto subito.
settings-radio-t-0-max-gain = 0 = guadagno massimo
settings-radio-reference-error-parts-per-million-2 = Errore di riferimento in parti per milione, applicato dal dispositivo stesso. Ha effetto subito.
settings-radio-radio-s-tuner = Il sintonizzatore di questa radio
settings-radio-tuner = Sintonizzatore
settings-radio-which-rspduo-s-two-tuners = Quale dei due sintonizzatori dell'RSPduo ascolta questa radio. L'altro appartiene alla radio configurata per esso — dai a quest'ultima lo stesso ricevitore e l'altro sintonizzatore. Ha effetto con Applica.
settings-radio-which-rspduo-s-two-tuners-2 = Quale dei due sintonizzatori dell'RSPduo porta l'antenna che stai ascoltando. L'altro porta la seconda antenna. Ha effetto con Applica.
settings-radio-which-rspduo-s-two-tuners-3 = Quale dei due sintonizzatori dell'RSPduo far girare (uno alla volta). Ha effetto con Applica.
settings-radio-applies-immediately = Ha effetto subito.
settings-radio-fm-broadcast-notch = Notch della radiodiffusione FM
settings-radio-hardware-notch-over-88-108 = Notch hardware sulla banda di radiodiffusione 88–108 MHz, per quando un trasmettitore locale sovraccarica tutto il resto. Ha effetto subito.
settings-radio-dab-notch = Notch DAB
settings-radio-hardware-notch-over-165-230 = Notch hardware sulla banda DAB 165–230 MHz. Ha effetto subito.
settings-radio-hdr-mode = Modalità HDR
settings-radio-enable-below-2-mhz = Attiva sotto 2 MHz
settings-radio-rspdx-s-high-dynamic-range =
    Il percorso ad alta gamma dinamica dell'RSPdx per LF/MF. Ha effetto subito.

    Non funziona ancora: sull'unico RSPdx su cui è stato provato, attivarlo silenzia il ricevitore — a ogni centro, filtro, porta, guadagno e frequenza provati. Ciò di cui la modalità ha bisogno non è nella superficie documentata dell'API, e trovarlo è il prossimo lavoro di qualcuno. Lascialo disattivato a meno che tu non sia quello che lo sta cercando.

    Cosa si sa: non è una modalità che segue la sintonia. Il percorso ha un filtro analogico fisso, costruito solo ai centri elencati sotto il filtro, quindi sintonizzare altrove non potrebbe funzionare nemmeno una volta che il resto lo faccia.
settings-radio-hdr-filter = Filtro HDR
settings-radio-feed-4-7-v-dc = Invia circa 4,7 V DC lungo il coassiale
settings-radio-second-tuner = Secondo sintonizzatore
settings-radio-rspduo-two-whole-tuners-one = Un RSPduo sono due sintonizzatori interi su una sola scheda, con clock da un unico riferimento. Fai girare entrambi e sentono le loro ampiezze nello stesso istante dallo stesso clock — il che rende coerenti due antenne su di essi, con una fase relativa fissata dai cavi di alimentazione anziché dal caso, e permette di combinare la coppia. Oppure tienili separati: i sintonizzatori si sintonizzano indipendentemente, quindi l'altro può essere una seconda radio sulla sua banda.
settings-radio-run-both-tuners = Fai girare entrambi i sintonizzatori
settings-radio-puts-rspduo-api-s-dual = Mette l'RSPduo nella modalità a doppio sintonizzatore dell'API, dove l'ADC è fissato a 6 MHz e l'ampiezza più larga è 2 Msps. Ha effetto con Applica, perché la modalità viene scelta all'apertura del dispositivo.
settings-radio-used = Usato per
settings-radio-what-other-tuner-takes-effect = A cosa serve l'altro sintonizzatore. Ha effetto con Applica — come far girare entrambi, questo viene scelto all'apertura della scheda.
settings-radio-add-second-radio-settings-radio = Aggiungi una seconda radio (Impostazioni → Radio → +), dalle questo stesso ricevitore e l'altro sintonizzatore dell'RSPduo, e impostala a far girare anche entrambi i sintonizzatori — qualunque radio apra la scheda la mette in modalità a doppio sintonizzatore, quindi entrambe devono aspettarselo. Le due poi si sintonizzano indipendentemente: HF in una scheda e VHF nell'altra, da una sola scheda. Condividono un unico clock ADC, quindi girano entrambe alla frequenza di campionamento richiesta da qualunque radio abbia aperto la scheda, e nessuna delle due può trasmettere.
settings-radio-what-do = Cosa farne
settings-radio-lna-state-2 = Il suo stato LNA
settings-radio-second-tuner-s-own-front = L'attenuazione del front-end proprio del secondo sintonizzatore. Impostata perché entrambe le antenne mostrino circa lo stesso rumore di fondo: è la regolazione su cui poggia tutto il resto, perché combinare pesa i due rami per il loro rumore, e un secondo front-end portato in sovraccarico consegna al filtro una copia distorta dell'interferenza — che non si può sottrarre da una non distorta. Ha effetto subito.
settings-radio-if-gain-reduction-2 = La sua riduzione del guadagno IF
settings-radio-second-tuner-s-if-gain = Il guadagno IF del secondo sintonizzatore, nell'unità nativa dell'RSP: 20 dB è il guadagno massimo. Ignorato mentre l'AGC è attivo — e un guadagno fisso è ciò che vuole il filtro, quindi vale la pena spegnere l'AGC per un nullo che intendi mantenere.
settings-radio-filter-length = Lunghezza del filtro
settings-radio-one-tap-gain-phase-null-2 = Una presa è un guadagno e una fase — un nullo a una frequenza che peggiora da entrambi i lati, che è tutto ciò che un fasatore analogico può fare. Ogni presa in più compra un periodo di campionamento della differenza di percorso tra le due antenne che il filtro può equalizzare, ed è ciò che trasforma quel notch in una banda silenziosa per tutta la sua estensione.
settings-radio-adaptation-2 = Adattamento
settings-radio-slow-steady-left-converging-inside = Lento e costante a sinistra, converge in meno di un secondo e a destra si vede che cerca. Parti veloce per trovare il nullo, poi mantienilo.
settings-radio-stop-filter-moving-reach-moment = Ferma il movimento del filtro. Raggiungilo nel momento in cui compare un nullo: un filtro lasciato in adattamento si rimirerà verso qualunque cosa diventi più forte, che su una banda tranquilla è la stazione che stai ascoltando.
settings-radio-restart = Riavvia
settings-radio-zero-filter-find-null-again = Azzera il filtro e ritrova il nullo.
settings-radio-nothing-here-can-tell-wanted = Nulla qui può distinguere un segnale desiderato da uno indesiderato — il filtro sa solo ciò che le due antenne hanno in comune. In Cancella, la seconda antenna vuole sentire la sorgente di rumore e il meno possibile della banda, altrimenti cancellerà diligentemente anche la stazione. In Combina, entrambe vogliono sentire la stessa stazione. Quanto profondo è il nullo finisce nel log ogni pochi secondi.
settings-radio-not-yet-verified-against-rspduo = Non ancora verificato su un RSPduo: il funzionamento a doppio sintonizzatore qui è scritto a partire dall'API di SDRplay anziché misurato sull'hardware.
settings-radio-no-receiver-listed-check-against = Nessun ricevitore è elencato con cui verificarlo — premi Riesamina, oppure leggi nel log cosa si è davvero aperto.
settings-radio-receive-only-1-khz-2 = Solo ricezione, 1 kHz–2 GHz. Richiede il servizio API SDRplay del produttore (sdrplay.com/api) — gli RSP successivi all'RSP1 originale non hanno un protocollo aperto. Dispositivo, frequenza di campionamento, banda e sintonizzatore RSPduo hanno effetto con Applica; tutto il resto ha effetto mentre lo modifichi.
settings-radio-board = Scheda
settings-radio-first-one-found-2 = Il primo trovato
settings-radio-receive-chain = Catena di ricezione
settings-radio-both-chains-tune-together-they = Entrambe le catene si sintonizzano insieme — condividono un unico sintetizzatore — ma sono front-end separati su prese separate. Scegli quella su cui è la tua antenna, che è l'impostazione da raggiungere quando una catena ha avuto la modifica di adattamento HF e l'altra è di serie.
settings-radio-receive-gain = Guadagno in ricezione
settings-radio-one-combined-figure-which-limesuite = Un'unica cifra combinata, che LimeSuite distribuisce tra LNA, TIA e PGA da sé. Accetta decibel interi, quindi tutto ciò che è più fine viene troncato.
settings-radio-receive-port = Porta di ricezione
settings-radio-automatic-follows-frequency-lnal-low = Automatico segue la frequenza: LNAL bassa, LNAH alta — a meno che un LimeRFE non sia collegato più in basso, che è un cavo su una presa, e allora è LNAW a ogni frequenza. Indica la presa su cui è cablata la tua, se non è quella. A quale catena appartiene la presa è il selettore qui sopra.
settings-radio-t-0-follows-sample-rate-worth = 0 segue la frequenza di campionamento. Vale la pena lasciarlo lì: un filtro più stretto di un quarto dell'ampiezza ritira in silenzio l'offset dell'oscillatore locale zero-IF, e la fuga dell'LO torna sopra il segnale che stai ascoltando.
settings-radio-following-rate = seguendo la frequenza
settings-radio-corrections = Correzioni
settings-radio-host-iq-dc-correction = Correzione I/Q / DC dell'host
settings-radio-adaptive-image-dc-removal-side = Rimozione adattiva dell'immagine e del DC da questo lato, in aggiunta alla calibrazione propria del chip. Disattivarla è il modo con un clic per distinguere un problema del driver da uno del DSP.
settings-radio-calibrate-automatically = Calibra automaticamente
settings-radio-runs-chip-s-own-dc = Esegue la calibrazione propria del chip per offset DC e immagine quando la radio viene aperta, e di nuovo quando la sintonia si è assestata su una nuova banda o un'altra presa. Quei valori sono misurati a una frequenza e sono sbagliati altrove, che è ciò che di solito è una portante al centro dell'ampiezza. Costa circa un secondo ogni volta.
settings-radio-calibrate-now = Calibra ora
settings-radio-stalls-receiver-better-part-second = Blocca il ricevitore per gran parte di un secondo.
settings-radio-second-aerial = Seconda antenna
settings-radio-socket = La sua presa
settings-radio-same-first = Uguale alla prima
settings-radio-gain-2 = Il suo guadagno
settings-radio-set-low-coupled-sample-your = Impostalo BASSO. Il campione accoppiato del tuo stesso trasmettitore è un segnale forte, e una catena di retroazione portata in compressione misura male la curva dell'amplificatore — insegna alla correzione la propria distorsione. Parti dal fondo e usa l'attenuatore dell'accoppiatore.
settings-radio-set-so-both-aerials-show = Impostalo perché entrambe le antenne mostrino circa lo stesso rumore di fondo. È la regolazione su cui poggia tutto: combinare pesa i due rami per il loro rumore, e una seconda catena portata in compressione consegna al filtro una copia distorta dell'interferenza — che non si può sottrarre da una non distorta.
settings-radio-table-steps-2 = Passi della tabella
settings-radio-how-finely-correction-follows-amplifier = Quanto finemente la correzione segue la curva dell'amplificatore. Più passi seguono un ginocchio più netto, ma ciascuno va appreso dai campioni che vi sono finiti — e la parte alta di un istogramma di ampiezza vocale è rada. Trentadue bastano per la curva liscia che un amplificatore HF ha davvero. Cambiarlo fa ricominciare la correzione.
settings-radio-how-hard-each-block-feedback = Quanto forte ogni blocco di retroazione sposta la correzione. La curva di un amplificatore non cambia, quindi non c'è bisogno di fretta — il centro media via il rumore di più trasmissioni.
settings-radio-keep-correction-curve-learned-clean = Mantieni la correzione com'è. Una curva appresa su una trasmissione pulita vale la pena di essere tenuta, e l'amplificatore non sarà cambiato alla successiva.
settings-radio-forget-correction-learn-again = Dimentica la correzione e riapprendila.
settings-radio-directional-coupler-amplifier-s-output = Un accoppiatore direzionale sull'uscita dell'amplificatore entra in questa catena, e il trasmettitore confronta ciò che è tornato con ciò che intendeva inviare — poi invia l'inverso della differenza, così ciò che esce dall'amplificatore è diritto. Una ventina di decibel in meno di intermodulazione sulle QSO altrui, con l'amplificatore che mantiene la sua potenza. La correzione resta unitaria finché la retroazione non si allinea alla trasmissione, quindi un accoppiatore non collegato non costa nulla; e non può mai chiedere al convertitore più del fondo scala, quindi un percorso di retroazione che legge sciocchezze non può sovrapilotare nulla. Come sta andando finisce nel log mentre trasmetti.
settings-radio-transmit-not-armed-so-there = La trasmissione non è armata, quindi non c'è nulla qui da correggere.
settings-radio-off-interface-publishes-no-transmit = Con questo disattivato l'interfaccia non pubblica alcun canale di trasmissione, quindi nulla può attivare la radio.
settings-radio-transmit-gain = Guadagno in trasmissione
settings-radio-transmit-port = Porta di trasmissione
settings-radio-board-s-two-transmit-sockets = Le due prese di trasmissione della scheda sono due reti di adattamento diverse, non due jack sulla stessa: BAND1 (TX_1) copre da 30 MHz a 1,9 GHz e BAND2 (TX_2) è la porta dei 13 cm. Attivare una banda radioamatoriale sotto i 23 cm dalla BAND2 manda la trasmissione in una rete di adattamento che ne lascia passare quasi nulla — ogni impostazione risulta corretta, il pilotaggio è quello che hai impostato, e il misuratore di potenza resta a zero. Lascialo su Automatico e segue la sintonia.
settings-radio-limesdr-transmits-about-100-khz = Un LimeSDR trasmette da circa 100 kHz a 3,8 GHz senza filtraggio proprio. Usa un filtro passa-basso, un canale LimeRFE appropriato, oppure un carico fittizio.
settings-radio-limerfe-front-end = Front-end LimeRFE
settings-radio-connected = Collegato tramite
settings-radio-limerfe-s-own-micro-usb = La porta micro-USB propria del LimeRFE, non quella della radio. 9600 baud, fissi dal suo firmware.
settings-radio-receive-connector = Connettore di ricezione
settings-radio-transmit-connector = Connettore di trasmissione
settings-radio-follow-dial = Segui la sintonia
settings-radio-switch-filters-match-operating-frequency = Commuta i filtri in modo che corrispondano alla frequenza operativa, prima che compaia qualsiasi RF. Sintonizzarsi all'interno di una banda non invia nulla sul collegamento di controllo.
settings-radio-relays = Relè
settings-radio-receive-attenuator = Attenuatore in ricezione
settings-radio-notch-filter = Filtro notch
settings-radio-fan = Ventola
settings-radio-worth-having-any-sustained-transmitting = Vale la pena tenerla attiva per qualsiasi trasmissione prolungata.
settings-radio-automatic-board-receives-switched-transmit = Su Automatico la scheda riceve, e viene commutata in trasmissione al key-down e riportata in ricezione al key-up — con qualunque cablaggio. I suoi canali radioamatoriali hanno un filtro con un commutatore trasmissione/ricezione su ciascun lato, quindi una scheda a cui vengono chieste entrambe le cose insieme mette quel commutatore sul trasmettitore e smette di sentire qualsiasi cosa. Il canale dei 30 MHz è raggiungibile solo tramite J5, che è un unico connettore per entrambe le direzioni; sopra di esso, trasmettere da J4 tiene il percorso di ricezione lontano dal connettore che pilota l'amplificatore.
settings-radio-limesdr-support-has-not-been = Il supporto LimeSDR non è stato verificato su hardware. Questa è la traccia dell'ultima sessione, per una segnalazione di problema.
settings-radio-gains-filters-corrections-antenna-sockets = Guadagni, filtri, correzioni, le prese d'antenna e ogni comando LimeRFE — i suoi connettori, la banda, i relè, l'attenuatore, il notch e la ventola — hanno effetto subito. La scheda, la catena di ricezione, la frequenza di campionamento, l'armamento della trasmissione e la connessione del LimeRFE hanno effetto con Applica.

## Settings window, Radio tab: messages with values

settings-radio-fill-from = RIEMPI DA { $name }
settings-radio-decimation-auto = Automatico (più vicina { $khz } kHz)
settings-radio-decimation-stage-server = Stadio { $stage } — la frequenza del server ÷ { $divisor }
settings-radio-decimation-stage = Stadio { $stage } — frequenza ÷ { $divisor }
settings-radio-kiwi-receive-only = Solo ricezione — è l'antenna di qualcun altro. Il panadapter è il circa 12 kHz che il ricevitore invia come I/Q, e segue la sintonia; la vista di banda sopra di esso è la sua waterfall, e sintonizzarsi attraverso di essa risintonizza il ricevitore. Circa { $rate } kB/s mentre è collegato. Premi "Applica / riconnetti" per cambiare senza riavviare.
settings-radio-connected-detail = Collegato: { $detail }
settings-radio-failed = Errore: { $error }
settings-radio-pluto-buffer = ≈ { $ms } ms, { $kib } KiB per trasferimento
settings-radio-driver-settings = Impostazioni di { $driver }
settings-radio-rx888-coverage = Copertura 0–{ $mhz } MHz, { $mbytes } MB/s via USB
settings-radio-rx888-digitised = dei { $mhz } MHz digitalizzati
settings-radio-elad-16-bit = { $khz } kHz (campioni a 16 bit)
settings-radio-elad-pad = Attenuatore d'ingresso da { $db } dB
settings-radio-hackrf-rx-preamp = Preamplificatore da { $db } dB in ricezione
settings-radio-hackrf-tx-preamp = Preamplificatore da { $db } dB in trasmissione
settings-radio-device-not-found = { $device } (non trovato)
settings-radio-lime-chain = Catena { $n } (RX{ $n }_*)
settings-radio-lime-chain-tx = Catena { $n } (RX{ $n }_* / TX{ $n }_*)
settings-radio-lime-second-chain = L'altra catena di ricezione della scheda, sulle prese { $sockets }. Condivide il sintetizzatore, quindi sente la stessa ampiezza nello stesso istante della prima — che è ciò che le permette di portare una seconda antenna, oppure un campione del tuo stesso trasmettitore.
settings-radio-tx-blocked = La trasmissione è bloccata: { $reason }
settings-radio-rfe-rx-fallback = In ricezione su { $port }, per queste si ricade sul percorso a banda larga non filtrato: { $bands }.
settings-radio-rfe-tx-fallback = In trasmissione su { $port }, per queste si ricade sul percorso a banda larga — né amplificatore di banda né filtraggio: { $bands }.

## Top bar and control strip (src/app/top_bar.rs)

topbar-how-far-one-press-moves = Di quanto sposta la sintonia una pressione. Tocca per passare al passo successivo: 10 Hz, 100 Hz, 500 Hz, 1, 2,5, 5, 9, 10 e 25 kHz. Disattiva la riga in Impostazioni › Interfaccia.
topbar-volume-gain-agc-squelch-noise = Volume, guadagno, AGC, squelch, i comandi del rumore e la lettura RDS sulla radiodiffusione FM
topbar-receiver = RICEVITORE
topbar-vfo-b-split-rit-xit = VFO A/B, split e gli offset RIT/XIT
topbar-tuning = SINTONIA
topbar-diversity-filter-which-way-combines = Il filtro di diversità: in che modo combina le due antenne, quanto in fretta si adatta, e il blocco nella posizione attuale
topbar-diversity = DIVERSITÀ
topbar-second-receiver-s-frequency-mode = La frequenza, il modo, il filtro e il livello del secondo ricevitore
topbar-sub-receiver = SUB RICEVITORE
topbar-transceiver-s-own-aerial-socket = La presa d'antenna propria del ricetrasmettitore, e il suo interruttore di alimentazione
topbar-radio = RADIO
topbar-tune-voice-keyer-drive-mic = Accordo, il manipolatore vocale, e i livelli di pilotaggio e microfono
topbar-transmit = TRASMISSIONE
topbar-tune = {" "}TUNE{" "}
topbar-panadapter-two-layers-their-speeds = Il panadapter — i suoi due livelli, le loro velocità e il dettaglio, il peak hold — più il contrasto della waterfall, la dimensione della FFT e gli skimmer
topbar-display = DISPLAY
topbar-logbook-spots-awards-memories-scanner = Logbook, spot, premi, memorie, lo scanner, le impostazioni e il manuale
topbar-system = SISTEMA
topbar-linked = Collegato
topbar-not-linked = Non collegato
topbar-bar-trace = barra / traccia
topbar-needle-bar-trace = ago / barra / traccia
topbar-swap-vfos = Scambia i VFO
topbar-copy-b = Copia A in B
topbar-second-receiver-right-ear-tunes = Secondo ricevitore, nell'orecchio destro. Si sintonizza indipendentemente da A/B — i suoi comandi compaiono nel modulo SUB, e la sua banda passante sulla waterfall.
topbar-band-plan = , dal piano di banda
topbar-repeater-shift-following-band-plan = Spostamento ripetitore: segue il piano di banda, che per questa frequenza non ha spostamento — simplex
topbar-repeater-shift-transmit-above-below = Spostamento ripetitore: trasmetti sopra o sotto la sintonia, per lavorare un ripetitore. Ora simplex
topbar-repeater-shift = SPOSTAMENTO RIPETITORE
topbar-auto = AUTO
topbar-take-shift-band-plan-dial = Prende lo spostamento dal piano di banda man mano che la sintonia si muove. Interviene solo dentro una sotto-banda di uscita di ripetitore — altrove lascia la radio in simplex, così i canali di chiamata restano in simplex.
topbar-offset = Offset
topbar-repeater-tone-ctcss-dcs-goes = Tono del ripetitore: il CTCSS/DCS inviato sotto la voce, il burst a 1750 Hz, e lo squelch a tono in ricezione
topbar-transmit-tone = TONO IN TRASMISSIONE
topbar-set-here-sent-nfm-sub = impostato qui, inviato in NFM — la segnalazione subaudibile è propria di un canale FM
topbar-normal = NORMALE
topbar-invert = INVERTI
topbar-bit-order-dcs-encoded-here = L'ordine dei bit con cui qui viene codificato il DCS è trascritto dallo standard e non è mai stato verificato su un ripetitore — se non si apre, prova il CTCSS
topbar-t-1750-hz-burst = BURST 1750 HZ
topbar-every-over = OGNI TRASMISSIONE
topbar-open-burst-every-time-transmitter = Apre con il burst ogni volta che il trasmettitore viene attivato
topbar-send = INVIA
topbar-send-burst-now-keying-transmitter = Invia il burst ora — attivando il trasmettitore per la sua durata, se non è già attivo
topbar-nfm-only-burst-fm-repeater = Solo NFM: il burst è l'apriporta di un ripetitore FM
topbar-receive-tone-squelch = SQUELCH A TONO IN RICEZIONE
topbar-require-receive-what-station-transmits = Richiedi in ricezione ciò che questa stazione trasmette
topbar-hide-docked-band-selector-click = Nascondi il selettore di banda agganciato — clicca di nuovo per riportarlo
topbar-show-docked-band-selector = Mostra il selettore di banda agganciato
topbar-dock = AGGANCIA
topbar-keep-band-mode-selector-open = Tieni aperto il selettore di banda e modo accanto alla waterfall, invece di chiudere questo popup ogni volta
topbar-band-mode = BANDA E MODO
topbar-hide-band-chip-brings-back = Nascondi — il chip di banda lo riporta
topbar-undock = SGANCIA
topbar-return-selector-top-bar-popup = Riporta il selettore nel popup della barra superiore
topbar-dec-off = DEC off
topbar-vol = Vol
topbar-counted-steps-not-decibels-what =
    {""}

    Contato in passi, non in decibel: quanto valga un passo è affare del ricevitore, e su un SDRplay dipende anche dalla banda. Il verso giusto resta più guadagno.
topbar-gain = Guadagno
topbar-plutosdr-s-own-agc-setting =
    {""}

    L'AGC proprio del PlutoSDR imposta questo guadagno, quindi muovere il cursore non fa nulla. Imposta Impostazioni → Radio → AGC su Manuale per controllarlo da qui.
topbar-agc-hang-time-click-cycle = Tempo di tenuta dell'AGC — clicca per scorrere: Off / Lento / Medio / Veloce
topbar-radio-s-own-agc-click = L'AGC proprio della radio — clicca per scorrere: Lento / Medio / Veloce. Da qui non può essere disattivato.
topbar-man = Man
topbar-manual-audio-gain-used-while = Guadagno audio manuale, usato mentre l'AGC è disattivato. Parte dal livello che l'AGC stava mantenendo quando è stato disattivato.
topbar-radio-s-own-squelch-sent = Lo squelch proprio della radio, inviato sul collegamento di controllo. È il cancello attraverso cui passa davvero l'audio — quello software si chiuderebbe solo di più su ciò che l'apparato ha già lasciato passare.
topbar-max-gain = guadagno massimo
topbar-manual-gain = guadagno manuale
topbar-noise-reduction = riduzione del rumore
topbar-impulse-noise-blanker = Limitatore di rumore impulsivo
topbar-auto-notch-cancel-constant-tone = Notch automatico: cancella le componenti a tono costante (battimenti)
topbar-noise-reduction-off-click-switch = Riduzione del rumore, disattivata — clicca per attivarla, oppure scegli un motore nelle righe qui sotto
topbar-binaural-audio-not-while-sub = Audio binaurale — non mentre il sub ricevitore ha l'orecchio destro
topbar-binaural-audio-passband-spread-across = Audio binaurale: la banda passante è distribuita sulle due orecchie, così i segnali a tonalità diverse arrivano da direzioni diverse e quello che sintonizzi scorre da un lato all'altro. Clicca per tornare al mono
topbar-binaural-cw-spread-passband-across = CW binaurale: distribuisce la banda passante sulle due orecchie, così i segnali a tonalità diverse arrivano da direzioni diverse — un pile-up diventa più posti invece di una sola nota affollata, e sintonizzare una stazione la fa scorrere. Meglio con le cuffie
topbar-binaural-audio-spread-passband-across = Audio binaurale: distribuisce la banda passante sulle due orecchie. Nella voce il rumore si spande sull'intera immagine mentre la stazione resta al centro, il che è più facile da ascoltare per un'ora — a costo che la voce stessa risulti spanta. Meglio con le cuffie
topbar-mute = SILENZIO
topbar-record-audio-raw-i-q = Registra l'audio, l'I/Q grezzo, oppure entrambi
topbar-wfm-stereo-forced-off-click = Stereo WFM forzato disattivato — clicca per lo stereo automatico
topbar-wfm-stereo-pilot-locked-click = Stereo WFM: pilota agganciato. Clicca per forzare il mono
topbar-wfm-stereo-automatic-no-pilot = Stereo WFM: automatico, nessun pilota su questa stazione
topbar-rds-data-arriving-click-see = I dati RDS stanno arrivando — clicca per vederli
topbar-rds-no-data-station-click = RDS — nessun dato su questa stazione. Clicca comunque per la diagnostica del decodificatore
topbar-drm-decoding-click-details = DRM in decodifica — clicca per i dettagli
topbar-drm-click-decoder-s-state = DRM — clicca per lo stato del decodificatore
topbar-hd-radio-decoding-click-details = HD Radio in decodifica — clicca per i dettagli
topbar-hd-radio-click-decoder-s = HD Radio — clicca per lo stato del decodificatore
topbar-presets = PRESET
topbar-preset-right-click = Clic destro per modificare questo preset
topbar-preset-edit = MODIFICA PRESET
topbar-preset-label = Etichetta
topbar-preset-label-hint = Il testo del pulsante. Fino a 8 caratteri.
topbar-preset-lo = Inf
topbar-preset-hi = Sup
topbar-preset-use-current = Usa attuale
topbar-preset-use-current-hint = Prendi i bordi del filtro attuale del ricevitore
topbar-preset-save = Salva
topbar-preset-reset = Ripristina
topbar-preset-reset-hint = Ripristina il preset predefinito di questo modo
topbar-preset-cancel = Annulla
topbar-width-per-sideband = LARGHEZZA PER BANDA LATERALE
topbar-width = LARGHEZZA
topbar-passband-width-hertz-type-drag = Larghezza della banda passante in hertz — scrivila, oppure trascina. Clicca nel campo per inserire un valore esatto.
topbar-edges = BORDI
topbar-low-edge-hz-carrier = Bordo inferiore, in Hz dalla portante
topbar-high-edge-hz-carrier = Bordo superiore, in Hz dalla portante
topbar-noise-reduction-voice-off-click = Riduzione del rumore (voce), disattivata — clicca per scegliere un motore
topbar-record-audio = REGISTRA AUDIO
topbar-mp3 = ● MP3
topbar-what-you-hear-mp3-receiver = Ciò che senti, in MP3: il ricevitore su un canale e la tua trasmissione sull'altro. È la registrazione di un QSO.
topbar-mono = MONO
topbar-mixed-one-channel-click-two = Miscelato su un canale — clicca per due
topbar-two-channels-rx-left-tx = Due canali: RX a sinistra, TX a destra — clicca per un canale miscelato
topbar-quick-clip = CLIP RAPIDA
topbar-stop-after = FERMA DOPO
topbar-no-stop = nessun arresto
topbar-auto-record = REGISTRAZIONE AUTOMATICA
topbar-set-squelch-auto-record-needs = Imposta uno squelch — la registrazione automatica ne ha bisogno per riconoscere il silenzio
topbar-auto-record-needs-sdr-front = La registrazione automatica richiede un front-end SDR con squelch software
topbar-stop-following-squelch = Smetti di seguire lo squelch
topbar-waiting-signal = in attesa di un segnale
topbar-record-spectrum = REGISTRA SPETTRO
topbar-i-q-wav = ● WAV I/Q
topbar-i-q-wav-2 = WAV I/Q
topbar-raw-spectrum-receiver-delivering-stereo = Lo spettro grezzo che il ricevitore fornisce, come WAV stereo a 32 bit float (RF64 oltre 4 GB) che SDR#, SDRuno, HDSDR e SDRangel aprono — e che sdroxide stesso riproduce con --file. Pesante: 8 byte per campione.
topbar-radio-hands-over-demodulated-audio = Questa radio consegna audio demodulato, quindi non c'è alcun I/Q da registrare.
topbar-what-capture-costs-sample-rate = Quanto costa una cattura a questa frequenza di campionamento
topbar-engine = MOTORE
topbar-no-noise-reduction-decoders-never = Nessuna riduzione del rumore — i decodificatori non la vedevano comunque
topbar-strength = INTENSITÀ
topbar-tone = TONO
topbar-ctcss-dcs-no-sub-audible = CTCSS / DCS — nessun tono subaudibile su questo segnale
topbar-tone-squelch = SQUELCH A TONO
topbar-carrier-squelch-open-any-signal = Squelch di portante: si apre con qualsiasi segnale
topbar-require-tone-currently-being-received = Richiedi il tono attualmente ricevuto
topbar-ctcss = CTCSS
topbar-any-dcs = QUALSIASI DCS
topbar-open-any-dcs-coded-signal = Si apre con qualsiasi segnale codificato DCS. Quale dei 104 codici trasporti non si può leggere in modo affidabile qui, quindi non c'è nulla di più fine da scegliere
topbar-which-socket-back-radio-receiving =
    Quale presa sul retro la radio sta ricevendo — il suo comando ANT, la stessa impostazione del pulsante ANT sul pannello frontale. Clicca per passare alla successiva.

    La scelta viene ricordata per banda, e ripristinata la prossima volta che la sintonia entra in quella banda.
topbar-radio-s-separate-receiving-antenna =
    L'antenna di ricezione separata della radio, inserita nel percorso di ricezione o esclusa da esso — la sua impostazione RX ANT propria. L'antenna sulla presa principale resta in trasmissione in ogni caso.

    La radio stessa ricorda questo per banda, quindi sdroxide lo rilegge dopo ogni cambio di banda invece di ripristinare ciò che aveva visto per ultimo: cliccare qui è l'unica cosa che lo sposta.
topbar-radio-s-own-power-switch =
    L'interruttore di alimentazione proprio della radio, tramite il collegamento di controllo, ed è l'unico vero on/off del programma — non l'interruttore LINK di sdroxide, che chiude la parte di sdroxide e lascia la radio accesa.

    Perché ON raggiunga qualcosa, la parte di controllo della radio deve restare sveglia mentre è spenta: Network Control sulla LAN, oppure una porta CI-V ancora alimentata dalla rete su un apparato spento dal pannello frontale.
topbar-switch-radio = Accendi la radio
topbar-switch-radio-off-audio-meters = Spegni la radio. Audio e misuratori si fermano con essa.
topbar-what-second-aerial-click-swap =
    A cosa serve la seconda antenna — clicca per scambiare.

    CANCELLA la sottrae dalla prima, con il guadagno, la fase e il ritardo che fanno allineare su entrambe una sorgente di rumore locale: la forma DSP di un fasatore che cancella il rumore. COMBINA somma le due nella fase che rinforza, pesate da quanto bene ciascuna sente — ricezione in diversità, che riempie le evanescenze.
topbar-hold = BLOCCA
topbar-stop-filter-moving-reach-moment = Ferma il movimento del filtro. Raggiungilo nel momento in cui compare un nullo: un filtro lasciato in adattamento si rimirerà verso qualunque cosa diventi più forte, che su una banda tranquilla è la stazione che stai ascoltando.
topbar-restart = RIAVVIA
topbar-zero-filter-find-null-again = Azzera il filtro e ritrova il nullo.
topbar-adapt = Adatta
topbar-how-fast-filter-chases-slow = Quanto in fretta il filtro insegue: lento e costante a sinistra, converge in meno di un secondo e a destra si vede che cerca. Parti veloce per trovare il nullo, poi BLOCCALO.
topbar-where-sub-receiver-listens-shift = Dove ascolta il sub ricevitore. Maiusc-clic sulla waterfall, oppure trascina dentro la banda passante del sub, per spostarlo.
topbar-dial = ←SINTONIA
topbar-move-sub-receiver-main-dial = Sposta il sub ricevitore sulla sintonia principale
topbar-dial-2 = SINTONIA←
topbar-move-main-dial-sub-receiver = Sposta la sintonia principale sul sub ricevitore
topbar-filter = Filtro
topbar-sub-receiver-passband-edges-hz = Bordi della banda passante del sub ricevitore, in Hz
topbar-sub-receiver-level-plays-right = Livello del sub ricevitore (suona nell'orecchio destro)
topbar-ptt = {" "}PTT{" "}
topbar-voice-keyer-record-transmit-stored = Manipolatore vocale: registra e trasmetti i messaggi memorizzati
topbar-drive = Pilotaggio
topbar-tune-2 = Accordo
topbar-tx-audio-processing-microphone-click = TX Audio sta elaborando il microfono — clicca per aprirlo
topbar-click-open-tx-audio-gate = Clicca per aprire TX Audio: gate, equalizzatore, compressore e limitatore per il microfono
topbar-mic = Mic
topbar-cessb = CESSB
topbar-microphone-gain = Guadagno del microfono
topbar-deviation-how-far-mode-s =
    Deviazione: di quanto la trasmissione di questo modo fa oscillare una radio che la modula da sé. Un trasmettitore FM converte il livello audio in oscillazione di frequenza e non ha un ALC che lo fermi, quindi il fondo scala verso un ingresso dati impostato per la voce supera la deviazione — e suona del tutto normale a chi ascolta, e non viene decodificato da nessuno.

    Conservata per modo, così una deviazione impostata per il pacchetto a 1200 baud non finisce mai su FT8.
topbar-transmit-audio-how-hard-mode =
    Audio in trasmissione: quanto forte questo modo pilota il modulatore di una radio che modula ciò che le inviamo — un apparato CAT sulla sua scheda audio, un FLEX, un Icom sulla sua porta di rete. Abbassalo finché l'ALC dell'apparato a malapena si muove, e imposta la potenza sulla radio; un ALC che lavora su un modo digitale a inviluppo costante è ciò che produce gli splatter.

    Il pilotaggio non è questo comando: su queste radio il Pilotaggio raggiunge il registro di potenza dell'apparato e non tocca mai il suo audio.

    Conservato per modo — FT8, RTTY, PSK e MCW ciascuno mantiene il proprio.
topbar-tx-audio = Audio TX
topbar-cw-psk-rtty-skimmers-decode = Skimmer CW / PSK / RTTY — decodificano i segnali su tutta la banda e li segnano sulla waterfall
topbar-skimmers = SKIMMER
topbar-run-skimmer = Avvia questo skimmer
topbar-minimum-snr-decoded-signal-needs = SNR minimo che un segnale decodificato deve avere per essere segnalato
topbar-needs-wideband-iq-source = richiede una sorgente I/Q a banda larga
topbar-cw-decoder = DECODIFICATORE CW
topbar-how-many-signals-model-reads = Quanti segnali il modello legge contemporaneamente. Gli altri mantengono il loro marcatore ma non portano testo.
topbar-spots = SPOT
topbar-save-skimmer-s-spot-list = Salva l'elenco spot dello skimmer su un file
topbar-show-hide-full-band-waterfall = Mostra/nascondi la striscia della waterfall a banda piena sopra il panadapter — tutto ciò che questo ricevitore può vedere in una volta
topbar-spectrum-waterfall-either-layer-both = Spettro e waterfall — uno dei due livelli, entrambi, o nessuno. Acceso quando entrambi sono visibili.
topbar-spectrum = SPETTRO
topbar-show-spectrum = MOSTRA SPETTRO
topbar-draw-spectrum-line-across-top = Disegna la linea dello spettro in cima al panadapter. Disattivata, la waterfall prende tutta l'altezza.
topbar-peak-hold = PEAK HOLD
topbar-trace-highest-level-each-column = Traccia il livello più alto che ogni colonna ha raggiunto sulla linea in tempo reale, decadendo di nuovo — ciò che la banda ha fatto mentre guardavi altrove
topbar-draw-spectrum-receding-surface-instead = Disegna lo spettro come una superficie che si allontana anziché come una linea piatta: lo spettro più recente in primo piano, quelli precedenti che scorrono via da te. Gli ultimi due secondi della banda come un paesaggio — una portante che va e viene è una cresta anziché una linea che si agita. Griglia e peak hold appartengono alla linea piatta e su di essa non vengono disegnati.
topbar-how-quickly-spectrum-line-follows = Quanto velocemente la linea dello spettro segue la banda. Più lento media più fotogrammi l'uno nell'altro: una linea più stabile, e una portante debole che resta ferma abbastanza a lungo da leggerla. La waterfall non ne è toccata — quelle righe ricevono ogni fotogramma comunque.
topbar-waterfall = WATERFALL
topbar-show-waterfall = MOSTRA WATERFALL
topbar-draw-scrolling-waterfall-below-spectrum = Disegna la waterfall che scorre sotto lo spettro. Disattivata, la linea dello spettro prende tutta l'altezza.
topbar-how-fast-waterfall-scrolls-lines = Quanto velocemente scorre la waterfall, in righe al secondo: Lento 5, Medio 28, Veloce 56, Più veloce 112, Velocissimo 224. Il motore li scandisce da sé, quindi i due più veloci sono dettaglio reale anziché la stessa riga disegnata due volte — nei limiti in cui il ricevitore può alimentarli: una riga non può mostrare più di una trasformata, e un front-end stretto ne produce solo qualche decina al secondo. Costano storia, perché la waterfall conserva un numero fisso di righe — 73 secondi a Medio, 9 a Velocissimo.
topbar-decode-labels = ETICHETTE DI DECODIFICA
topbar-mark-every-decoded-station-waterfall = Segna sulla waterfall ogni stazione decodificata con il suo nominativo, alla frequenza su cui è stata sentita — FT8, FT4 e gli altri modi a slot. Un buon inizio ne mette trenta sull'ampiezza due volte al minuto, sopra le tracce che stai leggendo; disattivandole la waterfall è soltanto la banda. L'elenco delle decodifiche accanto ha comunque tutti i nominativi.
topbar-switch-3d-set = Attiva il 3D per impostare questo
topbar-how-3d-spectrum-drawn-only = Come viene disegnato lo spettro 3D. Cambia solo la forma — entrambe le resa mostrano gli stessi spettri, e nascondono entrambe ciò che sta dietro, quindi un segnale forte in primo piano sovrasta la banda che copre.
topbar-lines = LINEE
topbar-one-trace-per-remembered-spectrum = Una traccia per ogni spettro ricordato, nel colore della linea piatta, ciascuna che nasconde quelle dietro di sé. La forma senza i livelli, e la lettura da scegliere quando la forma è ciò che cerchi — costa un po' di più di quella piena anziché meno, perché disegna la stessa superficie e poi traccia ogni cresta sopra di essa.
topbar-solid = PIENO
topbar-filled-surface-coloured-waterfall-s = Una superficie piena colorata con la palette della waterfall, così il livello sta nel colore oltre che nell'altezza e le due metà del panadapter concordano su come appare un segnale forte. Cambia la palette in Impostazioni › Display.
topbar-how-fast-3d-spectrum-flows = Quanto velocemente lo spettro 3D scorre via da te, in righe al secondo: Lento 6, Medio 12, Veloce 24, Più veloce 48. La superficie ha una profondità fissa di righe, quindi è anche quanto tempo trattiene. Più lento significa memoria più lunga e una superficie che striscia; più veloce una più corta che si muove. La waterfall mantiene la propria velocità di scorrimento, più sotto.
topbar-how-many-columns-panadapter-waterfall = Quante colonne vengono usate per disegnare il panadapter e la sua waterfall. Auto legge il renderer di questa macchina e la dimensione del panadapter e sceglie il massimo che può reggere — uno schermo 4K ne vuole 4096. Ogni colonna è un byte in ogni fotogramma, quindi un client collegato a una stazione remota paga il dettaglio sul proprio collegamento: 4096 colonne a 60 fps sono circa un quarto di megabyte al secondo, il doppio della larghezza standard.
topbar-read-machine-screen-take-most = Legge questa macchina e questo schermo, prende il massimo che possono reggere, e li segue se cambiano
topbar-width-force-right-now-whatever = La larghezza in vigore in questo momento, qualunque cosa chieda la riga sopra
topbar-keep-floor-ceiling-set-best = Mantieni il pavimento/soffitto impostati per il miglior contrasto della waterfall — riassestati dopo un cambio di banda, dopo uno spostamento o uno zoom, e quando i livelli derivano. Attivalo per adattare subito; disattivalo per tenere i livelli dove li hai impostati.
topbar-keep-tuned-frequency-middle-panadapter = Mantieni la frequenza sintonizzata al centro del panadapter: la finestra scorre sotto la sintonia anziché far saltare l'immagine di un'intera ampiezza quando sintonizzi oltre il bordo. Attivalo per centrare subito; disattivalo per spostare e zoomare dove vuoi.
topbar-spectrum-floor-ceiling-fft-size = Pavimento / soffitto dello spettro e dimensione della FFT
topbar-level-drawn-bottom-spectrum-darkest = Il livello disegnato in fondo allo spettro e come colore più scuro della waterfall. Alzalo finché il rumore di fondo appena si scurisce. Con FIT attivo, sposta invece il livello di FIT, e FIT resta attivo.
topbar-ceil = soffitto{" "}
topbar-waterfall-mean-not-peak = La cascata come potenza media (Welch) invece del picco: rumore calmo, stessi segnali. Un punto CW più corto di una riga o una portante più stretta di una colonna appare più debole.
topbar-welch-time-constant = Per quanto tempo WELCH fa la media, per la cascata e la linea dello spettro: più lungo è più calmo e mostra un cambiamento più tardi.
topbar-waterfall-reassign = REASSIGN: la cascata come spettrogramma riassegnato (Auger e Flandrin 1995): la potenza di ogni cella viene spostata sulla frequenza in cui è centrata, così una portante o un tono è una linea netta invece di una banda sfumata. L'alternativa a WELCH; il rumore resta altrettanto calmo.
topbar-reassign-dim = Quanto più scuro REASSIGN disegna ciò che non ha spostato su una linea: il rumore e ciò che resta sotto la soglia. 0 lo disegna come WELCH; più alto lascia le linee su fondo scuro.
topbar-level-drawn-top-bring-down = Il livello disegnato in cima. Abbassalo finché il segnale più forte che ti interessa non raggiunge il colore pieno. Con FIT attivo, sposta invece il livello di FIT, e FIT resta attivo.
topbar-fit-shift = Di quanto è spostato il livello di FIT su questo cursore. FIT mantiene lo spostamento su ogni banda; riattivare FIT lo azzera.
topbar-fft-size = DIMENSIONE FFT
topbar-flip = CAPOVOLGI
topbar-scroll-waterfall-upwards-newest-row = Fai scorrere la waterfall verso l'alto — riga più recente in basso (V)
topbar-logbook-all-qsos-digital-manual = Logbook — tutti i QSO (digitali + manuali)
topbar-live-spots-dx-cluster-pota = Spot in tempo reale — DX cluster, POTA, SOTA, PSK Reporter
topbar-award-tracking-dxcc-was-waz = Monitoraggio premi — DXCC / WAS / WAZ / locatori
topbar-band-conditions-published-forecast-beside = Condizioni di banda — la previsione pubblicata accanto a ciò che è stato davvero sentito su ciascuna banda
topbar-satellite-qo-100-beacon-calibration = Satellite — la calibrazione con il radiofaro QO-100 è in corso
topbar-satellite-doppler-tracking-qo-100 = Satellite — inseguimento Doppler, e la calibrazione con il radiofaro QO-100
topbar-ism-band-devices-decoding-now = Dispositivi in banda ISM — in decodifica ora, che la finestra sia aperta o meno. Disattivalo con DECODIFICA dentro la finestra.
topbar-ism-band-devices-weather-sensors = Dispositivi in banda ISM — sensori meteo, contatori e domotica sentiti intorno a te
topbar-public-sdrs-internet-browse-kiwisdr = SDR pubblici su internet — sfoglia le directory di KiwiSDR e SpyServer e aprine uno come radio
topbar-winlink-radio-email = Posta radio Winlink
topbar-memory-channels = Canali di memoria
topbar-scanner-stopped-signal = Scanner — fermo su un segnale
topbar-scanner-running = Scanner — in funzione
topbar-scan-memory-channels-frequency-range = Scansiona canali di memoria o un intervallo di frequenze
topbar-hfdl-ground-network-decode-log = Rete di terra HFDL — il registro delle decodifiche e la mappa degli aerei, sotto la waterfall. Attiva il decodificatore con ASCOLTA dentro il pannello.
topbar-hfdl-ground-network-decoding-now = Rete di terra HFDL — in decodifica ora. Apri il pannello.
topbar-hfdl-ground-network-aircraft-shortwave = Rete di terra HFDL — il collegamento dati in onde corte per aerei, un canale d'ascolto alla volta
topbar-grid-tracker-worked-maidenhead-squares = Tracciatore di locatori — quadrati Maidenhead lavorati su una mappa, con ciò che viene sentito
topbar-amplifier-set-up-hardrock-under = Amplificatore — configura un Hardrock in Impostazioni → Amplificatore
topbar-settings-device-gains-antennas-audio = Impostazioni — guadagni del dispositivo, antenne, dispositivi audio
topbar-user-manual-f1 = Manuale utente (F1)
topbar-range-was-typed-settings-radio = Quell'intervallo è stato scritto in Impostazioni ▸ Radio ▸ Gamma RX, non riportato dalla radio. Allargalo, oppure svuota la casella per usare ciò che il dispositivo riferisce di sé.
topbar-what-device-reports-about-itself = È ciò che il dispositivo riferisce di sé. Se la radio copre davvero questa banda, indica il suo intervallo reale in Impostazioni ▸ Radio ▸ Gamma RX.
topbar-band = BANDA
topbar-recent = RECENTI
topbar-recent-empty =
    Si riempie dopo 3 s su una postazione
topbar-mode = MODO
topbar-digital = DIGITALE

## Top bar: the System box's window chips.

topbar-chip-log = LOG
topbar-chip-spots = SPOT
topbar-chip-awards = PREMI
topbar-chip-bands = BANDE
topbar-chip-sat = SAT
topbar-chip-ism = ISM
topbar-chip-public-sdr = SDR PUBBLICO
topbar-chip-mail = POSTA
topbar-chip-mem = MEM
topbar-chip-scan = SCAN
topbar-chip-hfdl = HFDL
topbar-chip-grid = LOCAT.
topbar-chip-amp = AMP
topbar-chip-settings = ⚙ OPZIONI
topbar-chip-help = ? AIUTO

## Top bar: the mode-defaults chip.

topbar-changed-agc = AGC
topbar-changed-squelch = squelch
topbar-changed-auto-notch = notch automatico
topbar-changed-stereo = stereo
topbar-changed-binaural = binaurale
# $mode is a mode name (USB, CW…); $changed lists what differs from its defaults.
topbar-mode-defaults =
    Torna ai valori predefiniti di { $mode }. Modificati rispetto ad essi: { $changed }.

    Clicca per ripristinarli. Tornano i valori propri di { $mode }, e ciò che hai impostato qui viene dimenticato.

## Settings window, Radio tab: longer messages with values

settings-radio-no-such-baud = la radio non ha un'impostazione a { $baud } baud — verrà usato { $default }
settings-radio-icom-scope-hint =
    Trasmette lo sweep dello scope proprio della radio sul collegamento CI-V e lo disegna come panadapter — l'unica immagine della banda che un apparato con audio demodulato può dare, dato che l'audio che invia è ciò che è già passato dal suo filtro.

    Richiede la radio impostata per questo: Velocità di baud CI-V USB a 115200 e Porta CI-V USB su "Scollega da {"["}REMOTE]" (entrambe in SET > Connettori > CI-V), e la velocità di baud qui sopra impostata di conseguenza — sotto { $min_baud } gli sweep non entrano nel collegamento e lo scope resta spento.

    Gli sweep condividono il bus USB interno della radio con la sua scheda audio. Se l'audio in ricezione comincia a interrompersi con lo scope attivo, questa casella è la prima cosa da provare a disattivare.
settings-radio-rx-tx-words = { $rx } / TX { $tx }
settings-radio-soapy-native = L'hardware qui sopra è supportato direttamente da sdroxide: { $names }. Selezionando quell'interfaccia qui sopra si ottengono le sue impostazioni proprie — stadi di guadagno, filtri e notch che SoapySDR non può esprimere — e non serve alcun modulo SoapySDR.
settings-radio-rx888-width-hint = Quanto dello spettro digitalizzato mostra il panadapter in una volta. L'intera catena DSP lavora a questa larghezza, quindi una larghezza maggiore costa proporzionalmente più CPU — 1/2 è l'intera banda nella waterfall, e una quantità seria di calcoli. Sopra il punto di incrocio VHF ({ $crossover } MHz) il panadapter non guarda l'antenna ma la IF del sintonizzatore, che è larga 8 MHz e fissa a { $if_centre } MHz — quindi nulla più largo di { $widest } può esservi centrato, e la larghezza in più è spettro morto accanto al segnale anziché altro spettro.
settings-radio-sdrplay-serial-missing = Il seriale { $serial } non è tra i ricevitori che il servizio SDRplay riporta. Scegline uno dei ricevitori elencati, oppure ricollega questo e premi Riesamina.
settings-radio-hdr-filter-hint =
    Il filtro analogico davanti al percorso HDR — un controllo diverso dalla larghezza di banda propria del ricevitore. Ogni impostazione è costruita su un insieme fisso di centri e non può fare nulla altrove; questa è costruita su { $centres }. Ha effetto subito.

    Il percorso in sé non funziona ancora — vedi l'interruttore qui sopra — quindi questo sceglie un filtro per un modo che al momento è muto.
settings-radio-rspduo-only = Solo un RSPduo ha un secondo sintonizzatore, e il ricevitore selezionato qui è un { $model }, quindi non farà nulla.
settings-radio-lime-tx-gain-low = Il guadagno in trasmissione è { $db } dB, al fondo della sua gamma — pochi microwatt dalla scheda, che risulteranno nulla su un misuratore di potenza qualunque cosa ci sia a valle. È il predefinito perché un trasmettitore armato non ti sorprenda; alzalo prima di attivare.

## Settings window, General: SWR

settings-general-swr-tune-limit = (accordo: { $limit }:1)

## Top bar and control strip: messages with values

# $reading is the meter's own hover text; $faces lists the faces a click cycles through.
topbar-smeter-hover =
    { $reading }

    Clicca per scorrere le facce del misuratore: { $faces }
topbar-step-down = Giù { $step }
topbar-step-up = Su { $step }
# $auto is topbar-band-plan or empty.
topbar-repeater-shift-hover = Spostamento ripetitore { $shift } — in ricezione su { $rx }, in trasmissione su { $tx } MHz{ $auto }
topbar-tone-and-burst = Trasmissione di { $tone } sotto la voce, e un burst di { $ms } ms a 1750 Hz all'inizio di ogni trasmissione
topbar-tone-under-voice = Trasmissione di { $tone } sotto la voce
topbar-burst-every-over = Un burst di { $ms } ms a 1750 Hz all'inizio di ogni trasmissione
topbar-match-tx = CORRISPONDE TX ({ $tone })
topbar-decimation-on = Decimazione del front-end: al ricevitore viene data la parte centrale 1/{ $n } dei { $msps } Msps che questa radio trasmette — { $span } — e il resto viene scartato prima che una parte di esso raggiunga il ricevitore.
topbar-decimation-off = Decimazione del front-end, disattivata: il ricevitore vede tutti i { $span } che questa radio trasmette.
topbar-decimation-hint =
    { $what }

    Un'ampiezza più stretta significa una risoluzione della waterfall più fine, un rumore di fondo più basso (3 dB per ogni dimezzamento) e meno CPU — al costo della banda ai due lati. La sintonia può comunque andare ovunque; la radio sposta il suo LO per seguirla.

    Clicca per scorrere: disattivata / 2 / 4 … { $max }.
topbar-rx-gain-hint = Guadagno RX del front-end ({ $stage }). Troppo satura l'ADC del ricevitore e spalma segnali spuri su tutta la banda; troppo poco e diventa sordo.
topbar-rx-gain-stages =
    {""}

    Questo apparato ha { $n } stadi di guadagno RX — gli altri sono in Impostazioni → Dispositivo.
topbar-passband-now =
    {""}

    La banda passante è ora a { $dbfs } dBFS.
# $level is topbar-passband-now or empty.
topbar-squelch-hint =
    Fa passare l'audio solo sopra questa potenza nella banda passante di ricezione, in dBFS. A sinistra è aperto.{ $level } Impostalo sopra il rumore e sotto il segnale.

    Un flusso che arriva con l'AGC proprio della radio già applicata — ad esempio la IF a 12 kHz di un Icom — si trova molto più in alto su questa scala rispetto alla banda base grezza di un SDR, ed è per questo che il cursore arriva al fondo scala.
topbar-nr-cycle-hint =
    Riduzione del rumore: { $strength } — { $engine }

    Clicca per scorrere l'intensità: Bassa / Media / Alta / Disattivata. Il motore è nelle righe qui sotto.
topbar-recording-both = Registrazione di { $audio } e { $iq }
topbar-recording-audio = Registrazione dell'audio in { $audio }
topbar-recording-iq = Registrazione dell'I/Q in { $iq }
topbar-auto-record-armed = Registrazione automatica armata: un file per trasmissione, chiuso dopo { $secs } s di silenzio
topbar-rds-hover = RDS: { $ps }. Clicca per i dati della stazione
topbar-drm-hover = DRM: { $service }. Clicca per i dettagli della trasmissione
topbar-drm-no-codec = DRM: { $summary } — agganciato, ma il suo audio { $codec } non può essere decodificato qui. Clicca per sapere cosa manca
topbar-drm-state = DRM: { $summary } — clicca per lo stato del decodificatore
topbar-hd-hover = HD Radio: { $station }. Clicca per i dettagli della trasmissione
topbar-hd-state = HD Radio: { $summary } — clicca per lo stato del decodificatore
topbar-binaural-preset = { $preset } in ciascun orecchio — da { $lo } a { $hi } Hz, entrambe le bande laterali
topbar-nr-hover =
    Riduzione del rumore: { $strength } — { $engine }

    Clicca per cambiare motore o intensità
topbar-clipping = Ritaglio { $span } — premi di nuovo per ricominciarlo
topbar-record-clip = Registra { $span } e fermati — un ritaglio da allegare a un rapporto di ricezione
topbar-starting-clip = avvio di un ritaglio da { $span }…
topbar-stop-recording-after = Ferma la registrazione MP3 dopo { $minutes } minuti
topbar-stops-in = si ferma tra { $time }
topbar-auto-record-hint = Registra ogni trasmissione in un proprio file, chiuso dopo { $secs } secondi di silenzio
topbar-recording-silence = registrazione · { $secs } s di silenzio (si chiude a { $hold })
topbar-tone-open = In ricezione { $tone }, cioè lo squelch a tono — audio aperto
topbar-tone-closed = In ricezione { $heard }, ma lo squelch a tono vuole { $wanted } — l'audio resta chiuso
topbar-tone-heard = In ricezione CTCSS/DCS { $tone }
topbar-tone-nothing = Squelch a tono { $tone }: non viene ricevuto nulla che vi corrisponda
topbar-tone-use = USA { $tone }
topbar-sockets = Prese: { $sockets }
topbar-voice-keyer-tx = In trasmissione { $slot } — clicca per aprire il manipolatore vocale
topbar-drive-uncal-gen = Questa frequenza, che non appartiene ad alcuna banda radioamatoriale, non è calibrata, quindi l'impostazione raggiunge il trasmettitore per intero.
topbar-drive-uncal-band = { $band } non è calibrata, quindi l'impostazione raggiunge il trasmettitore per intero.
# $trim is signed (+1.5); $diff is its size; $more is "less" or "more".
topbar-drive-cal-gen = Questa frequenza, che non appartiene ad alcuna banda radioamatoriale, è calibrata { $trim } dB, quindi la stessa impostazione mette in aria { $diff } dB { $more ->
        [less] in meno
       *[more] in più
    } qui rispetto a una banda non calibrata.
topbar-drive-cal-band = { $band } è calibrata { $trim } dB, quindi la stessa impostazione mette in aria { $diff } dB { $more ->
        [less] in meno
       *[more] in più
    } qui rispetto a una banda non calibrata.
topbar-drive-hover =
    Quanto viene pilotato il trasmettitore. Su una radio che sdroxide modula da sé scala i campioni modulati; su un apparato con un proprio controllo di potenza è la frazione della potenza nominale che gli viene richiesta.

    { $trim }

    Impostazioni → Radio → Pilotaggio in trasmissione per banda.
topbar-cessb-hover =
    SSB a inviluppo controllato: più potenza media a parità di picco, senza splatter — tre o quattro decibel di volume percepito dall'altro capo. Di quanti decibel la voce viene spinta nel processore; 0 è disattivato, 6 è un buon primo tentativo. Non tocca il Pilotaggio.

    Ora: { $now }
topbar-waterfall-rate = { $rows } righe al secondo — { $seconds } secondi di banda
topbar-detail-auto-columns = { $n } colonne, qualunque cosa Auto avrebbe scelto qui
topbar-n-columns = { $n } colonne
topbar-fft-bins-whole = { $n } bin su tutto il flusso — uno per colonna, o più fini
topbar-fft-bins-pooled = { $n } bin raccolti nelle { $cols } colonne del panadapter: segnali più nitidi, non più segnali
topbar-sat-locked = Satellite — agganciato su { $name }
topbar-amp-in-line = { $model } — in linea (PTT)
topbar-amp-bypassed = { $model } — escluso (OFF)
topbar-amp-not-answering = { $model } — non risponde
topbar-filter-edges = Da { $lo } a { $hi } Hz dalla portante
topbar-filter-isb = Filtro di ricezione: { $width } Hz per ciascun orecchio ({ $edges }). Le due bande laterali dell'ISB sono trasmissioni separate, quindi la larghezza è quella di una sola di esse.
topbar-filter-width = Filtro di ricezione: { $width } Hz di larghezza ({ $edges }).
topbar-filter-hint =
    { $what }

    Clicca per la larghezza, i due bordi e i preset di questo modo, in numeri — le maniglie del panadapter piazzano una banda passante a occhio, qui si scrive un valore esatto.
topbar-band-outside-range =
    { $band } è fuori da ciò che questa radio riceve ({ $ranges } MHz).

    { $why }
# $time is "day" or "night".
topbar-band-forecast = { $band }: { $verdict } ({ $time ->
        [day] di giorno
       *[night] di notte
    }) — previsione di HAMQSL.com dagli indici solari, non una misura del tuo percorso.

## Top bar: short words and captions

topbar-squelch-open = aperto
topbar-off = off
topbar-recording = registrazione
topbar-sql = sql
topbar-stations = stazioni
topbar-caption-reaction = reazione
topbar-caption-flow = flusso
topbar-caption-detail = dettaglio
topbar-floor = pavimento

## Digital-mode panel tabs (src/app/panels/mod.rs, pane_label).

pane-aircraft = AEREI
pane-chart = GRAFICO
pane-chat = CHAT
pane-decodes = DECODIFICHE
pane-files = FILE
pane-heard = SENTITI
pane-image = IMMAGINE
pane-map = MAPPA
pane-messages = MESSAGGI
pane-monitor = MONITOR
pane-panel = PANNELLO
pane-qso = QSO
pane-reading = LETTURA
pane-receive = RICEZIONE
pane-saved = SALVATI
pane-send = INVIO
pane-spots = SPOT
pane-stations = STAZIONI
pane-status = STATO
pane-terminal = TERMINALE
pane-text = TESTO
pane-traffic = TRAFFICO
pane-vessels = NAVI
pane-wfall = WFALL

## Digital-mode panels, shared (src/app/panels/mod.rs)

panels-save = {" "}SALVA{" "}
panels-nothing-decoded-save = Niente da salvare: nulla è stato decodificato
panels-clear-rx = {" "}CANCELLA RX{" "}
panels-empty-receive-window-nothing-air = Svuota la finestra di ricezione. Nulla di ciò che è in aria si ferma.
panels-nothing-received-clear = Niente da cancellare: nulla è stato ricevuto
panels-save-what-panel-has-decoded = Salva su un file ciò che questo pannello ha decodificato
panels-prop = PROP
panels-shade-map-where-signals-are = Colora la mappa secondo dove i segnali stanno davvero arrivando, su ciascuna banda. Ogni ricezione è posta al punto medio del suo percorso — la porzione di ionosfera che lo ha piegato — non alla stazione lontana, quindi questa è una mappa del cielo anziché di dove vivono i radioamatori. È costruita dai percorsi a cui questa stazione ha preso parte, più — quando la Reverse Beacon Network è attiva in Impostazioni → Spot — ciò che gli skimmer del mondo stanno sentendo, che copre le bande su cui questa radio non è.
panels-night = NOTTE
panels-shade-where-sun-down-twilight = Colora dove il Sole è tramontato, e il crepuscolo tra i due, così la linea grigia compare sulla mappa. Le bande basse si allungano e quelle alte si chiudono dal lato notturno, e il terminatore stesso è dove sta il DX.
panels-all-bands = TUTTE LE BANDE
panels-every-band-once-one-hue = Ogni banda insieme, una tinta ciascuna — condizioni complessive.
panels-one-band = UNA BANDA
panels-one-band-cold-hot-blue = Una banda, da freddo a caldo: blu, verde, giallo, rosso.
panels-frequency-you-are-actually-working = La frequenza su cui stai davvero lavorando: la sintonia più l'offset di tono accanto. È ciò che va nel log — la sola sintonia risulta bassa di tanto.
panels-send-return = INVIA AL RITORNO
panels-freq = ⇵ FREQ
panels-nothing-agreed-mode-save-your = Nessuna frequenza concordata per questo modo — salva la tua qui sotto.
panels-forget-saved-frequency = Dimentica questa frequenza salvata
panels-dial-already-one-your-saved = Questa sintonia è già una delle tue frequenze salvate.
panels-dial-already-agreed-list-mode = Questa sintonia è già nell'elenco concordato per questo modo.
panels-decode-squelch-raise-stop-decoding = Squelch di decodifica — alzalo per smettere di decodificare il rumore
panels-message-repeats-long-transmit-held = Il messaggio si ripete finché la trasmissione è tenuta premuta — una meteora cattura la parte di esso che sta passando.
panels-w1abc-w9xyz-fn42 = W1ABC W9XYZ FN42
panels-tx = {"  "}TX ON{"  "}
panels-tx-2 = {"   "}TX{"   "}
panels-call-cq = {" "}CHIAMA CQ{" "}
panels-nocall = NOCALL

## TX Audio window (src/app/tx_audio.rs)

txaudio-heard-too-little-speech-talk = Sentito troppo poco parlato — parla normalmente per tutto il conteggio e riprova.
txaudio-tx-audio = TX Audio
txaudio-mode-not-voice-mode-so = Questo modo non è un modo vocale, quindi ciò che invia non passa per la catena. Le impostazioni fatte qui hanno effetto in SSB, AM e FM.
txaudio-bypass = ESCLUDI
txaudio-switch-every-stage-out-once = Escludi tutti gli stadi in una volta, compreso il riduttore di rumore, così il microfono esce non elaborato. Clicca di nuovo per ripristinare esattamente gli stadi che erano attivi — il confronto A/B più rapido di ciò che sta facendo la catena.
txaudio-denoise = RIDUCI RUMORE
txaudio-rnnoise-microphone-ahead-every-other = RNNoise sul microfono, prima di ogni altro stadio, così il rumore dell'ambiente viene rimosso prima che gate o compressore possano alzarlo. Aggiunge circa 10 ms di ritardo.
txaudio-digital-voice = VOCE DIGITALE
txaudio-also-run-freedv-rade-digital = Fai passare anche la voce digitale FreeDV / RADE nella catena. Disattivata di default: un vocoder è addestrato sul parlato naturale e potrebbe decodificare peggio una voce molto elaborata, quindi provalo in aria prima di affidarti ad esso. L'audio TCI e i modi digitali non passano mai per la catena.
txaudio-test = Prova
txaudio-rec = {" "}⏺ REC{" "}
txaudio-play = {" "}▶ RIPRODUCI{" "}
txaudio-play-test-phrase-through-chain = Riproduci la frase di prova attraverso la catena com'è impostata ora, sugli altoparlanti. Regola mentre suona e senti il cambiamento. L'audio in ricezione viene sostituito mentre suona.
txaudio-recording = registrazione…
txaudio-no-phrase-yet = nessuna frase ancora
txaudio-live = Dal vivo
txaudio-live-2 = {" "}🎧 DAL VIVO{" "}
txaudio-hear-your-microphone-through-chain = Senti il tuo microfono attraverso la catena mentre parli, senza attivare il trasmettitore. Usa le cuffie: attraverso gli altoparlanti si auto-alimenta. Ti senti con un ritardo di qualche decina di millisecondi, il che rende difficile parlare — regola i livelli sugli strumenti, e giudica il suono con REC / RIPRODUCI. L'audio in ricezione viene sostituito mentre questo è in funzione; premere PTT lo ferma.
txaudio-meters = {" "}STRUMENTI{" "}
txaudio-run-your-microphone-through-chain = Fai passare il microfono attraverso la catena solo per gli strumenti: niente sugli altoparlanti, quindi nessun ritardo su cui parlare e nulla da far riecheggiare. È il modo di impostare il guadagno del microfono e dell'ingresso, la soglia del gate e quanto lavora il compressore. L'audio in ricezione prosegue; premere PTT lo ferma.
txaudio-monitoring-live-use-headphones-speakers = Monitoraggio in diretta — usa le cuffie, altrimenti gli altoparlanti rientreranno nel microfono.
txaudio-voice-modes-only-tci-clients = Solo modi vocali. I client TCI e i modi digitali saltano la catena; quando è in uso, tieni piatti l'equalizzatore e il processore TX propri della radio.
txaudio-auto-set-stopped-microphone-was = Imposta automatico interrotto: il microfono è stato occupato.
txaudio-microphone-may-clipping-before-gets = {" "}Il microfono potrebbe andare in saturazione prima di arrivare qui — abbassa il suo guadagno proprio se suona aspro.
txaudio-level = Livello
txaudio-mic-control-same-one-transmit = Il comando Mic — lo stesso della barra di trasmissione, condiviso da ogni profilo. 0 dB al centro, +6 dB in cima.
txaudio-mic = Mic
txaudio-gain-top-mic-microphone-stored = Guadagno aggiunto a Mic per questo microfono, memorizzato con il profilo. Le soglie dei preset presuppongono una voce che raggiunge circa -10 dBFS sullo strumento IN; Imposta automatico la porta lì. Resta dentro sotto ESCLUDI, e vale anche per la voce digitale: è livello, non elaborazione.
txaudio-input = Ingresso
txaudio-auto-set = Imposta automatico
txaudio-auto-set-cancelled = Imposta automatico annullato.
txaudio-talk-now-you-would-air = Parla ora, come faresti in aria…
txaudio-db = -∞ dB
txaudio-chain = Catena
txaudio-order-audio-meets-them = In ordine, l'audio li incontra
txaudio-switch-stage = Attiva questo stadio
txaudio-earlier-chain = Più a monte nella catena
txaudio-later-chain = Più a valle nella catena
txaudio-output-limiter = Il limitatore d'uscita
txaudio-output = Uscita
txaudio-trim-limiter-always-last = Correzione e limitatore — sempre ultimi
txaudio-profiles = Profili…
txaudio-gate-expander = Gate / espansore
txaudio-pushes-room-noise-fan-rumble = Abbassa il rumore dell'ambiente, il ronzio della ventola e il sibilo nelle pause. Imposta la soglia appena sopra il rumore di fondo e sotto le sillabe più deboli.
txaudio-gate = GATE
txaudio-hard-gate-10-1-down = Gate netto: 10:1, fino a -40 dB
txaudio-expander = ESPANSORE
txaudio-gentle-expander-2-1-never = Espansore dolce: 2:1, mai oltre -15 dB — la stanza non diventa mai morta
txaudio-reset = AZZERA
txaudio-back-defaults = Torna ai valori predefiniti
txaudio-threshold = Soglia
txaudio-below-gate-starts-close = Sotto questo valore il gate inizia a chiudersi.
txaudio-return = Ritorno
txaudio-hysteresis-once-open-closes-only = Isteresi: una volta aperto, si chiude solo questo tanto sotto la soglia, così una voce che le sta intorno non fa vibrare il gate.
txaudio-hold = Tenuta
txaudio-how-long-stays-open-after = Quanto resta aperto dopo che la voce cala, così le finali delle parole non vengono tagliate.
txaudio-release = Rilascio
txaudio-how-fast-closes-once-hold = Quanto in fretta si chiude una volta finita la tenuta. Allungalo se il gate pompa.
txaudio-floor = Pavimento
txaudio-most-ever-turns-pauses-down = Il massimo di abbassamento che applica alle pause.
txaudio-ratio = Rapporto
txaudio-how-steeply-closes-below-threshold = Quanto ripidamente si chiude sotto la soglia.
txaudio-look-ahead = Anticipo
txaudio-opens-much-before-word-starts = Si apre questo tanto prima che inizi una parola, così la sua prima consonante non viene tagliata. Aggiunge lo stesso ritardo a tutto ciò che viene trasmesso.
txaudio-high-pass-low-shelf-two = Passa-alto, shelf dei bassi, due picchi, shelf degli alti e passa-basso. Trascina un punto sulla curva, oppure impostalo qui sotto. Imposta il passa-basso sulla larghezza del tuo filtro di trasmissione.
txaudio-flat = PIATTO
txaudio-every-band-back-0-db = Ogni banda riportata a 0 dB, entrambi i bordi disattivati
txaudio-freq = Freq
txaudio-gain = Guadagno
txaudio-q-slope = Q / Pendenza
txaudio-high-pass = Passa-alto
txaudio-removes-handling-noise-breath-pops = Elimina il rumore di manipolazione, gli scoppi del respiro e il ronzio sotto l'angolo.
txaudio-low-shelf = Shelf dei bassi
txaudio-body-mud-cut-little-here = Corpo, oppure fango: taglia un po' qui per una voce che suona scatolata.
txaudio-peak-1 = Picco 1
txaudio-bump-dip-voice-band-often = Una gobba o un avvallamento nella banda vocale — spesso un taglio intorno a 300–500 Hz.
txaudio-peak-2 = Picco 2
txaudio-presence-lift-around-2-3 = Presenza: un'esaltazione intorno a 2–3 kHz è ciò che emerge in SSB.
txaudio-high-shelf = Shelf degli alti
txaudio-brightness = Brillantezza.
txaudio-low-pass = Passa-basso
txaudio-stops-energy-above-transmit-filter = Impedisce che l'energia oltre il filtro di trasmissione venga spesa nel modulatore.
txaudio-t-24-db-oct = 24 dB/ott
txaudio-t-12-db-oct = 12 dB/ott
txaudio-click-steeper-gentler-slope = Clicca per una pendenza più ripida o più dolce
txaudio-de-esser = De-esser
txaudio-turns-down-only-sibilant-band = Abbassa solo la banda sibilante quando "s" e "sh" superano la soglia, così il resto della voce mantiene il suo livello. Trova la frequenza spazzandola mentre dici "sister silver cellophane" e fermati dove il sibilo è più forte.
txaudio-frequency = Frequenza
txaudio-centre-sibilant-band = Centro della banda sibilante.
txaudio-width-band-higher-narrower = Larghezza della banda — più alto è più stretta.
txaudio-band-level-where-reduction-starts = Livello della banda da cui inizia la riduzione. Troppo basso e lavora sempre, smorzando la voce.
txaudio-amount = Quantità
txaudio-most-band-ever-turned-down = Il massimo di abbassamento della banda. Parti intorno a -6 dB.
txaudio-attack = Attacco
txaudio-how-fast-reacts = Quanto velocemente reagisce.
txaudio-how-fast-lets-go = Quanto velocemente lascia andare.
txaudio-compressor = Compressore
txaudio-brings-quiet-syllables-up-holds = Alza le sillabe deboli e trattiene quelle forti. Punta a 3–6 dB di riduzione sui picchi; più di 10 dB è compressione evidente, più adatta alle gare.
txaudio-level-above-which-starts-compress = Livello oltre il quale inizia a comprimere.
txaudio-how-hard-compresses-above-threshold = Quanto comprime sopra la soglia.
txaudio-how-fast-reacts-peak-10 = Quanto velocemente reagisce a un picco. 10–25 ms mantiene le consonanti nitide.
txaudio-how-fast-recovers = Quanto velocemente si riprende.
txaudio-knee = Ginocchio
txaudio-soft-knee-width-0-hard = Larghezza del ginocchio morbido; 0 è un ginocchio netto.
txaudio-makeup = Recupero
txaudio-gain-after-compression-bring-level = Guadagno dopo la compressione, per riportare su il livello.
txaudio-drive = Pilotaggio
txaudio-gain-ahead-compressor-so-more = Guadagno prima del compressore, così più voce supera la soglia — alza la potenza media. Usalo insieme al limitatore d'uscita.
txaudio-phase = Fase
txaudio-all-pass-phase-rotator-ahead = Rotatore di fase passa-tutto prima del rivelatore. Livella i picchi sbilanciati che ha una voce, così lo stesso livello di picco porta più potenza media. 4 è un'impostazione comune in radiodiffusione; 0 è disattivato.
txaudio-last-stage-trim-look-ahead = L'ultimo stadio: una correzione, e un limitatore con anticipo che tiene ogni picco sotto il soffitto senza lo splatter che produce il ritaglio netto. Disattivato di default, così accendere la catena non cambia un livello di pilotaggio che hai già impostato.
txaudio-limiter = Limitatore
txaudio-on = Acceso
txaudio-ceiling = Soffitto
txaudio-no-peak-leaves-chain-above = Nessun picco esce dalla catena sopra questo valore.
txaudio-trim = Correzione
txaudio-output-gain-ahead-limiter = Guadagno d'uscita, prima del limitatore.
txaudio-off = {"  "}off
txaudio-meters-move-while-you-transmit = Gli strumenti si muovono mentre trasmetti una voce, riproduci la frase di prova, oppure usi DAL VIVO o STRUMENTI.
txaudio-profiles-2 = Profili
txaudio-profile-whole-chain-every-stage = Un profilo è l'intera catena: le impostazioni di ogni stadio, quali sono attivi, il loro ordine e il guadagno d'ingresso per il tuo microfono. Caricarne uno sostituisce la catena; il comando Mic e il pilotaggio restano come sono.
txaudio-name = Nome
txaudio-save = Salva
txaudio-store-chain-now-under-name = Memorizza la catena così com'è ora con questo nome, sostituendo un profilo con lo stesso nome
txaudio-import = Importa…
txaudio-read-profile-json-file = Leggi un profilo da un file .json
txaudio-tx-audio-profile = Profilo audio TX
txaudio-no-profiles-yet-save-stores = Nessun profilo ancora. Salva memorizza la catena che hai impostato ora.
txaudio-load = Carica
txaudio-export = Esporta…
txaudio-delete = Elimina
txaudio-starting-points = Punti di partenza
txaudio-set-your-level-auto-set = Imposta prima il tuo livello con Imposta automatico — le soglie dei preset lo presuppongono — poi regola da qui anziché trasmettere come caricato: registra una frase e ascolta dopo ogni modifica. Un preset mantiene il tuo guadagno d'ingresso.
txaudio-profile-loaded = Profilo caricato.

## Solar-system 3D view overlay (src/solar3d/overlay.rs)

solar-prop = PROP
solar-where-signals-are-actually-getting =
    Dove i segnali stanno davvero arrivando, banda per banda. Ogni ricezione a cui questa stazione prende parte — WSPR, FT8/FT4, JS8 e il logbook — è posta al punto medio del suo percorso, cioè la porzione di ionosfera che lo ha piegato, anziché alla stazione lontana. Quindi questa è una mappa del cielo, non di dove vivono i radioamatori.

    Può mostrare solo i percorsi di cui questa stazione è stata un capo. Gli oceani si illuminano perché lì cadono i punti medi dei percorsi lunghi; l'Antartide resta buia perché nessuno trasmette da lì. Disattivato di default — dipinge l'intero pianeta.
solar-orbits = ORBITE
solar-orbital-paths = Percorsi orbitali
solar-clouds = NUVOLE
solar-cloud-cover-noaa-s-global = La copertura nuvolosa dal mosaico geostazionario globale della NOAA, disegnata come una profondità d'aria anziché come un'immagine appiccicata sulla superficie: l'infrarosso dice quanto è fredda la sommità di ogni nube, e da lì si ricava quanto è alta. I temporali lampeggiano dentro le torri che costruiscono.
solar-planets = PIANETI
solar-other-seven-planets-eighteen-their = Gli altri sette pianeti, diciotto delle loro lune, e gli anelli di Saturno e Urano. Le posizioni dei pianeti vengono dal set di elementi kepleriani del JPL; le lune sono orbite circolari adattate ai dati JPL Horizons.
solar-coronal-mass-ejection-trajectory-cones = Coni di traiettoria delle espulsioni di massa coronale
solar-sun-obs = OSS. SOLE
solar-solar-observations-sun-s-disk = Osservazioni solari sul disco del Sole: regioni attive con macchie solari, e da dove provenivano i brillamenti
solar-labels = ETICHETTE
solar-body-region-labels = Etichette di corpi e regioni
solar-small-bodies = PICCOLI CORPI
solar-names-asteroids-comets-they-are = Nomi su asteroidi e comete. Vengono disegnati in ogni caso — questo decide solo se con loro vengono disegnate anche trentacinque designazioni. Qualunque sia l'impostazione, il corpo su cui è la camera e ciò che ha trovato la casella di ricerca restano con il nome.
solar-decoded-ft8-ft4-stations-path = Stazioni FT8/FT4 decodificate e il percorso verso la stazione con cui si lavora
solar-sats = SAT
solar-amateur-radio-satellites-their-orbits = Satelliti radioamatoriali, le loro orbite e l'elevazione dal tuo QTH
solar-aurora = AURORA
solar-auroral-oval-noaa-s-ovation = L'ovale aurorale dal modello OVATION della NOAA, disegnato come gusci di emissione alle loro altitudini reali, con il bordo verso l'equatore della curva del 10 % segnato sulla superficie
solar-awards = PREMI
solar-award-coverage-every-dxcc-entity = Copertura dei premi: ogni entità DXCC colorata da ciò che il tuo logbook ancora manca. L'arancio brucia dove non hai mai lavorato l'entità, l'ambra dove l'hai lavorata ma non è confermata, il verde spento dove è tornata una QSL. Segue il filtro di banda della finestra PREMI.
solar-view = VISTA
solar-which-body-camera-orbits-animated = Attorno a quale corpo orbita la camera, e il giro animato
solar-layers = LIVELLI
solar-what-drawn-orbits-clouds-cmes = Cosa viene disegnato: orbite, nuvole, CME, etichette, QSO, aurora
solar-which-sdo-channel-wraps-sun = Quale canale SDO avvolge il Sole, e quanto è vecchio
solar-weather = METEO
solar-aurora-propagation-numbers-which-bands = L'aurora, i numeri di propagazione e quali bande sono aperte
solar-space-weather = METEO SPAZIALE
solar-nothing-measured-yet = niente ancora misurato
solar-scale = SCALA
solar-body-moon-orbit-exaggeration = Esagerazione del corpo e dell'orbita lunare
solar-time = TEMPO
solar-scrub-whole-scene-forward-back = Scorri l'intera scena avanti e indietro
solar-activity = ATTIVITÀ
solar-replay-last-hour-decodes-globe = Riproduci l'ultima ora di decodifiche sul globo
solar-what-propagation-heat-map-showing = Cosa mostra la mappa termica della propagazione
solar-propagation = PROPAGAZIONE
solar-auto = ▶ AUTO
solar-fly-spline-through-set-framed = Vola lungo una spline attraverso un insieme di punti di vista inquadrati. Qualunque input del mouse la annulla.
solar-moons-ride-circular-orbits-fitted = Le lune seguono orbite circolari adattate ai dati JPL Horizons: entro uno o due gradi da dove si trovano davvero, e fino a sei per Miranda, il cui piano orbitale oscilla troppo in fretta perché un cerchio possa seguirlo.
solar-fetch-everything-again-now = Scarica di nuovo tutto ora
solar-all-sats = TUTTI I SAT
solar-show-every-satellite-element-set = Mostra ogni satellite dell'insieme di elementi, non solo quelli più noti. Gli anelli orbitali restano sui pochi selezionati — novanta insieme sono illeggibili.
solar-starting = avvio…
solar-body = corpo{" "}
solar-moon-orbit = orbita lunare{" "}
solar-stretch-earth-moon-distance-so = Allunga la distanza Terra→Luna così la coppia si può vedere separata
solar-sun = sole{" "}
solar-sun-radius-exaggeration-leave-1 = Esagerazione del raggio del Sole. Lascia a 1× per mantenere leggibile la geometria delle CME — un Sole gonfio inghiotte la base di ogni cono.
solar-show = MOSTRA
solar-paint-globe-what-getting-through = Colora il globo in base a ciò che sta passando
solar-paints-globe-what-getting-through = Colora il globo in base a ciò che sta passando, su ciascuna banda. Ogni ricezione è posta al punto medio del suo percorso — dove ha lavorato la ionosfera — anziché alla stazione lontana. È costruito da ciò che questa stazione sente, più gli skimmer del mondo quando la Reverse Beacon Network è attiva.
solar-display = DISPLAY
solar-all-bands = TUTTE LE BANDE
solar-every-band-once-one-hue = Ogni banda insieme, una tinta ciascuna, mescolando dove due si sovrappongono. La vista per "come sono le condizioni" anziché "è aperti i venti".
solar-one-band = UNA BANDA
solar-one-band-cold-hot-blue = Una banda, da freddo a caldo: blu, verde, giallo, rosso.
solar-band = BANDA
solar-nothing-heard-yet = niente di sentito ancora
solar-bands = BANDE
solar-sources = FONTI
solar-contacts-logbook-path-was-open = Contatti nel logbook. Un percorso che era aperto, senza un rapporto di segnale degno di questo nome — un RST non è un SNR, quindi questi contano per quanto è trafficata una cella e mai per il suo margine.
solar-stations-reported-hearing-one-downloaded = Stazioni che hanno riportato di sentire questa, scaricate da WSPRnet. L'unico riscontro che un radiofaro in trasmissione riceva mai.
solar-reverse-beacon-network-skimmers-what = Skimmer della Reverse Beacon Network: ciò che tutti gli altri stanno sentendo, su ogni banda, comprese quelle su cui questa radio non è. Più grossolani del resto — una riga RBN non porta locatori, quindi entrambi i capi vengono posti al centro del loro paese, il che è vicino per un paese piccolo e molto sbagliato per uno grande. Si attiva in Impostazioni → Spot.
solar-decodes-station-made-each-measured = Decodifiche fatte da questa stazione. Ognuna viene misurata rispetto al pavimento del proprio modo, così un rapporto WSPR e uno FT4 sono confrontabili.
solar-memory = MEMORIA
solar-half-life = emivita{" "}
solar-how-long-reception-takes-count = Quanto tempo deve passare perché una ricezione conti la metà. La memoria della ionosfera è breve — un'apertura di due ore fa non dovrebbe discutere con una di due minuti fa.
solar-live = DAL VIVO
solar-follow-band-happens = Segui la banda mentre accade
solar-replay = ⏸ RIPRODUCI
solar-replay-2 = ▶ RIPRODUCI
solar-replay-last-hour-decodes-over = Riproduci l'ultima ora di decodifiche, ancora e ancora
solar-min-ago = {" "}min fa
solar-where-last-hour-globe-showing = Dove nell'ultima ora sta mostrando il globo
solar-trail = scia{" "}
solar-how-long-decode-s-arc = Quanto a lungo l'arco di una decodifica resta sul globo dietro la testa
solar-speed = velocità{" "}
solar-how-much-faster-than-real = Quanto più veloce del tempo reale va la riproduzione
solar-qso-layer-off = Livello QSO disattivato
solar-no-decodes-yet = nessuna decodifica ancora
solar-qth-not-set-enter-your = QTH non impostato — inserisci il tuo quadrato locatore nelle Impostazioni
solar-mode = MODO
solar-path = PERCORSO
solar-sent = INVIATO
solar-rcvd = RICEV
solar-grid = LOCATORE
solar-dxcc = DXCC
solar-geostationary-always-above-your-horizon = Geostazionario — sempre sopra il tuo orizzonte.
solar-no-passes-next-48-hours = Nessun passaggio nelle prossime 48 ore dal tuo QTH.
solar-start = INIZIO
solar-max-el = EL MAX
solar-aos-los-are-azimuths-horizon = AOS/LOS sono azimut sull'orizzonte. Gli orari sono in UTC.
solar-locked-unlock = {" "}● BLOCCATO — SBLOCCA{" "}
solar-release-satellite-lock = Rilascia l'aggancio al satellite
solar-lock = {" "}BLOCCA SU{" "}
solar-track-satellite-main-window-doppler = Segui questo satellite nella finestra principale: RX e TX corretti per l'effetto Doppler, uplink mappato sul transponder, comando del rotore
solar-no-frequencies-file-one-add = Nessuna frequenza in archivio per questo — aggiungile in Impostazioni ▸ TLE.
solar-frequencies-yours = FREQUENZE · TUE
solar-frequencies = FREQUENZE
solar-link = COLLEGAMENTO
solar-downlink-mhz = DISCESA (MHz)
solar-uplink-mhz = SALITA (MHz)
solar-doppler-shifts-these-few-khz = L'effetto Doppler le sposta di qualche kHz durante un passaggio LEO.
solar-sim = -- SIM
solar-satellite-planet-comet = satellite, pianeta o cometa
solar-planet-asteroid-comet = pianeta, asteroide o cometa
solar-clear-search = Cancella la ricerca
solar-no-match = nessuna corrispondenza
solar-no-sounder = nessun sondatore
solar-heard = SENTITI ≥
solar-kp = Kp / A
solar-x-ray = Raggi X
solar-observed-above-sounder-better-than = osservato sopra il sondatore — migliore del modello
solar-power-n-s = potenza N/S
solar-edge-n-s = bordo N/S
solar-kp-peak-24-h = picco Kp 24 h
solar-observed-forecast = osservato | previsto
solar-bands-open = BANDE APERTE
solar-dxcc-coverage = COPERTURA DXCC
solar-ir-vis = IR+VIS
solar-ir-only = solo IR
solar-no-deep-convection = nessuna convezione profonda
solar-t-1-storm = 1 tempesta
solar-glancing = {" "}· radente
solar-direction-estimated = {" "}· direzione stimata

## Digital-mode setup dialogs (src/app/panels/setup.rs)

setup-aprs-setup = Configurazione APRS
setup-packet-setup = Configurazione pacchetto
setup-ft8-ft4-ft2-setup = Configurazione FT8 / FT4 / FT2
setup-my-callsign = Il mio nominativo
setup-my-grid = Il mio locatore
setup-station-call = Nominativo di stazione
setup-ssid-oe3jjs-10-packet-station = Con un SSID — OE3JJS-10. Una stazione a pacchetto usa per convenzione un SSID diverso dall'operatore, e non trasmette nulla finché questo non è impostato.
setup-speed = Velocità
setup-t-9600-needs-radio-s-data = 9600 richiede la porta dati della radio. Un percorso microfono e altoparlante lo distrugge a entrambe le estremità — è la radio, non sdroxide.
setup-tx-delay = Ritardo TX
setup-flags-sent-before-frame-so = Flag inviati prima di un frame, così l'altro capo può sentirci e agganciare il suo clock. Su un apparato CAT sdroxide da solo impiega 165–240 ms per andare in aria, e il tempo di prontezza in trasmissione dell'apparato si aggiunge a questo.
setup-packet-length = Lunghezza del pacchetto
setup-bytes = {" "}byte
setup-most-single-frame-carries-shorter = Il massimo che un singolo frame porta. I frame più corti sopravvivono a un percorso marginale — viene rispedito solo il frame colpito — ma costano più overhead. 128 in HF, 256 dove il percorso è solido.
setup-frames-sent-before-waiting-acknowledgement = Frame inviati prima di attendere una conferma. Di più riempie un buon percorso; su uno cattivo è più roba da rispedire.
setup-extended = Esteso
setup-ask-mod-128 = Richiedi mod-128
setup-extended-sequence-numbers-window-bigger = Numeri di sequenza estesi, per una finestra più grande di sette. Molti nodi rifiutano la richiesta con un DM, che sembra esattamente una stazione che non vuole parlarti — quindi lascialo disattivato a meno che tu non sappia che l'altro capo lo vuole.
setup-answer-calls = Rispondi alle chiamate
setup-accept-connections = Accetta connessioni
setup-off-winlink-client-which-dials = Disattivato per un client Winlink, che compone in uscita e non ha motivo di rispondere. Attivato per essere raggiungibile come peer: una chiamata arriva nel riquadro del terminale e parli con chi l'ha fatta. Una stazione il cui collegamento è già occupato da una sessione Winlink rifiuta le chiamate finché quella non finisce.
setup-connect-text = Testo di connessione
setup-what-caller-greeted = con cosa viene salutato chi chiama
setup-sent-station-connects-us-once = Inviato a una stazione che si collega a noi, una volta attivo il collegamento. Vuoto non invia nulla — il che sembra un guasto per chi ha chiamato.
setup-default-via = Predefinito via
setup-oe3xlr-1-oe3xms-1 = OE3XLR-1,OE3XMS-1
setup-digipeater-path-terminal-starts-path = Il percorso di digipeater con cui parte il terminale. Il percorso verso il tuo nodo locale è sempre lo stesso, e riscriverlo è il modo in cui si dimentica un salto.
setup-beacon = Radiofaro
setup-t-0-disables-timer = 0 disattiva il timer
setup-beacon-text = testo del radiofaro
setup-kiss-server = Server KISS
setup-serve = Servi
setup-offer-modem-kiss-tnc-socket = Offre questo modem come TNC KISS su un socket, così Pat, un client APRS o lo stack AX.25 di Linux possono usare la radio.
setup-aprs-call = Nominativo APRS
setup-callsign-station-beacons-under-ssid = Il nominativo con cui questa stazione invia il radiofaro, con il suo SSID — OE3JJS-9 per un'auto, -10 per un I-gate, -5 per un telefono. Lascialo vuoto per usare il nominativo qui sopra.
setup-no-callsign-station-will-not = Nessun nominativo: questa stazione non trasmette affatto — né radiofaro, né messaggio, né conferma.
setup-symbol = Simbolo
setup-two-characters-protocol-carries-them = I due caratteri come li porta il protocollo: una tabella (`/` o `\`) e un codice simbolo. Una cifra o una lettera in prima posizione è un overlay, disegnato sopra l'icona della tabella alternativa.
setup-path = Percorso
setup-wide1-1-wide2-1 = WIDE1-1,WIDE2-1
setup-how-far-you-ask-repeated = Fino a dove chiedi di essere ripetuto. È l'impostazione più importante del canale: ogni salto moltiplica le trasmissioni che l'intera rete fa su una frequenza condivisa.
setup-position = Posizione
setup-my-grid-2 = Dal mio locatore
setup-beacon-centre-locator-above-reported = Invia il radiofaro al centro del locatore qui sopra, riportato con l'ambiguità che un locatore ha davvero — uno a sei caratteri copre un paio di chilometri, e dirlo è onesto. Deseleziona per dare invece coordinate esatte.
setup-latitude = Latitudine
setup-longitude = Longitudine
setup-locator = dal locatore
setup-not-coordinate = non una coordinata
setup-comment = Commento
setup-sent-every-beacon = inviato con ogni radiofaro
setup-t-0-default-never-beacons-timer = 0 — il predefinito — non invia mai radiofari a intervalli. Trenta minuti è la convenzione per una stazione fissa; una stazione in movimento ne invia di più, ma ogni radiofaro è tempo di canale di qualcun altro.
setup-compressed = Compresso
setup-compressed-position-format-third-air = Il formato di posizione compresso: un terzo del tempo in aria e più preciso. Lo legge ogni ricevitore dagli anni '90.
setup-first-goes-out-one-interval = Il primo parte un intervallo da ora, non subito.
setup-messages = Messaggi
setup-acknowledge = Conferma
setup-answer-messages-addressed-you-acknowledgement = Rispondi ai messaggi indirizzati a te. Una conferma è una trasmissione che questa stazione fa senza che gliela si chieda, quindi una configurazione solo ricezione dovrebbe disattivarla — e il radiofaro smette così anche di dichiararsi raggiungibile.
setup-flags-sent-ahead-every-frame = Flag inviati davanti a ogni frame, così il modem dell'altro capo sente la portante e aggancia il suo clock prima che inizino i dati. Deve durare più di tutto ciò che sta tra la pressione di trasmissione e l'irradiazione: sdroxide da solo ne spende 165–240 ms su un apparato CAT, e una radio che riceve l'audio via rete ne accumula altri. Troppo poco e l'altro capo non si aggancia mai — la trasmissione c'è e nulla la decodifica.
setup-shared-packet-mode = condiviso con il modo pacchetto
setup-keep-stations = Mantieni le stazioni
setup-how-long-station-stays-map = Quanto una stazione resta sulla mappa dopo l'ultima volta che è stata sentita. È anche il metro della dissolvenza della mappa, così una finestra corta mostra solo ciò che è attivo.
setup-auto-reply = Risposta automatica
setup-answer-snr-grid-status = Rispondi a SNR? / GRID? / STATUS?
setup-answer-direct-question-addressed-you = Rispondi a una domanda diretta indirizzata a te o a @ALLCALL, con la risposta anziché con una conferma. Non risponde mai al traffico di un'altra stazione, e non risponde mai a se stessa.
setup-heartbeat = Heartbeat
setup-off = Disattivato
setup-turbo-does-not-beacon-local = Turbo non invia radiofari — è la velocità locale e VHF.
setup-sends-your-callsign-grid-so = Invia il tuo nominativo e il locatore, così gli altri sanno che sei ricevibile. Il primo parte un intervallo da ora, non subito.
setup-beacon-frequency = Frequenza del radiofaro
setup-t-500-1000-hz = 500–1000 Hz
setup-move-each-beacon-free-slot = Sposta ogni radiofaro in uno slot libero nella sotto-banda heartbeat, come fa JS8Call: è dove guardano le stazioni che cercano radiofari, e tiene un trasmettitore incustodito lontano dal QSO di qualcun altro. Lo slot viene scelto quando il radiofaro parte davvero, libero da tutto ciò che viene decodificato.
setup-working-freq = Frequenza di lavoro
setup-beacon-where-you-are-working = Invia il radiofaro dove stai lavorando. Contro la convenzione della banda, ma tiene tutto ciò che trasmetti in un solo posto.
setup-heartbeat-reply = Risposta heartbeat
setup-answer-heartbeats-signal-report = Rispondi agli heartbeat con un rapporto di segnale
setup-tell-station-beaconed-how-well = Dici a una stazione che ha inviato un radiofaro quanto bene l'hai ricevuta. Disattivato di default: una banda affollata porta un heartbeat in ogni slot, e rispondere a tutti inonderebbe proprio la banda che gli heartbeat servono a tenere quieta. Limitato a una risposta per stazione ogni 15 minuti, e mai mentre un messaggio sta ancora arrivando o mentre hai qualcosa in coda da inviare.
setup-status-message = Messaggio di stato
setup-tx-period = Periodo TX
setup-even = Pari
setup-odd = Dispari
setup-auto-sequence = Sequenza automatica
setup-decode-depth = Profondità di decodifica
setup-fast = Veloce
setup-normal = Normale
setup-deep = Profonda
setup-how-hard-ft8-decoder-works =
    Quanto lavora duramente il decodificatore FT8 sui segnali deboli.

    Veloce — un solo passaggio, nessuna sottrazione di segnali: il più rapido, con meno decodifiche.
    Normale — sottrazione multi-passaggio regolare: un po' più rapido di Profonda, un po' meno accurato.
    Profonda — il multi-passaggio con checkpoint: il maggior numero di decodifiche, e circa 1,2 s su uno slot affollato.
setup-auto-tx-frequency = Frequenza TX automatica
setup-choose-transmit-frequency-automatically-quietest = Scegli automaticamente la frequenza di trasmissione: il punto più tranquillo nel periodo in cui trasmetti, anziché la frequenza della stazione a cui rispondi. Disattivato NON la mantiene — risponde sulla frequenza della stazione chiamata. Per mantenerla, usa Mantieni TX.
setup-hold-tx-frequency = Mantieni frequenza TX
setup-pin-transmit-tone-where-nothing = Fissa il tono di trasmissione dove si trova. Nulla lo sposta: né rispondere a una stazione, né la coda delle chiamate, né chiamare CQ, né un clic su una decodifica o sulla waterfall. Prevale su Frequenza TX automatica. Cambiare banda è l'unica eccezione: l'offset impostato per ultimo sulla nuova banda torna con essa. Serve quando la tua licenza è più stretta del piano di banda — su un 60 m britannico con sintonia a 5357 kHz l'assegnazione finisce a 5358.0, quindi il tono deve restare sotto 1000 Hz.
setup-tx-watchdog = Sorveglianza TX
setup-stop-transmitting-after-long-no = Smetti di trasmettere dopo questo tempo senza risposta e senza alcuna tua azione. 0 la disattiva.
setup-give-up-after = Rinuncia dopo
setup-calls = {" "}chiamate
setup-unanswered-calls-one-station-before = Chiamate senza risposta a una stazione prima di passare oltre. Chiamare CQ è esente. 0 la disattiva.
setup-contest = Contest
setup-everyday-exchange-grid-report-rr73 = Lo scambio ordinario: locatore, rapporto, RR73.
setup-european-vhf-contests-send-signal = Contest VHF europei: invia un rapporto di segnale, un numero di serie e il tuo locatore a SEI caratteri, nel layout dei messaggi che WSJT-X chiama EU VHF Contest. CQ viene inviato come CQ TEST.
setup-serial = Seriale
setup-serial-number-next-exchange-will = Il numero di serie che porterà il prossimo scambio. Avanza da solo a ogni contatto registrato ed è ricordato tra le sessioni — impostalo a mano solo per iniziare un contest, o per riprendere da dove era arrivato un registro cartaceo.
setup-needs-6-character-grid = ⚠ richiede un locatore a 6 caratteri
setup-eu-vhf-contest-messages-carry = I messaggi dell'EU VHF Contest portano un locatore a sei caratteri (JN88DD, non JN88). Finché il mio locatore non ne ha uno, viene inviato invece lo scambio ordinario.
setup-dxpedition = DXpedition
setup-ordinary-ft8-operation = Operatività FT8 ordinaria.
setup-calling-dxpedition-running-fox-mode = Chiamare una DXpedition in modo Fox: chiama da sopra i 1000 Hz, scendi sulla Fox quando risponde, e registra sul suo RR73 senza inviare 73.
setup-run-pile-up-several-signals = Gestisci il pile-up: più segnali insieme, una coda di chiamanti, lavorati prima i più forti e i più rari. CHIAMA CQ lo avvia, FERMA QSO lo sospende.
setup-fox-signals = Segnali Fox
setup-once = {" "}insieme
setup-simultaneous-transmissions-spaced-60-hz = Trasmissioni simultanee, distanziate di 60 Hz. Condividono la potenza del trasmettitore, quindi più segnali significa che ciascuno è più debole.
setup-tx-audio = Audio TX
setup-how-loud-mode-s-burst =
    Quanto forte viene consegnato il burst di questo modo a una radio che lo modula da sé — e su FM questa è la deviazione, perché un trasmettitore FM converte il livello audio in oscillazione di frequenza e non ha un ALC che la fermi. Il pacchetto a 1200 baud vuole circa 3 kHz dove la voce ne vuole 5, quindi il fondo scala verso un ingresso dati impostato per la voce supera la deviazione — e suona del tutto normale a chi ascolta, e non viene decodificato da nessuno. Abbassalo finché altre stazioni non ti segnalano, oppure imposta il livello sulla radio.

    Questo livello appartiene solo a questo modo. Un modo che non hai mai impostato parte dal valore FM, e i modi a banda laterale partono dai propri — così una deviazione impostata per il 1200 baud non finisce mai su FT8.
setup-how-loud-mode-s-over =
    Quanto forte viene consegnata la trasmissione di questo modo a una radio che la modula da sé — un apparato CAT sulla sua scheda audio, un FLEX, un Icom sulla sua porta di rete. Sulla banda laterale questo è il pilotaggio verso il modulatore: abbassalo finché l'ALC dell'apparato a malapena si muove, e imposta la potenza sulla radio, perché un ALC che lavora su un modo a inviluppo costante è ciò che produce gli splatter. Il Pilotaggio qui raggiunge il registro di potenza dell'apparato, non il suo audio, quindi questo è il livello.

    Questo livello appartiene solo a questo modo: FT8, RTTY, PSK e MCW mantengono ciascuno il proprio, perché non caricano un modulatore allo stesso modo. Lo stesso cursore è sulla barra di trasmissione, dove lo raggiungi mentre trasmetti.
setup-deviation-fm = deviazione, in FM
setup-drive-ssb = pilotaggio, in SSB
setup-grid = Locatore
setup-report = Rapporto
setup-r-report = R+Rapporto

## AMP window (src/app/amp.rs)

amp-t-10-w-scale-rescaled = , scala 10 W, riscalata
amp-t-10-w-scale = , scala 10 W
amp-t-2-w-scale-rescaled = , scala 2 W, riscalata
amp-t-2-w-scale = , scala 2 W
amp-t-100-w-scale-rescaled = , scala 100 W, riscalata
amp-t-100-w-scale = , scala 100 W
amp-t-20-w-scale-rescaled = , scala 20 W, riscalata
amp-t-20-w-scale = , scala 20 W
amp-rescaled = , riscalata
amp-and = {" "}E{" "}
amp-not-while-transmitting-switching-relay = Non durante la trasmissione: commutare un relè con l'RF presente può danneggiarlo. Disattiva prima la trasmissione.
amp-no-amplifier-set-up-choose = Nessun amplificatore configurato. Scegli la sua porta in Impostazioni → Amplificatore.
amp-settings = IMPOSTAZIONI
amp-dock = AGGANCIA
amp-put-amp-window-back-inside = Riporta la finestra AMP dentro la finestra principale
amp-pop-out = STACCA
amp-give-amp-window-window-own = Dai alla finestra AMP una finestra propria, da spostare su un altro schermo
amp-rig = , dall'apparato
amp-rig-flagged-reading = , l'apparato ha segnalato la sua lettura
amp-drv = , da DRV
amp-rig-s-swr = , dal ROS dell'apparato
amp-ptt-keys-amplifier-radio-off = PTT attiva l'amplificatore con la radio; OFF lo lascia in bypass. Clicca per commutare.
amp-switch-antenna-jack = Commuta la presa d'antenna
amp-band-down = Banda giù
amp-band-up = Banda su
amp-tuner = ACCORDATORE
amp-tuned = Accordato
amp-tune = ACCORDA
amp-start-tune-then-key-steady = Avvia un accordo — poi attiva una portante stabile (CW, FM o RTTY). Premi di nuovo per fermarlo.
amp-put-tuner-line-bypass = Inserisci l'accordatore in linea, oppure escludilo
amp-tuning = Accordo in corso…
amp-bottom = IN BASSO
amp-pick-any-reading-either-needle = Scegli una lettura qualsiasi per uno dei due aghi. L'ago superiore è bianco, quello inferiore blu.
amp-forward-power-measured-radio-amplifier = Potenza diretta, misurata dalla radio: l'amplificatore non ne riporta in OFF
amp-forward-power-taken-drv-amplifier = Potenza diretta, presa da DRV: l'amplificatore non ne riporta in OFF, e in bypass la potenza della radio passa direttamente
amp-forward-power = Potenza diretta
amp-reflected-power-measured-radio-amplifier = Potenza riflessa, misurata dalla radio: l'amplificatore non ne riporta in OFF
amp-reflected-power-worked-out-fwd = Potenza riflessa, calcolata da FWD e dal ROS: l'amplificatore non ne riporta in OFF
amp-reflected-power = Potenza riflessa
amp-standing-wave-ratio-red-past = Rapporto d'onda stazionaria: rosso al limite di accordo o oltre (Impostazioni → Generale). Controlla l'antenna
amp-standing-wave-ratio-measured-radio = Rapporto d'onda stazionaria, misurato dalla radio: l'amplificatore non ne riporta in OFF
amp-standing-wave-ratio = Rapporto d'onda stazionaria
amp-drive-radio = Pilotaggio dalla radio
amp-drain-supply = Alimentazione di drain
amp-drain-current = Corrente di drain
amp-temp = TEMP
amp-heat-spreader-temperature-yellow-49 = Temperatura del dissipatore: giallo da 49 °C (120 °F), rosso sopra 65 °C (149 °F), fuori linea sopra 70 °C (158 °F). Clicca per °C/°F.
amp-c = °C
amp-f = °F
amp-performance = PRESTAZIONI
amp-numbers-hold-peaks-bars-are = i numeri mantengono i picchi · le barre sono in tempo reale
amp-input-power = POTENZA D'INGRESSO
amp-supply-volts-drain-amps = Tensione di alimentazione × corrente di drain
amp-efficiency = EFFICIENZA
amp-output-power-input-power = Potenza in uscita ÷ potenza in ingresso
amp-gain = GUADAGNO
amp-output-power-over-drive-decibels = Potenza in uscita rispetto al pilotaggio, in decibel
amp-off-line-over-temperature = FUORI LINEA · temperatura eccessiva
amp-connected-transmitting = COLLEGATO · IN TRASMISSIONE
amp-connected = COLLEGATO
amp-connecting = Connessione…
amp-atu-bypassed = ATU ESCLUSO
amp-no-inductance-needed = nessuna induttanza necessaria
amp-no-capacitance-needed = nessuna capacità necessaria
amp-c-out-antenna-side = C OUT · lato antenna
amp-c-transmitter-side = C IN · lato trasmettitore

## SSTV panel (src/app/panels/sstv.rs)

sstv-chunks-received-lit-still-missing = Blocchi ricevuti (accesi) e ancora mancanti (spenti)
sstv-sstv = SSTV
sstv-auto = Auto
sstv-signal = Segnale
sstv-station-sending-mode-sdroxide-does = Una stazione sta trasmettendo in un modo che sdroxide non ha. Il segnale e la sintonia vanno bene — semplicemente non esiste un decodificatore per questo. Nulla da correggere dalla tua parte.
sstv-listening = in ascolto…
sstv-restart-rx = Riavvia RX
sstv-abandon-picture-being-received-listen = Abbandona l'immagine in ricezione e resta in ascolto della successiva intestazione. Per una trasmissione che ha iniziato a decodificarsi nel modo sbagliato — altrimenti il ricevitore resta impegnato finché quel modo non finisce.
sstv-callsign-last-station-sent-fsk = Il nominativo che l'ultima stazione ha inviato come FSK ID dopo la sua immagine.
sstv-tx-slant = Inclinazione TX
sstv-transmit-clock-trim-ppm-remove = Correzione del clock di trasmissione (ppm) per eliminare l'inclinazione sul decodificatore dell'altro capo
sstv-reset-0-ppm = Riporta a 0 ppm
sstv-fsk-id = FSK ID
sstv-send-your-callsign-tones-after = Invia il tuo nominativo in toni dopo ogni immagine — l'identificazione che leggono i ripetitori SSTV e gli altri programmi. Aggiunge circa 2,5 secondi, e non invia nulla finché non hai impostato un nominativo.
sstv-tx-lead = Anticipo TX
sstv-silence-sent-after-keying-before = Silenzio inviato dopo l'attivazione e prima della testata e del codice VIS dell'immagine. Un decodificatore che perde una qualsiasi parte di quell'intestazione non mostra affatto l'immagine, quindi questo copre il tempo tra la richiesta di PTT all'apparato e il suo effettivo ingresso in aria. 0 per un SDR che si attiva istantaneamente.
sstv-live = DAL VIVO
sstv-waiting-picture = in attesa di un'immagine…
sstv-waiting-signal = in attesa di un segnale…
sstv-no-low-audio = audio assente / basso
sstv-received = RICEVUTE
sstv-reading-radio-s-pictures = Lettura delle immagini della radio…
sstv-decoded-pictures-collect-here = Le immagini decodificate si raccolgono qui.
sstv-click-enlarge-right-click-delete = Clicca per ingrandire · clic destro per eliminare
sstv-delete-picture = Elimina questa immagine
sstv-loading = caricamento…
sstv-transmit = TRASMETTI
sstv-click-edit-slot-s-message = Clicca per modificare il messaggio di questo slot · doppio clic per caricare un'immagine
sstv-change-image = Cambia immagine…
sstv-load-image = Carica immagine…
sstv-clear = Cancella
sstv-empty-slot-s-picture-message = Svuota l'immagine di questo slot (il messaggio resta)
sstv-banner = Banner…
sstv-what-printed-across-top-every = Ciò che è stampato in cima a ogni immagine che questa stazione invia
sstv-fetching-slot-s-picture = Recupero dell'immagine di questo slot…
sstv-load-image-into-slot = Carica un'immagine in questo slot →
sstv-drawn-slot-s-image = Disegnato sull'immagine di questo slot
sstv-tx = {"   "}TX{"   "}
sstv-abort-tx = {" "}ANNULLA TX{" "}
sstv-delete-picture-store = Elimina questa immagine dall'archivio
sstv-received-image = Immagine ricevuta
sstv-no-longer-store = non più nell'archivio
sstv-loading-full-size = caricamento a dimensione piena…
sstv-save-picture = Salva immagine…
sstv-save-copy-computer = Salva una copia su questo computer
sstv-delete-sure = Eliminare — sei sicuro?
sstv-click-again-delete-good = Clicca di nuovo per eliminarla definitivamente
sstv-delete = Elimina…
sstv-image-banner = Banner dell'immagine
sstv-drawn-into-every-picture-station = Disegnato in ogni immagine che questa stazione trasmette, sopra il messaggio proprio dello slot.
sstv-draw-banner = Disegna il banner
sstv-substituted-when-picture-composed = Sostituito quando l'immagine viene composta:
sstv-top-left = In alto a sinistra
sstv-top-right = In alto a destra
sstv-colours = Colori
sstv-strip-top-edge-fades-black = La striscia, sul suo bordo superiore — sfuma in nero, oppure nel colore del gradiente sotto di essa
sstv-both-texts = Entrambi i testi
sstv-gradient = Gradiente
sstv-fade = sfuma verso
sstv-fade-strip-top-colour-second = Fai sfumare la striscia dal suo colore superiore a un secondo colore anziché al nero.
sstv-text-outline = Contorno del testo
sstv-draw-outline-around-banner-text = Disegna un contorno attorno al testo del banner, così resta leggibile su una striscia affollata o chiara.
sstv-text-gradient = Gradiente del testo
sstv-fade-banner-text-colour-second = Fai sfumare il testo del banner dal suo colore a un secondo colore lungo la larghezza dell'immagine.
sstv-rainbow-text = Testo arcobaleno
sstv-all-text-overriding-colours-above = tutto il testo, sostituendo i colori qui sopra
sstv-draw-every-text-picture-banner = Disegna ogni testo dell'immagine — banner e messaggio — come un arcobaleno orizzontale, ignorando i colori qui sopra.
sstv-message = Messaggio
sstv-slot-message-s-text-colour = Colore del testo del messaggio dello slot
sstv-outline = contorno in
sstv-height = Altezza
sstv-how-tall-strip-pixels-transmitted = Quanto è alta la striscia, in pixel dell'immagine trasmessa. Il testo è dimensionato su di essa — un fotogramma SSTV è largo solo 320 pixel, quindi un banner più alto è ciò che lo rende leggibile dall'altro capo.
sstv-reset = Ripristina
sstv-back-callsign-left-version-right = Torna al nominativo a sinistra e alla versione a destra
sstv-waiting-station-s-digital-mode = In attesa che si carichino le impostazioni dei modi digitali della stazione…
sstv-rifp = RIFP
sstv-size = Dimensione
sstv-calling-frequency-draft-names = La frequenza di chiamata che la bozza indica
sstv-encode = Codifica
sstv-how-picture-encoded-into-object = Come l'immagine viene codificata nell'oggetto che trasporta RIFP. Auto prova ciascuna e invia la più piccola.
sstv-try-every-encoding-send-smallest = Prova ogni codifica, invia la più piccola (mai con perdita)
sstv-gray = Grigio
sstv-grayscale-depth-rifp-s-raster = Profondità della scala di grigi. Il raster di RIFP è in scala di grigi per definizione — il colore non trova posto nel suo manifest.
sstv-dither = Dither
sstv-diffuse-quantisation-error-worth-below = Diffondi l'errore di quantizzazione — conviene sotto gli 8 bit
sstv-caption = Didascalia
sstv-sent-manifest-content-hint-shown = Inviata nel manifest come suggerimento del contenuto, e mostrata sotto l'immagine dai ricevitori. Viaggia come testo — non viene disegnata nell'immagine.
sstv-what-picture = Cos'è questa immagine
sstv-repeat-data = Ripeti i dati
sstv-send-every-data-frame-many = Invia ogni frame di dati questo numero di volte. RIFP è unidirezionale e senza richieste di ripetizione, quindi questo è l'unico recupero che un ricevitore ottiene.
sstv-chunk = Blocco
sstv-payload-octets-per-data-frame = Ottetti di payload per frame di dati (il profilo consiglia 192)
sstv-valid-frames-frames-failed-crc = Frame validi, frame con CRC errato e immagini complete verificate
sstv-no-transfer-progress = nessun trasferimento in corso
sstv-forget-incomplete-transfer = Dimentica questo trasferimento incompleto

## Memories and voice keyer windows (src/app/windows.rs)

windows-recall = Richiama
windows-delete = Elimina
windows-edit-name-frequency-mode-antenna = Modifica il nome, la frequenza, il modo, l'antenna e la configurazione del ripetitore
windows-r = {" "}R
windows-repeater-shift-tone-1750-hz = Lo spostamento del ripetitore, il tono e il burst a 1750 Hz memorizzati con questo canale
windows-abandon-edit-escape = Annulla la modifica (Esc)
windows-save = SALVA
windows-keep-changes-enter = Conserva le modifiche (Invio)
windows-recall-channel-without-moving-antenna = Richiama questo canale senza spostare l'antenna
windows-antenna-switch-when-channel-recalled = L'antenna su cui commutare quando questo canale viene richiamato. — lascia com'è ciò su cui si trova già la radio.
windows-shift = Spostamento
windows-tone = Tono
windows-invert = INVERTI
windows-normal = NORMALE
windows-dcs-polarity = Polarità DCS
windows-open-every-over-tone-burst = Apri ogni trasmissione con il burst di tono mentre questo canale è richiamato
windows-memories = Memorie
windows-memory-name = nome della memoria
windows-store = Memorizza
windows-folder-name = nome della cartella
windows-new-folder = Nuova cartella
windows-import = IMPORTA
windows-read-channel-list-chirp-csv =
    Legge un elenco di canali da un file CSV di CHIRP (.csv) — un'esportazione di un elenco di ripetitori, una tabella di canali marittimi o PMR, o qualsiasi altra cosa che CHIRP sappia scrivere.

    Ogni canale porta con sé la sua frequenza, il modo, lo spostamento del ripetitore e il tono CTCSS o DCS. I canali già presenti in questo elenco — stessa frequenza, stesso modo — vengono saltati, quindi reimportare un elenco aggiornato aggiunge ciò che è nuovo senza raddoppiare ciò che non lo è.
windows-chirp-csv = CSV CHIRP
windows-export = ESPORTA
windows-write-list-out-chirp-csv = Scrive questo elenco come file CSV di CHIRP, da caricare su un portatile o da conservare come copia di sicurezza.
windows-no-memories-yet = nessuna memoria ancora
windows-drag-memory-onto-folder-file = trascina una memoria su una cartella per archiviarla, sotto di esse per toglierla dall'archivio
windows-drop-here-unfile = rilascia qui per togliere dall'archivio
windows-sort = Ordina
windows-order-they-were-stored = Nell'ordine in cui sono state memorizzate
windows-name-ignoring-case = Per nome, senza distinguere maiuscole e minuscole
windows-frequency = Per frequenza
windows-band-then-frequency-inside-each = Per banda, poi per frequenza all'interno di ciascuna
windows-descending-click-ascending = Decrescente — clicca per crescente
windows-ascending-click-descending = Crescente — clicca per decrescente
windows-delete-folder-memories-move-top = Elimina cartella — le sue memorie passano al livello superiore
windows-rename-folder = Rinomina cartella
windows-empty-drop-memories-here = vuota — trascina qui delle memorie
windows-voice-keyer = Manipolatore vocale
windows-rec-records-your-microphone-play = REC registra dal microfono, RIPRODUCI ti permette di ascoltare ciò che hai registrato, TX lo manda in aria — come fa un tasto del tastierino numerico, un pad MIDI, oppure send_voice_mem di rigctld.
windows-stop-store = Ferma e memorizza
windows-stop = FERMA
windows-play = RIPRODUCI
windows-stop-listening = Smetti di ascoltare
windows-nothing-recorded-slot = Nulla registrato in questo slot
windows-not-while-transmitting = Non durante la trasmissione
windows-listen-message-nothing-transmitted = Ascolta questo messaggio — non viene trasmesso nulla
windows-stop-transmitting = Smetti di trasmettere
windows-tune-active-switch-off-first = TUNE è attivo — disattivalo prima
windows-transmit-message = Trasmetti questo messaggio
windows-erase-recording = Cancella questa registrazione
windows-rade-message-encoded-digital-voice = RADE: il messaggio viene codificato dal codec di voce digitale, esattamente come lo sarebbe una trasmissione dal vivo.

## TX Audio window: messages with values

txaudio-rec-hint = Registra una frase di prova dal microfono — fino a { $max } s, senza attivare il trasmettitore. Clicca di nuovo per fermare. Viene conservata non elaborata, così RIPRODUCI può far passare la stessa frase attraverso ogni modifica che fai.
txaudio-phrase-length = frase di { $secs } s
# $clip is txaudio-microphone-may-clipping-before-gets or empty.
txaudio-auto-set-done = Guadagno d'ingresso impostato a { $gain } dB (i picchi della voce erano a { $loud } dBFS).{ $clip }
txaudio-listening = In ascolto… { $secs } s
txaudio-auto-set-hint = Parla normalmente per { $secs } s — come faresti in aria — e il guadagno d'ingresso viene impostato così che i picchi della voce stiano intorno a { $target } dBFS. Clicca di nuovo per annullare.
txaudio-eq-point = { $hz } Hz, { $db } dB — trascina per spostare
txaudio-not-a-profile = Non è un profilo audio TX: { $error }
txaudio-imported = Importato "{ $name }".
txaudio-saved = Salvato "{ $name }".
txaudio-deleted = Eliminato "{ $name }".
txaudio-loaded-preset = Caricato il punto di partenza "{ $name }".

## Solar-system 3D view: messages with values

solar-age-offline = { $age } · offline
solar-radius-exaggeration = Esagerazione del raggio di Terra/Luna (massimo { $max }× a questa scala dell'orbita lunare)
solar-brightest-cell = cella più luminosa ≈ { $peak } percorsi · { $bands } { $bands ->
        [one] banda
       *[other] bande
    }
solar-hits-in-hour = { $hits } nell'ora
solar-passes-from = { $name } · PASSAGGI DA { $grid }
solar-elements-age = elementi di { $age } · SGP4
solar-point-at = Punta a { $az }° ({ $compass }), elevazione { $el }°.
solar-published-as = pubblicato come { $name }
solar-fly-there = { $name } — ↵ per volarci
solar-sats-tracked = { $n } su { $total } tracciati
solar-bodies-found = { $n } su { $total } corpi
solar-sats-and-bodies = { $n } su { $total } tracciati · { $bodies } su { $bodies_total } corpi
solar-confidence-on-band = { $confidence } su { $band }
solar-kp-now = { $kp } ora
solar-kp-in = { $kp } tra { $hours } h
solar-valid-age = valido { $valid } · { $age } di età
solar-full-bar = barra piena = { $share } del mondo
solar-own-paths-memory = dove sono andati i percorsi di questa stazione · memoria di { $minutes } min
solar-n-storms = { $n } tempeste
solar-clouds-legend = NUVOLE { $time } · { $age } di età · { $channels } · { $storms } · fulmini simulati
solar-cme-eta = ETA { $eta } (+{ $hours } h)
solar-cme-arrived = l'arrivo era { $eta } ({ $hours } h fa)
# $glancing and $estimated are solar-glancing / solar-direction-estimated or empty.
solar-cme-earth-directed = CME DIRETTA VERSO LA TERRA { $time } · { $speed } km/s · { $when }{ $glancing }{ $estimated }

## Digital-mode setup dialogs: messages with values

setup-mode-setup = Configurazione { $mode }
setup-transmits-as = Trasmette come { $call }
setup-decimal-degrees = Gradi decimali, negativi per { $hemisphere }. Incollane uno direttamente da una mappa.

## AMP window: messages with values

amp-unreliable-at-swr = { $meters } NON AFFIDABILE CON QUESTO ROS
amp-lower-drive-first = Abbassa prima il pilotaggio: { $drive } W sovrapiloterebbero l'amplificatore in linea (il suo limite è { $limit } W).
amp-live-value = in tempo reale { $value } { $unit }
amp-tripped = SCATTATO{ $why }

## Digital-mode panels, shared: messages with values

panels-top-paths = massimo ≈ { $n } percorsi
panels-no-agreed-frequency = Nessuna frequenza concordata per { $mode } — apri questo per salvare quella su cui sei
panels-mode-frequencies-hint = Le { $n } frequenze per { $mode } — sceglierne una sintonizza la sintonia, e puoi salvare le tue
panels-mode-frequencies = { $mode } · frequenze
panels-global-convention = Una convenzione globale che il piano di banda della Regione IARU { $region } non riserva ai dati a banda stretta — controlla il tuo piano di banda prima di trasmettere qui.
panels-outside-data-segment = Ambra: fuori dal segmento dati della Regione { $region }.
panels-save-dial = ＋ Salva { $mhz } MHz
panels-save-dial-hint = Ricorda questa sintonia sotto { $mode } e offrila qui d'ora in poi. Salvata sulla stazione, quindi ogni schermo collegato a questa radio la ha.
panels-tr-period = Periodo T/R di { $period } secondi
# $mark is panels-burst-mark while transmitting, otherwise empty.
panels-this-turn = Questo turno: { $into } s di { $slot }. Le decodifiche arrivano alla fine, ed è allora che può iniziare la trasmissione successiva.{ $mark }
panels-burst-mark = {" "}Il segno è dove finisce il burst, a { $secs } s.
panels-tr-period-burst = Periodo T/R di { $slot } secondi, burst di { $burst } s

## Memories and voice keyer windows: messages with values

windows-n-channels = { $n } { $n ->
        [one] canale
       *[other] canali
    }
windows-slot-n = Slot { $n }
windows-record-from-mic = Registra dal microfono (fino a { $max } s)

## SSTV panel: messages with values

sstv-picture-too-big = Quell'immagine pesa { $size } MB — il limite è { $limit } MB.
sstv-auto-mode = Auto ({ $mode })
sstv-not-decoded = { $what } — non decodificato
sstv-last = ultimo: { $mode }
# $place is sstv-store-on-radio or sstv-store-in.
sstv-saved-where = Le immagini decodificate vengono salvate { $place } e si raccolgono qui.
sstv-older-in-store = { $n } più vecchie nell'archivio
sstv-older-load-more = { $n } più vecchie — carica altre
sstv-delete-picture-from = Elimina questa immagine { $place }
sstv-rifp-profile = { $profile } · ~{ $khz } kHz occupati · la sintonia è il centro del canale
sstv-rifp-too-wide = ⚠ { $profile } occupa ~{ $khz } kHz — troppo largo per un segmento a banda stretta
sstv-rifp-no-frequency = RIFP non assegna alcuna frequenza, e sdroxide lo trasmetterà ovunque tu sintonizzi. Un canale da { $khz } kHz sta solo dove è consentito il funzionamento a banda larga o FM — { $segments } — e non in un segmento a banda stretta, tanto meno in HF. Anche dentro quei segmenti le condizioni della tua licenza possono essere più ristrette. Sei tu l'operatore; controlla le tue regole.
sstv-rifp-tx-frame = ● Frame TX { $frame }/{ $frames } · { $secs } s rimanenti
sstv-rifp-sent-as = inviato come { $encoding } · { $octets } ottetti
sstv-rifp-rx-stats = frame { $frames } · errati { $bad } · immagini { $pictures }
sstv-rifp-session-idle = sessione { $session } · inattiva { $secs } s
sstv-rifp-session-waiting = sessione { $session } · blocchi trattenuti, in attesa del manifest · inattiva { $secs } s
sstv-store-on-radio = sulla radio, in { $dir }
sstv-store-in = in { $dir }

## decodes (src/app/panels/decodes.rs)

decodes-decodes = DECODIFICHE
decodes-adif = ADIF
decodes-sort = Ordina
decodes-single-list = Elenco unico
decodes-every-decode-one-list-newest = Ogni decodifica in un unico elenco, prima il turno più recente, anziché blocchi di turni pari/dispari. I chip di ordinamento ordinano allora l'intero elenco anziché ciascun turno, e ogni riga porta il suo orario di slot — ciano per uno slot pari, oro per uno dispari.
decodes-cq-only = Solo CQ
decodes-only-stations-calling-cq-only = Solo le stazioni che chiamano CQ — e solo le chiamate a cui puoi rispondere: un CQ diretto (DX, EU, JA, POTA, TEST …) viene elencato quando nomina te e nascosto quando nomina qualcun altro.
decodes-new-only = Solo nuove
decodes-only-stations-would-new-entity = Solo le stazioni che sarebbero nuove: entità, banda-slot, locatore o nominativo
decodes-auto-tx-frq = FRQ TX automatica
decodes-overridden-hold-tx-lift-hold = Sostituito da Mantieni TX. Togli il blocco per scegliere in che verso si sposta la frequenza di trasmissione.
decodes-pick-our-transmit-frequency-automatically = Scegli automaticamente la nostra frequenza di trasmissione: il punto più tranquillo nel periodo in cui trasmettiamo, anziché la frequenza di chi stiamo rispondendo — loro trasmettono nell'altro periodo, quindi la loro non dice nulla su chi c'è quando attiviamo. Disattivato NON la mantiene: risponde sulla frequenza della stazione chiamata. Per mantenerla, usa Mantieni TX.
decodes-hold-tx = Mantieni TX
decodes-pin-transmit-tone-where-nothing =
    Fissa il tono di trasmissione dove si trova. Nulla lo sposta: né rispondere a una stazione, né la coda delle chiamate, né chiamare CQ, né un clic su una decodifica o sulla waterfall. Disattivalo per spostarlo, poi riattivalo.

    Il cambio di banda è l'unica eccezione, ed è un atto tuo: l'offset impostato per ultimo sulla nuova banda torna con essa, quindi un valore dei 60 m non viene portato sui 20 m e quello dei 20 m non viene portato sui 60 m.

    Per quando la licenza è più stretta del piano di banda — su un 60 m britannico con sintonia a 5357 kHz l'assegnazione finisce a 5358.0, quindi il tono deve restare sotto 1000 Hz, e ciascuno dei due spostamenti automatici può uscire dalla banda tra una trasmissione e la successiva.
decodes-type-transmit-offset-hz-then = Scrivi l'offset di trasmissione in Hz, poi premi Invio. Dove la licenza è più stretta del piano di banda, l'intero intervallo sotto il bordo è tuo da scegliere, quindi qui non viene suggerito nulla.
decodes-clear-rx = {" "}CANCELLA RX{" "}
decodes-empty-decode-list-nothing-air = Svuota l'elenco delle decodifiche. Nulla di ciò che è in aria si ferma.
decodes-even = PARI
decodes-text = TESTO
decodes-dxcc = DXCC
decodes-band = BANDA
decodes-grid = LOCATORE
decodes-dupe = DOPPIO
decodes-reply = RISPOSTA
decodes-queued-click-remove = In coda — clicca per rimuovere
decodes-work-station-after-current-one = Lavora questa stazione dopo quella attuale
decodes-qsos-worked-since-sdroxide-was = QSO lavorati da quando sdroxide è stato avviato
decodes-setup = ⚙ CONFIGURA
decodes-qso-complete = ✓ QSO COMPLETATO
decodes-contact-complete-log-held-here = Il contatto è completo ed è nel log. Viene trattenuto qui per alcuni minuti, così il messaggio finale può essere ripetuto se l'altra stazione ripete il suo — non resta altro da fare.
decodes-tx = ● TX
decodes-dxpedition-mode-transmit-frequency-held = Modalità DXpedition. La frequenza di trasmissione è tenuta fuori dalla metà della banda passante della Fox (sotto 1000 Hz) finché la Fox non risponde.
decodes-watchdog = SORVEGLIANZA
decodes-transmitting-stopped-no-reply-no = Trasmissione interrotta: nessuna risposta e nessuna azione per il periodo di sorveglianza. Chiama CQ o scegli un messaggio per riprendere.
decodes-well-inside-tolerance = Ben dentro la tolleranza.
decodes-no-active-qso-pick-decode = nessun QSO attivo — scegli una decodifica a cui rispondere, oppure Chiama CQ
decodes-ft8-engine-idle = Motore FT8 inattivo
decodes-being-worked = in lavorazione
decodes-clear = CANCELLA
decodes-no-messages = — nessun messaggio —
decodes-call-cq = {"  "}CHIAMA CQ{"  "}
decodes-stop-qso = {" "}FERMA QSO{" "}
decodes-stop-tx = {" "}FERMA TX{" "}
decodes-free-text-13-chars = testo libero (13 caratteri)
decodes-send = INVIA

## sat (src/app/sat.rs)

sat-pass-profile = PROFILO DEL PASSAGGIO
sat-satellite = SATELLITE
sat-satellites = SATELLITI
sat-passes = PASSAGGI
sat-set-your-grid-passes-settings = Imposta il tuo locatore per i passaggi (Impostazioni ▸ Generale).
sat-next = PROSSIMO
sat-peak = PICCO ≥
sat-all-satellites = tutti i satelliti
sat-off-only-satellites-frequencies-file = Disattivato: solo i satelliti con frequenze in archivio — quelli con cui puoi lavorare — e quelli che hai incollato tu. Attivato: ogni satellite dell'elenco, che con un abbonamento possono essere migliaia e richiedono un po' di tempo per la ricerca.
sat-no-passes-window-lower-peak = Nessun passaggio in questa finestra. Abbassa il picco, oppure allarga la finestra.
sat-length = DURATA
sat-peak-2 = PICCO
sat-from-to = DA → A
sat-up-now = ● visibile ora
sat-unlock = SBLOCCA
sat-release-lock-dial-stays-where = Rilascia il blocco; la sintonia resta dov'è
sat-azimuth = AZIMUT
sat-elevation = ELEVAZIONE
sat-range = DISTANZA
sat-rate = VELOCITÀ
sat-downlink = DISCESA
sat-doppler-rx = DOPPLER RX
sat-uplink = SALITA
sat-doppler-tx = DOPPLER TX
sat-no-pass-inside-next-48 =Nessun passaggio nelle prossime 48 ore dal tuo QTH.
sat-elements-stale-corrections-suspended-until = ⚠ Elementi obsoleti — correzioni sospese fino a un aggiornamento del TLE
sat-doppler-correction-needs-iq-front = La correzione Doppler richiede un front-end IQ — un apparato CAT traccia solo
sat-rotator-not-connected = Rotore: non collegato
sat-doppler = DOPPLER
sat-apply-computed-correction-rx-tx = Applica la correzione calcolata a RX e TX
sat-rotator = ROTORE
sat-steer-antenna-through-rotctld-client = Orienta l'antenna tramite il client rotctld (Impostazioni ▸ Server)
sat-name-norad = nome o NORAD
sat-set-your-grid-passes-settings-2 = imposta il tuo locatore per i passaggi (Impostazioni ▸ Generale)
sat-norad = NORAD
sat-pass = PASSAGGIO
sat-always-visible = sempre visibile
sat-not-here = non visibile da qui
sat-no-elements = nessun elemento
sat-pick-satellite-see-links-lock = Scegli un satellite per vederne i collegamenti e agganciarlo.
sat-no-frequencies-file-one-add = Nessuna frequenza in archivio per questo — aggiungile in Impostazioni ▸ TLE.
sat-link = COLLEGAMENTO
sat-downlink-mhz = DISCESA (MHz)
sat-uplink-mhz = SALITA (MHz)
sat-mode = MODO
sat-tune = SINTONIZZA
sat-set-dial-mode-link-no = Imposta sintonia e modo su questo collegamento — nessun blocco, nessuna correzione
sat-locked = {" "}AGGANCIATO{" "}
sat-lock = {" "}BLOCCA SU{" "}
sat-track-satellite-doppler-correction-rx = Segui questo satellite: correzione Doppler su RX e TX, salita mappata sul transponder, e la vista 3D lo segue

## wefax (src/app/panels/wefax.rs)

wefax-wefax = WEFAX
wefax-stop = {" "}■ FERMA{" "}
wefax-end-chart-now-save-what = Chiude la carta ora e salva ciò che è arrivato
wefax-start = {" "}● AVVIA{" "}
wefax-start-chart-now-without-waiting = Avvia una carta subito, senza attendere un tono di avvio
wefax-phasing = fasatura…
wefax-subcarrier-offset-1900-hz-tune = Offset della sottoportante da 1900 Hz. Sintonizza per circa zero: la portante fax va da 1500 Hz (nero) a 2300 Hz (bianco), e un ricevitore disallineato di qualche centinaio di hertz taglia l'immagine a nero pieno o bianco pieno.
wefax-auto-start = AVVIO AUTOMATICO
wefax-begin-chart-when-300-hz = Inizia una carta quando si sente il tono di avvio a 300 Hz (IOC 576) o 675 Hz — e chiude quella in corso e fascia la nuova quando arriva la trasmissione successiva, con o senza un tono di stop prima.
wefax-auto-stop = STOP AUTOMATICO
wefax-end-450-hz-stop-tone = Chiude sul tono di stop a 450 Hz. Disattivalo per continuare a registrare attraverso una stazione che invia più carte di seguito.
wefax-phase = FASE
wefax-slant = INCLINAZIONE
wefax-sample-clock-trim-if-chart = Correzione del clock di campionamento. Se la carta pende a sinistra, aumentala; a destra, diminuiscila. Una scheda audio di cento ppm fuori dalla norma sposta lateralmente di una riga intera una carta da un quarto d'ora.
wefax-view = VISTA
wefax-scale-chart-panel-width = Scala la carta alla larghezza del pannello
wefax-whole = INTERA
wefax-shrink-chart-until-all-view = Riduci la carta finché tutta non è visibile in una volta
wefax-half-size = Metà dimensione
wefax-one-screen-pixel-per-fax = Un pixel dello schermo per ogni pixel del fax — scorri per il dettaglio
wefax-twice-size-scroll-move-around = Doppia dimensione; scorri per muoverti nella carta
wefax-height = ALTEZZA
wefax-stretch-picture-vertically-chart-came = Allunga l'immagine in verticale. Una carta uscita schiacciata o allungata di solito è decodificata alla velocità di riga sbagliata — questo la rende leggibile, e i chip LPM la correggono davvero. Doppio clic per scrivere un valore.
wefax-reset = RIPRISTINA
wefax-back-picture-s-own-proportions = Torna alle proporzioni dell'immagine
wefax-follow = SEGUI
wefax-keep-newest-lines-view-scrolling = Tiene visibili le righe più recenti. Scorrere verso l'alto lo disattiva, così puoi leggere ciò che è già arrivato; tornare in fondo lo riattiva.
wefax-tune-fax-schedule-usb-wait = Sintonizza un programma fax in USB e attendi un tono di avvio, oppure premi AVVIA per iniziare a metà carta.
wefax-saved = SALVATE
wefax-path = PERCORSO
wefax-reading-radio-s-charts = Lettura delle carte della radio…
wefax-finished-charts-collect-here = Le carte completate si raccolgono qui.
wefax-delete-chart = Elimina questa carta
wefax-loading = caricamento…
wefax-stations = STAZIONI
wefax-broadcast-radiofax-schedules-picking-one = Programmi di radiofax di radiodiffusione — sceglierne uno sintonizza la sintonia
wefax-frequencies-are-published-carrier-dial = Le frequenze sono la portante pubblicata; la sintonia va 1,9 kHz sotto di essa, cosa che viene fatta per te. I programmi cambiano e le stazioni chiudono — consideralo un punto da cui iniziare a cercare, non un orario.
wefax-delete-chart-store = Elimina questa carta dall'archivio
wefax-newer = ◀ PIÙ RECENTE
wefax-chart-received-after-one = La carta ricevuta dopo questa
wefax-older = PIÙ VECCHIA ▶
wefax-chart-received-before-one = La carta ricevuta prima di questa
wefax-no-longer-store = non più nell'archivio
wefax-loading-full-size = caricamento a dimensione piena…
wefax-save = SALVA
wefax-save-copy-chart-computer = Salva una copia di questa carta su questo computer
wefax-sure = SEI SICURO?
wefax-click-again-delete-chart-good = Clicca di nuovo per eliminare questa carta definitivamente
wefax-delete = ELIMINA
wefax-file-chart-was-saved = Il file con cui questa carta è stata salvata

## aprs (src/app/panels/aprs.rs)

aprs-starting-aprs-modem = avvio del modem APRS…
aprs-aprs = APRS
aprs-no-callsign = nessun nominativo
aprs-nothing-will-transmitted-no-beacon = Non verrà trasmesso nulla — né radiofaro, né messaggio, nemmeno una conferma — finché questa stazione non ha un nominativo. Impostane uno in Impostazioni → Generale, oppure uno specifico per APRS con il suo SSID in CONFIGURA.
aprs-no-position = nessuna posizione
aprs-beacon-needs-somewhere-report-fill = Un radiofaro ha bisogno di un punto da riportare. Compila il tuo locatore in Impostazioni → Generale, oppure indica le coordinate in CONFIGURA.
aprs-setup = CONFIGURA
aprs-callsign-symbol-digipeater-path-position = Nominativo, simbolo, percorso dei digipeater, posizione e radiofaro
aprs-how-often-beacon-your-position = Ogni quanto inviare la tua posizione come radiofaro, senza presidio. `off` — il predefinito — non invia mai radiofari: scegliere un modo non deve mettere una stazione in aria. Trenta minuti è la convenzione per una stazione fissa, più spesso per una in movimento, e ogni radiofaro è tempo di canale di qualcun altro.
aprs-beacon = {" "}RADIOFARO{" "}
aprs-station-has-no-callsign-see = Questa stazione non ha un nominativo — vedi l'avviso a sinistra.
aprs-station-has-no-position-report = Questa stazione non ha una posizione da riportare — vedi l'avviso a sinistra.
aprs-send-one-position-report-now = Invia subito un rapporto di posizione, senza attendere l'intervallo. Attende che il canale sia libero come tutto il resto.
aprs-stations = STAZIONI
aprs-nothing-heard-yet = niente di sentito ancora
aprs-heard-direct = sentito direttamente
aprs-two-characters-station-sent-say = I due caratteri che la stazione ha inviato per dire cos'è
aprs-killed = ANNULLATO
aprs-station-put-object-here-has = La stazione che ha messo qui questo oggetto l'ha annullato
aprs-sender-deliberately-reported-square-rather = Il mittente ha riportato deliberatamente un quadrato anziché un punto. La mappa disegna il quadrato.
aprs-digipeaters-last-frame-came-through = I digipeater attraverso cui è passato l'ultimo frame. Un `*` segna quello che l'ha davvero ripetuto.
aprs-message = {" "}MESSAGGIO{" "}
aprs-address-message-box-station = Indirizza la casella dei messaggi a questa stazione
aprs-center = {" "}CENTRA{" "}
aprs-put-station-middle-map-double = Metti questa stazione al centro della mappa. Doppio clic sulla mappa per restituire la vista all'adattamento automatico.
aprs-channel = CANALE
aprs-messages = MESSAGGI
aprs-frames-arrived-failed-their-check =Frame arrivati che non hanno superato la sequenza di controllo — una collisione, un'attenuazione o un segnale troppo debole per essere letto. Un conteggio che sale senza che nulla venga decodificato significa che il modem *sta* sentendo il canale: controlla lo strumento di livello e apri lo squelch della radio.
aprs-frames-read-cleanly-off-channel = Frame letti senza errori sul canale che non erano APRS — la sessione pacchetto di qualcun altro, oppure un formato che questa build non decodifica.
aprs-every-frame-channel-arrived-other = Ogni frame sul canale, così come è arrivato — incluso il traffico altrui, e tutto ciò che il decodificatore non ha saputo leggere.
aprs-nothing-channel-yet = nulla sul canale ancora
aprs-no-messages = Nessun messaggio.
aprs-waiting-channel = in attesa del canale
aprs-sent-retried-still-waiting-acknowledged = inviato e ritentato, in attesa ancora di conferma
aprs-air-waiting-acknowledged = in aria, in attesa di conferma
aprs-acknowledged-far-end = confermato dall'altro capo
aprs-refused-far-end-not-retried = rifiutato dall'altro capo — non ritentato
aprs-no-ack = nessuna conferma
aprs-no-answer-after-every-retry = nessuna risposta dopo ogni tentativo
aprs-send = {" "}INVIA{" "}
aprs-station-has-no-callsign-so = Questa stazione non ha un nominativo, quindi non si può trasmettere nulla. Impostane uno in Impostazioni → Generale, oppure uno specifico per APRS con il suo SSID in CONFIGURA.
aprs-needs-station-address-something-say = Servono una stazione a cui indirizzarlo e qualcosa da dire.
aprs-send-keep-retrying-until-far = Invialo, e continua a ritentare finché l'altro capo non conferma. I messaggi sono l'unica cosa su questo canale a cui si risponde.
aprs-busy = OCCUPATO
aprs-stations-heard-still-inside-window = Stazioni sentite ancora nella finestra impostata in Configurazione APRS, e se un'altra stazione è sul canale in questo momento. Nulla verrà attivato mentre è occupato.
aprs-tx = ● TX
aprs-over-progress-frames-waiting-channel = La trasmissione in corso, i frame in attesa che il canale si liberi, oppure il tempo fino al prossimo radiofaro programmato.
aprs-audio-reaching-modem-flat-means = Audio che arriva al modem. Piatto significa che non arriva nulla — verifica che l'uscita dati della radio sia quella su cui sta ascoltando sdroxide. Si accende di verde mentre il modem sente una portante.

## vdl2 (src/app/panels/vdl2.rs)

vdl2-starting-vdl2-decoder = avvio del decodificatore VDL2…
vdl2-aircraft-datalink = collegamento dati aeronautico
vdl2-tune-middle-vdl2-group-decoder = Sintonizzati al centro del gruppo VDL2. La finestra propria del decodificatore scorre da lì per includere quanti più dei quattordici canali la radio riesce a raggiungere.
vdl2-setup = CONFIGURA
vdl2-which-channels-listen-how-hard = Su quali canali ascoltare, quanto forte deve essere un burst, e quanto log conservare
vdl2-save-vdl2-message-log-file = Salva il registro dei messaggi VDL2 su un file
vdl2-vdl2-channel = Canale VDL2
vdl2-messages = MESSAGGI
vdl2-address-flight-registration-label = indirizzo, volo, registrazione o etichetta
vdl2-held = BLOCCATO
vdl2-hold = BLOCCA
vdl2-hold-message-log-where-so = Blocca il registro dei messaggi dove si trova, così uno può essere letto senza che la trasmissione successiva lo spinga su nello schermo. Il decodificatore continua a girare e i contatori continuano a muoversi; rilasciando si vede tutto ciò che è arrivato nel frattempo.
vdl2-nothing-decoded-yet = nulla di decodificato ancora
vdl2-nothing-matches-filter = nulla corrisponde al filtro
vdl2-stations = STAZIONI
vdl2-nobody-heard-yet = nessuno sentito ancora
vdl2-vdl2-setup = Configurazione VDL2
vdl2-channels = Canali
vdl2-every-25-khz-slot-136 = Ogni slot da 25 kHz da 136,650 a 136,975 MHz. Un convertitore a discesa per ciascuno, tutti dentro la stessa finestra del ricevitore. Disattivarne uno fa risparmiare un po' di tempo di processore; non rende gli altri più sensibili.
vdl2-burst-threshold = Soglia del burst
vdl2-above-each-channel-s-own = sopra il rumore di fondo proprio di ciascun canale
vdl2-lower-catches-weaker-transmissions-costs = Più bassa cattura trasmissioni più deboli e costa tempo di processore sul rumore; più alta le perde. Il fondo è appreso per canale e una modifica qui non lo butta via.
vdl2-keep-log = Conserva nel registro
vdl2-messages-2 = {" "}messaggi
vdl2-track-most = Traccia al massimo
vdl2-stations-2 = {" "}stazioni
vdl2-longest-silent-go-first = prima vanno quelle più silenziose da più tempo
vdl2-forget-station-after = Dimentica una stazione dopo
vdl2-nothing-heard = senza aver sentito nulla da essa
vdl2-show-unread-payloads = Mostra i payload non letti
vdl2-hex = in esadecimale
vdl2-frames-carrying-x-25-clnp = Frame che portano X.25, CLNP o le applicazioni di collegamento dati sopra di essi. SDRoxide li indica e mostra i byte anziché leggerli, e nasconderli nasconderebbe quanto di quel traffico sia.
vdl2-registration = Registrazione
vdl2-flight = Volo
vdl2-label = Etichetta
vdl2-block = Blocco
vdl2-sequence = Sequenza
vdl2-mode = Modo
vdl2-more-blocks-follow = seguono altri blocchi
vdl2-check = Controllo
vdl2-message-crc-good = CRC del messaggio corretto
vdl2-message-crc-not-checked = CRC del messaggio non verificato
vdl2-parity = Parità
vdl2-link-control-no-payload = controllo di collegamento, nessun payload
vdl2-name = NOME
vdl2-addr = IND
vdl2-type = TIPO
vdl2-msgs = MSG

## cw (src/app/panels/cw.rs)

cw-tone-being-copied-tone-transmitted = Il tono che viene copiato, e il tono trasmesso — in CW sono la stessa frequenza. Clicca sulla waterfall per spostarlo su un segnale.
cw-down-10-hz = Giù di 10 Hz
cw-up-10-hz = Su di 10 Hz
cw-qrg-main-readout-tuning-line = QRG: la lettura principale e la linea di sintonia sono sulla frequenza su cui si lavora. Clicca per la sintonia del dial, con un tono di sidetone sotto di essa — ciò che mostra la maggior parte delle radio.
cw-put-main-readout-tuning-line =
    Porta la lettura principale e la linea di sintonia sul segnale anziché sul dial, così la frequenza mostrata è quella che citerebbero entrambi gli operatori e la linea di sintonia sta al centro della banda passante. La sintonia non cambia; cambiano solo i numeri.

    Cliccando su un segnale lo si porta sul cursore solo con la precisione che consente il passo di clic — Comandi → arrotondamento della sintonia al clic, 10 Hz di default. Un passo grossolano lascia il segnale fuori dall'intonazione fino a metà di esso, e la lettura lo dirà.
cw-sending-speed-read-off-signal = Velocità di invio letta dal segnale
cw-signal-noise-500-hz-same = Segnale su rumore in 500 Hz — la stessa cifra che riporta un rapporto
cw-how-far-off-cursor-signal = Quanto il segnale è davvero lontano dal cursore. Il decodificatore lo segue; la banda passante no, quindi sposta un po' il cursore se cresce.
cw-listening = — in ascolto —
cw-tx = ● TX
cw-trainer = ALLENATORE
cw-learn-morse-read-characters-words = Impara il Morse: leggi caratteri e parole, riproducili alla tua velocità, e un esercizio Koch che aggiunge un carattere alla volta. Riproduce qui, dagli altoparlanti di questo computer — nulla attiva la radio.
cw-nothing-copied-yet = — nulla di copiato ancora —
cw-key-send-characters-you-send = Premi il tasto per inviare — i caratteri che invii compaiono qui.
cw-type-line-return-sends = Scrivi una riga, Invio la invia…
cw-type-here-send = Scrivi qui per inviare…
cw-key = {" "}TASTO ●{" "}
cw-key-2 = {" "}TASTO{" "}
cw-hold-key-bound-cw-straight = Tieni premuto il tasto assegnato al tasto CW dritto — la barra spaziatrice per impostazione predefinita, e qualunque tasto in Impostazioni → Comandi — come tasto dritto: giù finché lo tieni, su al rilascio, invece di scrivere testo. La casella di trasmissione è bloccata mentre è attivo, e l'intero manipolatore è affidato al tasto: il testo in coda viene scartato.
cw-radio-sends-own-keyer-text =
    Questa radio invia dal proprio manipolatore: il testo passa per la porta di controllo e l'apparato temporizza gli elementi, quindi non c'è nulla tra il tasto dritto e l'aria che una mano possa pilotare.

    Per manipolarlo a mano, imposta Manipolazione CW su "Scheda audio (MCW)" in Impostazioni → Radio. L'apparato viene allora tenuto su una banda laterale e il sidetone del manipolatore viene trasmesso come audio, che è la via pilotata dal tasto dritto.
cw-watchdog = SORVEGLIANZA
cw-straight-key-was-held-down = Il tasto dritto è stato tenuto premuto troppo a lungo — un rilascio perso anziché una mano — quindi la portante è stata abbattuta e la trasmissione disattivata. Premi di nuovo il tasto per continuare.
cw-tx-2 = {"  "}TX ON{"  "}
cw-tx-3 = {"   "}TX{"   "}
cw-send-what-box-now-without = Invia subito ciò che c'è nella casella, senza attendere Invio
cw-hold-key-down-between-characters = Tieni premuto il tasto tra un carattere e l'altro, così nulla di ciò che scrivi resta in attesa
cw-call-cq = {" "}CHIAMA CQ{" "}
cw-nocall = NOCALL
cw-clear = {" "}CANCELLA{" "}
cw-stop-sending-drop-whatever-has = Smetti di inviare e scarta ciò che non è ancora uscito. Con il tasto dritto attivo, va via con esso anche la rilettura di ciò che hai manipolato.
cw-sidetone = SIDETONE
cw-play-keyed-tone-through-computer = Riproduci il tono manipolato dagli altoparlanti di questo computer oltre a inviarlo, così senti ciò che stai inviando. Con `Scheda audio (MCW)` il tono va alla radio e da nessun'altra parte, quindi senza questo si invia nel silenzio. Disattivalo dove il monitor della radio o una copia in aria fa già il lavoro.
cw-hold-what-typed-until-return = Tieni ciò che scrivi finché non premi Invio, poi invia la riga in un unico blocco anziché manipolare ogni carattere mentre lo scrivi. Vale la pena averlo su un ricetrasmettitore che si attiva da solo dal testo, dove ogni passaggio al suo manipolatore è un altro ciclo trasmissione-ricezione.
cw-your-own-message-buttons-contest = I tuoi pulsanti di messaggio — uno scambio di contest, una risposta con nome e QTH, TNX 73 GL. Ognuno invia tutto il suo testo in una volta, e F2–F10 premono i primi nove. Viaggiano con la configurazione della stazione, quindi anche un client remoto li ha.
cw-cw-messages = MESSAGGI CW
cw-clear-rx = {" "}CANCELLA RX{" "}
cw-empty-receive-window-straight-key = Svuota la finestra di ricezione e la rilettura del tasto dritto. Nulla di ciò che è in aria si ferma.
cw-lock = BLOCCA
cw-decode-speed-set-here-instead = Decodifica alla velocità impostata qui anziché leggerla dal segnale. Aiuta un segnale troppo debole perché la ricerca di velocità si stabilizzi.
cw-farnsworth-send-characters-full-speed = Farnsworth: invia i caratteri a piena velocità e allunga solo gli intervalli tra di essi, così si sentono nel ritmo giusto ma arrivano abbastanza lenti da poterli scrivere.
cw-off-normal-spacing = Disattivato — spaziatura normale
cw-keying-speed = Velocità di manipolazione
cw-idle-0 = INATTIVO 0
cw-how-long-transmit-held-after = Per quanto tempo la trasmissione resta attiva dopo l'ultimo carattere, o dopo il rilascio del tasto dritto, prima che la portante cada. Più lungo copre le pause di chi scrive lentamente; più corto esce prima dalla frequenza; Disattivato la abbatte appena tutto ciò che è in coda è uscito.
cw-off-drop-once = Disattivato — abbatti subito

## textmodem (src/app/panels/text_modem.rs)

textmodem-tune-down-10-hz = Sintonizza giù di 10 Hz
textmodem-tune-up-10-hz = Sintonizza su di 10 Hz
textmodem-tx = ● TX
textmodem-listening = — in ascolto —
textmodem-type-line-return-sends = Scrivi una riga, Invio la invia…
textmodem-type-here-transmit = Scrivi qui per trasmettere…
textmodem-tx-2 = {"  "}TX ON{"  "}
textmodem-tx-3 = {"   "}TX{"   "}
textmodem-call-cq = {" "}CHIAMA CQ{" "}
textmodem-nocall = NOCALL
textmodem-clear = {" "}CANCELLA{" "}
textmodem-hold-what-typed-until-return = Tieni ciò che scrivi finché non premi Invio, poi invia la riga in un unico blocco anziché trasmettere ogni carattere in aria mentre lo scrivi. Permette di rileggere e correggere una riga prima che ne esca qualcosa.
textmodem-your-own-message-buttons-working = I tuoi pulsanti di messaggio — condizioni di lavoro, meteo, una risposta standard. Ognuno invia tutto il suo testo in una volta, e F2–F10 premono i primi nove. Viaggiano con la configurazione della stazione, quindi anche un client remoto li ha.
textmodem-messages = MESSAGGI
textmodem-hell = HELL
textmodem-contrast = Contrasto
textmodem-harder-softer-dots-redraws-whole = Punti più o meno marcati — ridisegna l'intera striscia
textmodem-width = Larghezza
textmodem-t-2row = {" "}2RIGHE{" "}
textmodem-draw-every-column-twice-stacked = Disegna ogni colonna due volte, impilata. Hell non ha sincronizzazione verticale, quindi questo mantiene una copia completa del testo leggibile qualunque sia la fase.
textmodem-rev = {" "}INV{" "}
textmodem-reverse-video-light-dots-dark = Video invertito — punti chiari su carta scura
textmodem-clear-rx = {" "}CANCELLA RX{" "}
textmodem-hold-channel-idle-sends-blank = Blocca il canale: a riposo invia carta bianca, così la striscia continua a scorrere
textmodem-hold-what-typed-until-return-2 = Tieni ciò che scrivi finché non premi Invio, poi invia la riga in un unico blocco anziché disegnare ogni carattere sulla striscia mentre lo scrivi. Una correzione scritta in diretta è già sulla carta; una fatta prima di Invio non ci è mai stata.
textmodem-shift = Spostamento
textmodem-shift-5-hz = Spostamento −5 Hz
textmodem-shift-5-hz-2 = Spostamento +5 Hz
textmodem-baud = Baud
textmodem-reverse-swap-mark-space-needed = Inverti: scambia marcatura e spazio. Serve quando il segnale viene ricevuto sulla banda laterale opposta a quella su cui è stato inviato, il che inverte i toni.
textmodem-track-tuning-error-automatically-instead = Segui automaticamente l'errore di sintonia anziché restare fissato sul cursore.
textmodem-tones = Toni
textmodem-mode = Modo
textmodem-off-keyed = attivo/disattivo a manipolazione

## ais (src/app/panels/ais.rs)

ais-starting-ais-decoder = avvio del decodificatore AIS…
ais-tune-between-two-ais-channels = Sintonizzati fra i due canali AIS — 161,975 (AIS 1, marittimo 87B) e 162,025 (AIS 2, 88B). La sintonia va in mezzo perché lì non trasmette nulla, quindi il picco DC di un ricevitore zero-IF non cade su nessuno dei due.
ais-setup = CONFIGURA
ais-channels-timeouts-trail-length-how = Canali, timeout, lunghezza della scia e fino a dove arrivano i vettori
ais-vessels = NAVI
ais-nothing-heard-yet = niente di sentito ancora
ais-virtual-nothing-there = virtuale — non c'è niente
ais-position-stale = posizione non aggiornata
ais-search-rescue-transmitter-man-overboard = un trasmettitore di ricerca e soccorso, un radiofaro di uomo in mare o un EPIRB AIS
ais-call-sign = nominativo
ais-station-time = ora della stazione
ais-dgnss = {" "}(DGNSS)
ais-first-heard = sentito per la prima volta
ais-center = CENTRA
ais-put-vessel-middle-chart = Metti questa nave al centro della carta
ais-last-message-station-nmea-sentences = L'ultimo messaggio di questa stazione, come le frasi NMEA che un ricevitore metterebbe sul filo. Incollalo in qualunque altro decodificatore AIS per verificare cosa ne ha ricavato sdroxide.
ais-ais-setup = Configurazione AIS
ais-channels = Canali
ais-drop-chart-after = Rimuovi dalla carta dopo
ais-without-position-report = senza un rapporto di posizione
ais-five-minutes-default-not-ads = Cinque minuti di default, non i dieci secondi di ADS-B: una nave all'ancora riporta una volta ogni tre minuti, e una finestra più corta svuoterebbe gran parte di un porto tra due rapporti perfettamente validi. Oltre quel tempo la nave esce dalla carta e la sua riga diventa grigia — non viene sbiadita, perché un simbolo attenuato su una posizione vecchia è comunque un'affermazione su dove si trova una nave, con lo stesso inchiostro di quelle vere.
ais-drop-list-after = Rimuovi dall'elenco dopo
ais-nothing-heard-all = senza aver sentito nulla
ais-message-carrying-ship-s-name = Il messaggio che porta il nome di una nave torna ogni sei minuti, quindi una finestra d'elenco breve continua a scartare navi proprio prima che dicano come si chiamano.
ais-trail-length = Lunghezza della scia
ais-history-behind-each-target = di storia dietro ogni bersaglio
ais-minutes-rather-than-points-because = In minuti anziché in punti, perché le frequenze di segnalazione AIS coprono due ordini di grandezza: un conteggio fisso sarebbe ottanta secondi di un traghetto e due ore di una petroliera all'ancora, disegnate allo stesso modo. Zero disattiva le scie.
ais-speed-vector = Vettore di velocità
ais-how-far-ahead-vector-reaches = fino a dove arriva il vettore in avanti
ais-slot-threshold = Soglia dello slot
ais-above-channel-s-learned-noise = sopra il rumore di fondo appreso del canale
ais-track-most = Traccia al massimo
ais-vessels-2 = {" "}navi
ais-ais-receive-only-here-always = L'AIS qui è solo ricezione e lo sarà sempre: è un servizio per la sicurezza della vita in mare, e inserirvi traffico di navi falso non è coperto da alcuna licenza.
ais-fill-my-grid-digimode-setup = Compila Il mio locatore nella configurazione dei modi digitali per distanze e rilevamenti, così la carta si inquadra attorno a dove sei anziché attorno a ciò che è più lontano.
ais-name = NOME
ais-mmsi = MMSI
ais-type = TIPO

## qo100 (src/app/qo100.rs)

qo100-receiver-cannot-reach-10489-750 = Questo ricevitore non può raggiungere 10489.750 MHz da solo — configura prima un offset LNB/convertitore (Impostazioni ▸ Radio)
qo100-run-fast-spectral-tracker-every = Avvia il tracker spettrale veloce: ogni secondo cerca, nella finestra di parcheggio, i due lobi simmetrici del radiofaro e riporta dove si trova la portante
qo100-telemetry = TELEMETRIA
qo100-also-run-ao-40-uncoded = Avvia anche il decodificatore di frame non codificati AO-40 — parola di sincronizzazione, CRC e il testo di telemetria — con una lettura passo passo (portante → sincronizzazione → CRC) di quanto arriva ogni passaggio
qo100-auto = AUTO
qo100-auto-correct-let-tracker-adjust = Auto-correzione: lascia che il tracker regoli da solo l'offset del convertitore/LNB — un anello chiuso lento che, su una stima pulita e stabile, riporta il radiofaro su 10489.750 MHz e lo tiene lì mentre l'LNB deriva. Riapre il front-end ogni volta che agisce.
qo100-narrower-search-smaller-slice = Più stretta — cerca in una porzione più piccola
qo100-wider-when-beacon-isn-t = Più larga — per quando il radiofaro non viene trovato alla larghezza attuale
qo100-clear-mark = cancella il segno
qo100-drop-hand-placed-beacon-mark = Togli il segno del radiofaro posto a mano e torna alla lettura del decodificatore
qo100-lo = lo −
qo100-lo-2 = lo +
qo100-hi = hi −
qo100-hi-2 = hi +
qo100-radio-s-own-tuning-range = La gamma di sintonia propria di questa radio non arriva a 10489.750 MHz — serve un offset LNB/convertitore (Impostazioni ▸ Radio ▸ Convertitore) prima che questa finestra possa fare qualcosa.
qo100-open-settings-radio = Apri Impostazioni ▸ Radio
qo100-tune-10489-750-mhz = Sintonizza su 10489.750 MHz
qo100-tip-if-you-can-see = suggerimento: se vedi il radiofaro nella striscia ma non si aggancia, fai doppio clic su di esso — lo segna come 10489.750 MHz e permette ad APPLICA di correggere su di esso
qo100-tracker = TRACKER
qo100-no-twin-lobe-shape-parking = nessuna forma a doppio lobo nella finestra di parcheggio
qo100-shape = FORMA
qo100-drift = DERIVA
qo100-need-longer-run-estimates = — (servono più stime in sequenza)
qo100-cycles = CICLI
qo100-receiver = RICEVITORE
qo100-target = BERSAGLIO
qo100-measured = MISURATO
qo100-clicked = {"  "}(cliccato)
qo100-tracker-2 = {"  "}(tracker)
qo100-not-locked-yet = non ancora agganciato
qo100-converter-offset = OFFSET DEL CONVERTITORE
qo100-decode = DECODIFICA
qo100-decode-filling-first-frame-buffer = DECODIFICA — riempimento del primo buffer di frame…
qo100-decode-off-turn-telemetry-chip = DECODIFICA — disattivata. Attiva il chip TELEMETRIA (riga in alto) per la lettura portante → sincronizzazione → CRC.
qo100-apply-correction = {" "}APPLICA CORREZIONE{" "}
qo100-write-converter-lnb-offset-puts = Scrive l'offset del convertitore/LNB che porta il radiofaro dove hai cliccato su 10489.750 MHz, e riapre il ricevitore — la stessa breve interruzione che fa Impostazioni ▸ Radio ▸ Applica
qo100-write-converter-lnb-offset-puts-2 = Scrive l'offset del convertitore/LNB che porta la stima del tracker su 10489.750 MHz, e riapre il ricevitore — la stessa breve interruzione che fa Impostazioni ▸ Radio ▸ Applica
qo100-waiting-second-crc-valid-frame = In attesa di un secondo frame con CRC valido prima di proporre di scrivere questo — un solo aggancio potrebbe essere una coincidenza
qo100-write-corrected-converter-lnb-offset = Scrive l'offset del convertitore/LNB corretto e riapre il ricevitore — una breve interruzione, la stessa che fa Impostazioni ▸ Radio ▸ Applica
qo100-decoder-runs-receiving-station-readout = il decodificatore gira sulla stazione ricevente — la sua lettura non viene inviata ai client remoti
qo100-starting = avvio…
qo100-searching-first-window-fills-after = ricerca in corso — la prima finestra si riempie dopo circa 24 s, poi si ripete

## js8 (src/app/panels/js8.rs)

js8-multi = MULTI
js8-decode-every-js8-speed-not = Decodifica ogni velocità JS8, non solo quella su cui trasmetti. Le quattro velocità condividono la sotto-banda e sono quattro forme d'onda diverse, quindi senza questo una stazione su un'altra velocità è invisibile — e uno scambio tra due velocità non può avvenire affatto. Costa circa quattro volte la CPU in ricezione.
js8-hb-auto = HB AUTO
js8-turbo-does-not-beacon-local = Turbo non invia radiofari — è la velocità locale e VHF
js8-beacon-your-callsign-grid-every = Invia il tuo nominativo e il locatore ogni 15 minuti
js8-until-next-heartbeat = Fino al prossimo heartbeat
js8-setup = ⚙ CONFIGURA
js8-tx = ● TX
js8-heard = SENTITI
js8-nothing-heard-yet = — niente di sentito ancora —
js8-dxcc = DXCC
js8-band = BANDA
js8-grid = LOCATORE
js8-dupe = DOPPIO
js8-reply = RISPOSTA
js8-no-messages = — nessun messaggio —
js8-cq = {" "}CQ{" "}
js8-hb = {" "}HB{" "}
js8-clear = {" "}CANCELLA A{" "}
js8-forget-selected-station-composer-goes = Dimentica la stazione selezionata — il compositore torna a @ALLCALL
js8-already-addressing-allcall = Si sta già indirizzando a @ALLCALL
js8-stop = {" "}FERMA{" "}
js8-send = {" "}INVIA{" "}
js8-message = Messaggio…

## logbook (src/app/logbook.rs)

palette-hint = Vai a… una modalità, una banda, una finestra, un'impostazione, una memoria o una frequenza (14.074)
palette-nothing = nulla con questo nome
palette-group-window = Finestra
palette-group-settings = Impostazioni
palette-group-memory = Memoria
palette-group-tune = Sintonia
palette-tune-to = Sintonizza su { $mhz } MHz
palette-win-bands = Condizioni di banda
palette-win-awards = Premi
palette-win-windows = Elenco finestre
logbook-logbook = LOGBOOK
logbook-new-entry = + NUOVA VOCE
logbook-adif = ADIF
logbook-import = IMPORTA
logbook-import-qsos-adif-adi-adif = Importa QSO da un file ADIF (.adi o .adif)
logbook-new-qso = NUOVO QSO
logbook-edit-qso = MODIFICA QSO
logbook-call = Nominativo
logbook-lookup = CERCA
logbook-look-up-name-qth-grid = Cerca nome / QTH / locatore
logbook-grid = Locatore
logbook-freq-mhz = Freq MHz
logbook-mode = Modo
logbook-rst-sent = RST inviato
logbook-rst-rcvd = RST ricevuto
logbook-name = Nome
logbook-state = Stato
logbook-country = Paese
logbook-date-utc = Data UTC
logbook-time = Ora
logbook-pwr-w = Pot W
logbook-contest = Contest
logbook-s-sent = N.S. inviato
logbook-s-rcvd = N.S. ricevuto
logbook-comment = Commento
logbook-save = {" "}SALVA{" "}
logbook-cancel = ANNULLA
logbook-lotw = LoTW ✓
logbook-eqsl = eQSL ✓
logbook-card = QSL ✓
logbook-lotw-2 = LoTW ↑
logbook-eqsl-2 = eQSL ↑
logbook-qrz = QRZ ↑
logbook-hamqth = HamQTH ↑
logbook-club-log = Club Log ↑
logbook-wavelog = Wavelog ↑
logbook-delete-entry = Elimina questa voce
logbook-edit = MODIFICA
logbook-upload-qso-configured-logs = Invia questo QSO ai log configurati
logbook-no-qsos-yet-run-ft8 = nessun QSO ancora — avvia FT8/FT4 oppure aggiungi una voce manuale

## scanner (src/app/scanner.rs)

scanner-scanner = Scanner
scanner-work-through-stored-memory-channels = Scorri i canali di memoria memorizzati
scanner-work-through-slice-band-channel = Scorri una porzione di banda su una griglia di canali
scanner-stop = STOP
scanner-start = AVVIO
scanner-from = Da
scanner-step = Passo
scanner-mode = Modo
scanner-range-empty-high-edge-has = Quell'intervallo è vuoto — il limite superiore deve stare almeno un canale sopra quello inferiore.
scanner-skip-while-holding-pass-over = SALTA mentre è tenuto premuto per passare oltre quel canale da ora in poi
scanner-skipping = Salto
scanner-clear = CANCELLA
scanner-stop-passing-over-any-them = Smetti di passare oltre uno qualsiasi di essi
scanner-stop-passing-over-channel = Smetti di passare oltre questo canale
scanner-no-memory-channels-stored-yet = Nessun canale di memoria ancora — memorizzane alcuni nella finestra MEM.
scanner-skip-channel-pass-over = SALTA un canale per passarvi oltre
scanner-skip = SALTA
scanner-look-all-channels-fall-inside = Cerca tutti i canali che cadono dentro una finestra del ricevitore sulla stessa trasformata da cui è costruito il panadapter, e sintonizza solo quelli su cui c'è qualcosa. Un elenco su una banda costa allora una sintonia per giro, per quanto lungo sia, invece di un tempo di assestamento per canale. La scansione ascolta comunque ogni candidato prima di fermarsi, quindi ciò che la ferma non cambia — ma lo sweep misura tramite la FFT anziché tramite il filtro del ricevitore, quindi controlla la soglia se comincia a fermarsi su niente.
scanner-radio-hands-over-demodulated-audio = Questa radio consegna audio demodulato e non ha uno spettro proprio in cui cercare, quindi la sua scansione di memoria visita ogni canale comunque.
scanner-fast = VELOCE
scanner-read-list-off-spectrum-instead = leggi l'elenco dallo spettro invece di visitare ogni canale
scanner-folders = Cartelle
scanner-scan-every-folder-every-folder = Scansiona ogni cartella, e ogni cartella creata d'ora in poi. Scegli le cartelle per scansionare solo quelle.
scanner-unfiled = Senza cartella
scanner-nothing-filed-under-folders-you = Nulla è archiviato nelle cartelle che hai scelto.
scanner-stops = Si ferma a
scanner-use-receiver-s-own-squelch = Usa lo squelch proprio del ricevitore, così la scansione si ferma esattamente dove si aprirebbe l'audio
scanner-squelch-off-scan-will-stop = lo squelch è disattivato — la scansione si fermerà sulla prima cosa che esamina
scanner-channel-power-signal-has-reach = Potenza del canale che un segnale deve raggiungere per fermare la scansione
scanner-listens = Ascolta per
scanner-how-long-stay-candidate-before = Quanto restare su un candidato prima di giudicarlo. Sotto circa un decimo di secondo lo strumento di livello non si è assestato e i segnali deboli vengono persi
scanner-resumes = Riprende
scanner-carry-once-signal-drops = Prosegui appena il segnale cade
scanner-carry-after-fixed-time-regardless = Prosegui dopo un tempo fisso, in ogni caso
scanner-stay-until-you-press-next = Resta finché non premi SUCCESSIVO
scanner-how-long-stay = Quanto restare
scanner-how-long-wait-after-signal = Quanto attendere dopo che il segnale cade
scanner-move-don-t-stop-channel = Passa oltre, e non fermarti più su questo canale
scanner-next = SUCCESSIVO
scanner-move-now = Passa oltre ora

## adsb (src/app/panels/adsb.rs)

adsb-starting-ads-b-decoder = avvio del decodificatore ADS-B…
adsb-ads-b = ADS-B
adsb-t-1090-mhz-mode-s = Mode S a 1090 MHz
adsb-tune-receiver-1090-000-mhz = Sintonizza il ricevitore su 1090.000 MHz, il collegamento in discesa ADS-B mondiale. C'è un solo canale.
adsb-setup = CONFIGURA
adsb-timeouts-trail-length-how-far = Timeout, lunghezza della scia e fino a dove arrivano i vettori di velocità
adsb-aircraft = AEREI
adsb-nothing-heard-yet = niente di sentito ancora
adsb-ground = a terra
adsb-position-stale = posizione non aggiornata
adsb-gnss-altitude = Altitudine GNSS
adsb-ground-speed = velocità al suolo
adsb-vertical-rate = velocità verticale
adsb-first-heard = sentito per la prima volta
adsb-center = CENTRA
adsb-put-aircraft-middle-map = Metti questo aereo al centro della mappa
adsb-last-frame-accepted-aircraft = L'ultimo frame accettato da questo aereo
adsb-ads-b-setup = Configurazione ADS-B
adsb-drop-map-after = Rimuovi dalla mappa dopo
adsb-without-position-report = senza un rapporto di posizione
adsb-past-aircraft-comes-off-map = Oltre questo tempo l'aereo esce dalla mappa e la sua riga diventa grigia. Non viene sbiadito: un quadrato spento su una posizione vecchia è comunque un'affermazione su dove si trova un aereo, con lo stesso inchiostro di quelli veri.
adsb-drop-list-after = Rimuovi dall'elenco dopo
adsb-nothing-heard-all = senza aver sentito nulla
adsb-trail-length = Lunghezza della scia
adsb-points = {" "}punti
adsb-history-dots-behind-each-target = punti di storia dietro ogni bersaglio
adsb-speed-vector = Vettore di velocità
adsb-how-far-ahead-leader-line = fino a dove arriva la linea guida in avanti
adsb-one-minute-usual-radar-convention = Un minuto è la convenzione radar usuale: la linea è esattamente lunga quanto la distanza che l'aereo percorre in quel tempo, quindi due guide uguali sono due velocità uguali a qualunque zoom. Zero le disattiva.
adsb-track-most = Traccia al massimo
adsb-aircraft-2 = {" "}aerei
adsb-aircraft-ground-are-placed-against = Gli aerei a terra sono collocati rispetto alla posizione della stazione — un squitter di superficie non ha una decodifica univoca propria — quindi compila Il mio locatore nella configurazione dei modi digitali se l'aeroporto vicino non mostra nulla.
adsb-call = NOMINATIVO
adsb-icao = ICAO
adsb-v-s = V/S

## drm (src/app/drm.rs)

drm-waiting-receiver = in attesa del ricevitore…
drm-not-drm-set-mode-drm = Non in DRM. Imposta il modo su DRM su una radiodiffusione digitale in onde corte — la sintonia va sul centro del canale, non su una banda laterale.
drm-no-drm-signal-locked-decoder = Nessun segnale DRM agganciato. Il decodificatore ha bisogno di qualche secondo su una portante pulita: il DRM intreccia su 400 ms o 2 s prima che se ne possa leggere qualcosa.
drm-station-s-audio-codec-not = Il codec audio di questa stazione non viene decodificato qui. L'xHE-AAC richiede libfdk-aac nel sistema — non può essere incluso, perché la sua licenza e quella di questo programma sono incompatibili. Installalo (Debian/Ubuntu: libfdk-aac2, Arch: libfdk-aac, macOS: brew install fdk-aac, Windows: libfdk-aac-2.dll accanto a sdroxide.exe) e riavvia. Tutto il resto di questa schermata viene decodificato normalmente.
drm-quality = QUALITÀ
drm-last-minute = ultimo minuto
drm-waiting-locked-signal = in attesa di un segnale agganciato
drm-audio-frames-decoding-but-station = Frame audio in decodifica — ma il codec di questa stazione non può essere decodificato qui
drm-audio-frames-decoding = Frame audio in decodifica
drm-samples-reaching-decoder = Campioni che raggiungono il decodificatore
drm-time = ORA
drm-symbol-timing-recovered = Temporizzazione dei simboli recuperata
drm-frame = FRAME
drm-transmission-frames-found = Frame di trasmissione trovati
drm-fast-access-channel-what-transmission = Canale di accesso rapido — cos'è la trasmissione
drm-service-description-channel-what-services = Canale di descrizione dei servizi — quali sono i servizi
drm-audio = AUDIO
drm-constellation = COSTELLAZIONE
drm-waiting-decoded-symbols = in attesa dei simboli decodificati
drm-sampled-across-frame = campionato lungo il frame
drm-whole-frame = intero frame
drm-mode = MODO
drm-robustness-mode-channel-width-ground = Modo di robustezza e larghezza di canale. A è per un percorso a onda di superficie e porta il massimo; D è per un percorso a onda ionosferica molto disperso e porta il minimo.
drm-interleave = INTERLEAVING
drm-how-far-transmission-spreads-each = Quanto lontano la trasmissione distribuisce ogni frame nel tempo. Lungo resiste a evanescenze più profonde e richiede proporzionalmente più tempo per agganciare.
drm-protection = PROTEZIONE
drm-offset = OFFSET
drm-residual-sample-clock-error-against = Errore residuo del clock di campionamento rispetto al trasmettitore. Grande e costante significa che il riferimento del ricevitore è fuori, non la trasmissione.
drm-doppler = DOPPLER
drm-delay = RITARDO
drm-doppler-delay-spread-path-how = Dispersione Doppler e di ritardo del percorso — quanto velocemente si muove e quanto distanti arrivano i suoi echi.
drm-service = SERVIZIO
drm-decode-service-multiplex = Decodifica questo servizio del multiplex

## atchat (src/app/panels/atchat.rs)

atchat-starting-atchat-net-station = avvio della stazione AtCHAT NET…
atchat-atchat = ATCHAT
atchat-net-multi-station-keyboard-file = NET — tastiera multi-stazione e file
atchat-net = {" "}IN RETE{" "}
atchat-net-click-leave-state-kept = In rete — clicca per uscire (lo stato viene conservato; clicca di nuovo per rientrare)
atchat-rejoin = {" "}RIENTRA{" "}
atchat-off-net-click-rejoin = Fuori rete — clicca per rientrare
atchat-off-net = fuori rete
atchat-carrier = ● PORTANTE
atchat-another-station-transmitting-channel-busy = Un'altra stazione sta trasmettendo — il canale è occupato.
atchat-tx = ● TX
atchat-setup = ⚙ CONFIGURA
atchat-station-callsign-virtual-channel-address = Nominativo della stazione e indirizzo del canale virtuale
atchat-virtual-channel = CANALE VIRTUALE
atchat-work-net-over-tcp-endpoint = Lavora in rete tramite un endpoint TCP anziché la radio — un loopback compatibile con channel_server per lo sviluppo senza radio. Disattivato mette la stazione in aria.
atchat-roster = ELENCO
atchat-nobody-heard-yet = nessuno sentito ancora
atchat-chat = {" "}CHAT{" "}
atchat-common-channel-lines-here-go = Il canale comune — le righe qui vanno a tutti (ALL)
atchat-log = {" "}LOG{" "}
atchat-station-s-own-air-activity = L'attività in aria della stazione — elezione del master, invecchiamento dell'elenco, tentativi ARQ
atchat-no-activity-logged-yet = Nessuna attività registrata ancora.
atchat-no-messages-yet = Nessun messaggio ancora.
atchat-message-everyone-all = messaggio a tutti (ALL)
atchat-send = {" "}INVIA{" "}
atchat-transfers = TRASFERIMENTI
atchat-all = → ALL
atchat-sent-file-follows-active-chat = Un file inviato segue la scheda di chat attiva
atchat-send-file = {" "}INVIA FILE{" "}
atchat-send-file-image-over-air = Invia un file o un'immagine in aria, con CRC a blocchi e ARQ
atchat-nothing-flight = nulla in volo
atchat-images = IMMAGINI
atchat-no-images-received = Nessuna immagine ricevuta.


## rds (src/app/rds.rs)

rds-station = STAZIONE
rds-diagnostics = DIAGNOSTICA
rds-which-programme-type-table-read = Con quale tabella di tipo di programma leggere questa stazione. RDS si usa ovunque fuori dal Nord America; RBDS aggiunge il nominativo che il codice di identità di una stazione statunitense compone. Automatico segue il codice paese esteso che la stazione invia, e finché non ne arriva uno non compone alcun nominativo — gli intervalli di identità che leggerebbe sono condivisi con i codici RDS di nove paesi, quindi seleziona RBDS per dare un nome a una stazione che non invia un codice paese.
rds-waiting-receiver = in attesa del ricevitore…
rds-no-rds-station-not-every = Nessun RDS su questa stazione. Non ogni trasmettitore lo porta, e uno debole può portarlo senza che arrivi.
rds-rds-only-carried-wfm-broadcast = L'RDS è portato solo dalle stazioni di radiodiffusione WFM.
rds-holding = BLOCCATO
rds-block-sync-has-dropped-last = La sincronizzazione dei blocchi è caduta — questa è l'ultima stazione letta.
rds-identity = IDENTITÀ
rds-programme = PROGRAMMA
rds-described = DESCRITTO COME
rds-content = CONTENUTO
rds-music = Musica
rds-speech = Parlato
rds-traffic = TRAFFICO
rds-announcement-air-now = annuncio in onda ora
rds-carried-station = portato da questa stazione
rds-not-carried = non portato
rds-radio-text = TESTO RADIO
rds-station-clock = OROLOGIO DELLA STAZIONE
rds-country-code = CODICE PAESE
rds-also = ANCHE SU
rds-also-tagged = ANCHE CONTRASSEGNATO
rds-sync = SINCRO
rds-no-sync = NESSUNA SINCRO
rds-blocks-failed-their-check-plus = Blocchi che non hanno superato il controllo, più quelli che il correttore d'errore ha dovuto riparare. Le correzioni contano come errori di proposito: una cifra in salita è il primo segno che una stazione sta per smettere di decodificarsi.
rds-front-end-overloading = FRONT-END IN SOVRACCARICO
rds-converter-clipping-which-puts-distortion = Il convertitore sta saturando, il che mette distorsione su tutto il multiplex — compreso il sottoportante dati a 57 kHz. Abbassa il guadagno RF, oppure inserisci un attenuatore: l'RDS cede circa un decibel prima che si senta qualcosa, quindi una stazione che suona perfetta può essere ancora troppo forte perché i suoi dati sopravvivano.
rds-group-types = TIPI DI GRUPPO
rds-recent-groups-newest-last = GRUPPI RECENTI — il più recente in fondo
rds-nothing-decoded-yet = nulla di decodificato ancora

## winlink (src/app/winlink.rs)

winlink-mail = POSTA
winlink-abort = ANNULLA
winlink-stop-session-progress = Ferma la sessione in corso
winlink-connect = COLLEGA
winlink-compose = SCRIVI
winlink-connecting = connessione…
winlink-not-connected-yet = non ancora collegato
winlink-loading = caricamento…
winlink-date = DATA
winlink-from = DA
winlink-subject = OGGETTO
winlink-no-subject = (nessun oggetto)
winlink-delete-message = Elimina questo messaggio
winlink-arch = ARCH
winlink-move-archive = Sposta nell'archivio
winlink-select-message = seleziona un messaggio
winlink-reply = RISPONDI
winlink-delete = ELIMINA
winlink-to = A
winlink-cc = Cc
winlink-subject-2 = Oggetto
winlink-callsign-smtp-someone-example-org = Un nominativo, oppure SMTP:qualcuno@example.org per la posta internet. Separa più destinatari con virgole.
winlink-file-outbox = METTI IN POSTA IN USCITA
winlink-discard = SCARTA
winlink-filed-messages-go-out-next = i messaggi messi in posta in uscita partono alla prossima sessione
winlink-mail-log = REGISTRO POSTA
winlink-session-running = sessione in corso…
winlink-no-session-yet = nessuna sessione ancora

## wspr (src/app/panels/wspr.rs)

wspr-receptions = RICEZIONI
wspr-save-wspr-reception-list-csv = Salva l'elenco delle ricezioni WSPR in CSV
wspr-wspr = WSPR
wspr-weak-signal-propagation-beacon = radiofaro di propagazione a segnali deboli
wspr-time-left-two-minute-slot = Tempo rimasto in questo slot da due minuti. Alla fine la registrazione passa al decodificatore e cade la successiva decisione di trasmissione.
wspr-tx = ● TX
wspr-decoding = decodifica…
wspr-band = Banda
wspr-last-slot = Ultimo slot
wspr-nothing-heard = nulla di sentito
wspr-receive-only = solo ricezione
wspr-next-slot = Prossimo slot
wspr-waiting-radio-take-change = in attesa che la radio accetti il cambio…
wspr-hop-next = Salta al prossimo
wspr-transmit = TRASMETTI
wspr-receive-only-2 = Solo ricezione.
wspr-power = POTENZA
wspr-what-you-actually-radiate = ciò che irradi davvero
wspr-goes-out-message-everyone-who = Questo viene inviato nel messaggio, e chiunque ti senta giudica il percorso da esso — quindi una cifra ottimistica qui rende sbagliate le loro misure oltre alle tue.
wspr-roam = ROAM
wspr-pick-different-offset-inside-200 = Scegli un offset diverso dentro la finestra da 200 Hz per ogni trasmissione. È la convenzione: duecento hertz condivisi da tutti funzionano solo se nessuno si ferma nel mezzo. Disattivato resta dove si trova il cursore.
wspr-band-hop = SALTO DI BANDA
wspr-move-dial-band-band-between = Sposta la sintonia da banda a banda tra uno slot e l'altro, così un solo ricevitore campiona l'intero spettro anziché una sua porzione. Se giri tu il VFO la funzione si mette in pausa — premi questo due volte per riprendere.
wspr-upload = INVIA
wspr-send-what-station-decodes-wsprnet = Invia a wsprnet.org ciò che questa stazione decodifica. Riportare ciò che senti è ciò che fa di un ricevitore WSPR una parte della rete; non mette nulla in aria.
wspr-who-heard-me = CHI MI HA SENTITO
wspr-ask-wsprnet-org-every-few = Chiedi a wsprnet.org ogni pochi minuti quali stazioni hanno decodificato questa. WSPR non prevede alcuna forma di conferma, quindi è l'unico modo in cui un radiofaro impara qualcosa della propria portata.
wspr-bands = BANDE
wspr-none-selected-hopping-has-nowhere = nessuna selezionata — il salto non ha dove andare
wspr-set-your-callsign-grid-general = Imposta il tuo nominativo e il locatore nella scheda Generale delle Impostazioni prima di trasmettere.

## packet (src/app/panels/packet.rs)

packet-monitor = MONITOR
packet-busy = OCCUPATO
packet-frames-arrived-but-failed-their = Frame arrivati che non hanno superato la sequenza di controllo — una collisione, un'attenuazione, o un segnale troppo debole per essere letto.
packet-setup = ⚙ CONFIGURA
packet-station-callsign-speed-tx-delay = Nominativo di stazione, velocità, ritardo TX, il percorso dei digipeater, il radiofaro e il server KISS
packet-starting-packet-modem = avvio del modem a pacchetto…
packet-nothing-heard-yet = niente di sentito ancora
packet-terminal = TERMINALE
packet-connected = {" "}COLLEGATO{" "}
packet-calling = {" "}CHIAMATA{" "}
packet-idle = {" "}INATTIVO{" "}
packet-extended-sequence-numbers-window-up = Numeri di sequenza estesi — una finestra fino a 127 frame.
packet-frames-sent-not-yet-acknowledged = Frame inviati e non ancora confermati dall'altro capo.
packet-retries-against-link-s-limit = Tentativi rispetto al limite di questo collegamento. Un conteggio che sale mentre i frame non confermati restano fermi è ciò che un percorso in attenuazione appare da questa parte.
packet-mail-window-using-link = La finestra POSTA sta usando il collegamento.
packet-one-radio-one-channel-one = Una radio, un canale, un collegamento. Una sessione Winlink e questo terminale sono due modi di usarlo, e chi lo chiede per primo lo ottiene.
packet-digipeaters-nearest-first-separated-commas = Digipeater, il più vicino per primo, separati da virgole — OE3XLR-1,OE3XMS-1. Lascialo vuoto per una stazione che senti direttamente, oppure per usare il percorso predefinito delle impostazioni pacchetto.
packet-disconnect = {" "}DISCONNETTI{" "}
packet-hang-up-properly-wait-far = Riattacca come si deve, e attendi che l'altro capo sia d'accordo. Cambiare modo invece termina la sessione senza avvisare nessuno.
packet-connect = {" "}COLLEGA{" "}
packet-mail-window-has-link-finish = La finestra POSTA ha il collegamento. Finisci o ferma prima quella sessione.
packet-station-has-no-callsign-yet = Questa stazione non ha ancora un nominativo — impostane uno in CONFIGURA, sopra il riquadro del monitor. Non trasmette nulla finché non è impostato.
packet-call-station-connected-mode-node = Chiama questa stazione in modo collegato — un nodo, un BBS, o un altro operatore.
packet-needs-callsign-call = Serve un nominativo da chiamare.
packet-clear = {" "}CANCELLA{" "}
packet-empty-transcript-link-untouched-connected = Svuota la trascrizione. Il collegamento non viene toccato — una stazione collegata resta collegata.
packet-not-connected-type-callsign-above = Non collegato. Scrivi un nominativo sopra e premi COLLEGA per chiamare un nodo o un BBS.
packet-type-here = scrivi qui
packet-not-connected = non collegato
packet-send = {" "}INVIA{" "}
packet-send-line-node-bbs-reads = Invia la riga. Un nodo o un BBS legge una riga alla volta.

## fsq (src/app/panels/fsq.rs)

fsq-speed = Velocità
fsq-call = Chiamata
fsq-callsign-directed-fsqcall-messages-defaults = Nominativo per i messaggi diretti (FSQCALL); il predefinito è il tuo nominativo
fsq-tx = ● TX
fsq-contacts = CONTATTI
fsq-heard = SENTITI
fsq-none = — nessuno —
fsq-allcall = ALLCALL
fsq-image = IMMAGINE
fsq-clear = CANCELLA
fsq-forget-pictures-received-so-far = Dimentica le immagini ricevute finora
fsq-send-image = Invia immagine…
fsq-right-click-remove = Clic destro per rimuovere
fsq-remove-picture = Rimuovi questa immagine
fsq-listening = — in ascolto —
fsq-message = Messaggio…
fsq-send = {" "}INVIA{" "}
fsq-call-cq = {" "}CHIAMA CQ{" "}
fsq-heard-2 = {" "}? sentito{" "}
fsq-clear-2 = {" "}CANCELLA{" "}
fsq-fsq-contacts = Contatti FSQ
fsq-add = Aggiungi

## publicsdr (src/app/publicsdr.rs)

publicsdr-not-stated = non indicato
publicsdr-public-sdrs = SDR PUBBLICI
publicsdr-available = DISPONIBILI
publicsdr-hide-receivers-are-full-ones = Nascondi i ricevitori che sono pieni, e quelli il cui operatore non ha aperto canali ad app diverse dal browser
publicsdr-band = IN BANDA
publicsdr-only-receivers-cover-current-dial = Solo i ricevitori che coprono la frequenza di sintonia attuale
publicsdr-low-bw = BASSA LARGH.
publicsdr-map-cities = CITTÀ
publicsdr-map-cities-tip = Città su questa mappa, sì o no.
publicsdr-view-list = ELENCO
publicsdr-view-map = MAPPA
publicsdr-view-map-tip = Gli stessi ricevitori su una mappa del mondo. Clic su un punto per scegliere tutti i ricevitori sotto di esso.
publicsdr-map-pick = Clic su un punto per scegliere i ricevitori sotto di esso.
publicsdr-map-ghost = { $name }: ora non è nell'elenco — visto qui l'ultima volta il { $seen }
publicsdr-map-names = NOMI
publicsdr-map-names-tip = Mostra i nomi dei ricevitori sulla mappa. Disattivato: solo quelli scelti hanno il nome.
publicsdr-check-online = ONLINE
publicsdr-check-silent = MUTO
publicsdr-check-waiting = VERIFICO…
publicsdr-check-no-answer = Nessuna risposta entro 20 secondi.
publicsdr-take-spyserver-low-bandwidth-shape = Usa uno SpyServer nella sua forma a bassa larghezza di banda: una finestra I/Q stretta che segue la sintonia più la vista di banda propria del server, invece di megabit di I/Q a banda larga. Nessun effetto su un KiwiSDR, che ha una sola forma.
publicsdr-refresh = ⟳ AGGIORNA
publicsdr-fetch-both-directories-again = Scarica di nuovo entrambe le directory
publicsdr-name-place-antenna-band = nome, luogo, antenna, banda
publicsdr-clear-search = Cancella la ricerca
publicsdr-fetching-receiver-lists = Recupero degli elenchi dei ricevitori…
publicsdr-machine-radio-attached-does-not = La macchina a cui è collegata questa radio non risponde alle domande sui dispositivi, quindi non può nemmeno scaricare gli elenchi.
publicsdr-no-match = nessuna corrispondenza
publicsdr-nothing-matches-try-turning-filter = nulla corrisponde — prova a riattivare un filtro

## hd (src/app/hd.rs)

hd-hd-radio = HD Radio
hd-waiting-receiver = in attesa del ricevitore…
hd-not-hd-radio-set-mode = Non in HD Radio. Imposta il modo su HD Radio su una portante di radiodiffusione FM con le sidecar digitali — la sintonia va sul centro della portante analogica, non accanto ad essa.
hd-no-hd-radio-lock-digital = Nessun aggancio HD Radio. Le bande laterali digitali sono trasmesse circa 20 dB sotto la portante analogica, quindi una stazione ascoltabile può essere comunque troppo rumorosa per essere decodificata: il decodificatore ha bisogno di qualche secondo su una pulita.
hd-sync = SINCRO
hd-ofdm-frame-timing-recovered = Temporizzazione dei frame OFDM recuperata
hd-audio = AUDIO
hd-audio-frames-decoding = Frame audio in decodifica
hd-quality = QUALITÀ
hd-lower = INFERIORE
hd-upper = SUPERIORE
hd-last-minute = ultimo minuto
hd-waiting-locked-signal = in attesa di un segnale agganciato
hd-mer-l = MER L
hd-modulation-error-ratio-lower-digital = Rapporto di errore di modulazione della banda laterale digitale inferiore.
hd-mer-u = MER U
hd-modulation-error-ratio-upper-digital = Rapporto di errore di modulazione della banda laterale digitale superiore.
hd-cber = CBER
hd-channel-bit-error-ratio-after = Rapporto di errore sui bit di canale dopo il codice interno. Sotto circa 1e-3 il codice esterno può correggerlo; sopra, l'audio comincia a spezzarsi.
hd-offset = OFFSET
hd-residual-carrier-frequency-offset-large = Offset residuo di frequenza della portante. Grande e costante significa che il riferimento del ricevitore è fuori, non la trasmissione.
hd-psmi = PSMI
hd-primary-service-mode-indicator-which = Indicatore primario del modo di servizio: quale dei modi di servizio ibridi FM la stazione sta trasmettendo.
hd-programme = PROGRAMMA
hd-restricted-access = {" "}— accesso limitato

## bands (src/app/bands.rs)

bands-bands = BANDE
bands-daylight = ☀ GIORNO
bands-night = ☾ NOTTE
bands-no-forecast-yet-fetching-hamqsl = · nessuna previsione ancora — recupero da HAMQSL.com
bands-forecast-download-off = · download della previsione disattivato (Impostazioni → Interfaccia)
bands-band = BANDA
bands-condx = CONDX
bands-paths = PERCORSI
bands-reach = PORTATA
bands-best = MIGLIORE
bands-condx-global-forecast-rest-what = CONDX è una previsione globale; il resto è ciò che è stato davvero sentito. Una riga vuota significa che non è stato decodificato nulla — il che può voler dire che la banda era chiusa, o solo che non c'era nessuno.
bands-ibp-beacons = RADIOFARI IBP
bands-meteor-showers = PIOGGE DI METEORE
bands-peak = PICCO

## spots (src/app/spots.rs)

spots-pota = POTA
spots-sota = SOTA
spots-freedv = FREEDV
spots-spots = SPOT
spots-longwave-shortwave-broadcast-stations-air = Stazioni di radiodiffusione in onde lunghe e corte in onda ora
spots-view = IN VISTA
spots-only-spots-inside-panadapter-span = Solo gli spot entro l'ampiezza del panadapter
spots-setup = ⚙ CONFIGURA
spots-feeds-lookup-upload-settings = Flussi, ricerca e impostazioni di invio
spots-call-station-site-frequency = nominativo, stazione, sito, frequenza
spots-clear-search = Cancella la ricerca
spots-no-match = nessuna corrispondenza
spots-no-spots-enable-feed-setup = nessuno spot — attiva un flusso in ⚙ CONFIGURA
spots-nothing-matches-search = nulla corrisponde alla ricerca

## ism (src/app/ism.rs)

ism-ism-devices = DISPOSITIVI ISM
ism-decoding = DECODIFICA
ism-needs-wideband-iq-source-cat = Richiede una sorgente IQ a banda larga; un apparato CAT invia solo audio
ism-how-far-burst-must-stand = Di quanto un burst deve emergere sopra il rumore di fondo proprio del canale prima di essere decodificato
ism-give-each-band-width-normally = Dai a ogni banda la larghezza di cui di solito ha bisogno — un quarto di megahertz per le bande OOK a 315, 345 e 433 MHz, un megahertz intero per 868 e 915.
ism-several-hundred-more-device-decoders = Altre centinaia di decodificatori di dispositivi, dal progetto rtl_433, incluse. Legge i dispositivi OOK a 433 MHz che i decodificatori propri di sdroxide non raggiungono affatto. Dove entrambi possono leggere un dispositivo, rtl_433 conosce molte più varianti e prende il sopravvento.
ism-t-1-your-own = 1 proprio
ism-device-decoders-registered-how-many = Decodificatori di dispositivi registrati, quanti vengono dal tuo rtl433_flex.conf, e quante trasmissioni sono state decodificate da quando è partito.
ism-reload-decoders = RICARICA DECODIFICATORI
ism-re-read-rtl433-flex-conf = Rilegge rtl433_flex.conf dopo averlo modificato. I dispositivi già sentiti restano nell'elenco.
ism-rtl-433-knows-more-variants = rtl_433 conosce più varianti di queste rispetto ai decodificatori propri di sdroxide, quindi le legge mentre ascolta qui. Seleziona un'altra banda e tornano a prevalere quelli propri di sdroxide.
ism-decoder-your-rtl433-flex-conf = Un decodificatore nel tuo rtl433_flex.conf è stato rifiutato. Gli altri sono stati caricati. Correggi la riga, poi premi RICARICA DECODIFICATORI.
ism-waiting-decoder = in attesa del decodificatore…
ism-where-decoder-actually-listening-follows = Dove il decodificatore sta davvero ascoltando. Segue il centro hardware del ricevitore, non il VFO — su un front-end largo i due possono distare molto, ed è la finestra a decidere quali canali sono raggiungibili.
ism-transmissions-gate-opened-how-many = Le trasmissioni su cui il cancello si è aperto, e quante hanno prodotto un frame valido. Molti burst e nessuna decodifica significa che la banda è occupata da dispositivi che sdroxide non sa ancora leggere. Nessuno affatto significa che nulla raggiunge la soglia — abbassala, oppure verifica che la sintonia sia sulla banda.
ism-nothing-heard-yet = niente di sentito ancora
ism-click-tune-receiver =
    {""}

    Clicca per sintonizzare il ricevitore su di esso

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

navtex-messages = MESSAGGI
navtex-sync = SINCRO
navtex-character-phase-locked-signal-being = La fase dei caratteri è agganciata — si sta leggendo un segnale.
navtex-no-character-phase-yet-no = Nessuna fase dei caratteri ancora: nessun segnale, oppure non questo.
navtex-characters-whose-first-copy-was = Caratteri la cui prima copia era corrotta e sono stati presi dalla ripetizione cinque slot dopo, e quelli in cui entrambe erano errate. Un carattere NAVTEX è inviato due volte; questa è tutta la sua correzione d'errore.
navtex-swap-mark-space-tones-signal = Scambia i toni di marcatura e spazio, per un segnale ricevuto sull'altra banda laterale. Disattivato è la banda laterale superiore sulla frequenza del canale, che è ciò che indica ogni istruzione di sintonia pubblicata per il servizio.
navtex-starting-navtex-receiver = avvio del ricevitore NAVTEX…
navtex-nothing-yet-station-transmits-ten = ancora nulla — una stazione trasmette per dieci minuti ogni quattro ore
navtex-cut-short = interrotto
navtex-message = MESSAGGIO
navtex-time-day-stated-message-utc = Ora del giorno indicata nel messaggio (UTC): solo visualizzazione
navtex-receiving = RICEZIONE
navtex-monitor = MONITOR
navtex-everything-decoded-message-not = tutto ciò che è decodificato, messaggio o no

## rfpaint (src/app/panels/rf_paint.rs)

rfpaint-rf-paint = RF PAINT
rfpaint-scan-speed = Velocità di scansione
rfpaint-how-fast-text-image-scanned = Quanto velocemente il testo o l'immagine viene scansionata sulla waterfall. Più basso è più lento e più leggibile; 100% = velocità base, 25% (centro) è il predefinito.
rfpaint-abort = Annulla
rfpaint-text-paint = TEXT PAINT
rfpaint-type-text-paint = Scrivi il testo da dipingere…
rfpaint-preview-waterfall = ANTEPRIMA WATERFALL
rfpaint-type-text-preview = scrivi del testo per l'anteprima
rfpaint-transmit = {"  "}TRASMETTI{"  "}
rfpaint-image-paint = IMAGE PAINT
rfpaint-no-image-loaded = nessuna immagine caricata
rfpaint-load-image = Carica immagine…
rfpaint-load-image-preview = carica un'immagine per l'anteprima

## hfdl (src/app/hfdl.rs)

hfdl-listen = ASCOLTA
hfdl-switches-engine-s-hfdl-downconverter = Attiva o disattiva il convertitore a discesa e il decodificatore HFDL del motore. La decodifica costa una corsia da 24 kHz e un thread di lavoro, che la finestra sia aperta o no.
hfdl-channel-assigned-frequency-listen-anything = Il canale (la frequenza assegnata) su cui ascoltare. Va bene qualunque punto della banda — le stazioni HFDL trasmettono da 2,8 a 22 MHz — contro una corsia fissa da 24 kHz centrata qui.
hfdl-save-hfdl-decode-log-file = Salva il registro delle decodifiche HFDL su un file
hfdl-riverhead-northern-atlantic-north-america = Riverhead (Atlantico settentrionale / Nord America) — il predefinito, e il canale su cui il decodificatore è stato validato.
hfdl-hfdl-assigned-frequency = Una frequenza assegnata HFDL.
hfdl-one-your-saved-channels-click = Uno dei tuoi canali salvati — clicca per sintonizzare, clic destro per rimuovere.
hfdl-hfdl-selected-but-decoding-off = HFDL è selezionato ma la decodifica è disattivata — premi ASCOLTA qui sopra
hfdl-decodes = DECODIFICHE
hfdl-nothing-decoded-yet-ground-station = Nulla di decodificato ancora — il squitter di una stazione di terra si ripete ogni ~32 s una volta che è nel canale.
hfdl-no-decode-matches-filter = Nessuna decodifica corrisponde al filtro.
hfdl-decoding = DECODIFICA
hfdl-decoding-off = DECODIFICA DISATTIVATA
hfdl-dbfs = — dBFS

## rade (src/app/panels/rade.rs)

rade-rade = RADE
rade-freedv-v1-digital-voice = Voce digitale FreeDV V1
rade-tx = ● TX
rade-mute-analog = MUTA ANALOGICO
rade-mute-demodulated-audio-so-only = Silenzia l'audio demodulato, così si sente solo il parlato decodificato. Altrimenti il segnale grezzo passa ogni volta che il modem non ha nulla da riprodurre — quel fruscio è il modo di individuare una trasmissione prima che si sincronizzi, quindi lascialo disattivato durante l'accordo.
rade-sync = SINCRO
rade-how-far-received-signal-sits = Di quanto il segnale ricevuto si trova lontano da dove il modem se lo aspetta. Valori grandi si decodificano comunque — il ciclo di acquisizione li segue — ma spostare un po' la sintonia per portarlo vicino allo zero dà il margine migliore.
rade-snr = SNR —
rade-callsign-station-put-end-over =
    Il nominativo che la stazione ha inserito nel suo frame di fine trasmissione — l'unico punto di una trasmissione RADE in cui ne viene inviato uno. È anche ciò che viene riportato a FreeDV Reporter, quindi una stazione che senti compare su qso.freedv.org come sentita da te.

    Trattenuto per un minuto dopo che il segnale cala, e scartato quando inizi una trasmissione tua: indica la stazione sulla frequenza, non ogni stazione che il ricevitore abbia mai sentito.
rade-stop-talking = SMETTI DI PARLARE
rade-talk-ptt = PARLA (PTT)
rade-open-close-rade-over-modem = Apre o chiude una trasmissione RADE. Il modem ha bisogno di circa 120 ms di parlato prima che parta il primo frame, e invia un frame di fine trasmissione quando smetti, quindi la trasmissione prosegue un po' oltre il pulsante.
rade-rade-v1-occupies-roughly-1060 = RADE V1 occupa circa 1060–1880 Hz della banda passante — banda laterale superiore, oppure inferiore su 160, 80 e 40 m, dove RADE mantiene la pratica in fonia e sdroxide commuta per te. Metti il segnale dentro la banda ombreggiata sulla waterfall; da lì il modem lo trova.

## frame (src/app/frame.rs)

frame-acknowledge = Conferma
frame-dismiss = Chiudi
frame-get-sdroxide-com = Scaricalo da sdroxide.com
frame-reconnecting = Riconnessione…
frame-not-connected = Non collegato.
frame-reconnect-now = Riconnetti ora
frame-transmit-lockout-disabled = ⚠ BLOCCO DI TRASMISSIONE DISATTIVATO
frame-engine-was-started-oob-tx = Questo motore è stato avviato con --oob-tx. Il blocco sulle bande radioamatoriali è disattivato: attiverà il trasmettitore su qualunque frequenza l'hardware supporti.
frame-transmitting-outside-your-licence-offence = Trasmettere fuori dalla propria licenza è un reato in ogni paese che ne rilascia una. Continua solo se sei autorizzato a usare le frequenze su cui stai per attivare — una licenza MARS/CAP o commerciale, un permesso sperimentale, oppure un carico fittizio.
frame-i-understand = {"  "}HO CAPITO{"  "}
frame-restart-without-oob-tx-put = Riavvia senza --oob-tx per ripristinare il blocco.

## smeter (src/widgets/smeter.rs)

smeter-alc-how-hard-transmitter-being = ALC — quanto è pilotato il trasmettitore, in percentuale della massima potenza che può emettere. 100 % è il fondo scala, non la piena potenza in watt: quanto valga in antenna dipende dalla radio e dal Pilotaggio.
smeter-radio-reports-no-forward-power =
    {""}

    Questa radio non riporta la potenza diretta, quindi la cifra accanto all'ALC è di nuovo la lettura dell'ALC anziché watt.
smeter-no-swr-radio-has-no =
    {""}

    Nessun ROS: questa radio non ha un ponte con cui misurarlo, e sdroxide non ne inventerà uno — può mostrare soltanto ciò che la radio stessa riporta. Un apparato con misuratore ROS (via CAT, TCI o collegamento LAN) e una scheda HPSDR lo compilano entrambi; un normale trasmettitore SDR no.
smeter-no-signal-report-yet-receiver = Nessun rapporto di segnale ancora — il ricevitore non ha fornito una lettura dello strumento.
smeter-s9-73-dbm-each-s =
    {""}

    S9 è −73 dBm e ogni unità S sotto di esso vale 6 dB. La cifra in dBm è onesta quanto la calibrazione del front-end — un ricevitore non calibrato riporta dBFS con un'etichetta dBm.
smeter-ovld-converter-into-rails-so =
    {""}

    OVLD: il convertitore è arrivato ai suoi limiti, quindi il livello mostrato sottostima il segnale. Riduci il guadagno del front-end.
smeter-table-held = , tabella bloccata
smeter-ps = PS --

## multi (src/multi.rs)

multi-tx = ● TX
multi-mute-radio-s-audio = Silenzia l'audio di questa radio
multi-show-radio-here = Mostra questa radio qui
multi-show-on-map = Mostra sulla mappa
multi-add-radio-computer-settings-radio = Aggiungi una radio su questo computer (per aggiungerne una a una stazione, usa Impostazioni → Radio)
multi-add-radio = Aggiungi una radio
multi-take-radio-off-row = Togli questa radio da questa riga. Continua a funzionare; gli anelli la spengono.
multi-open-pane = Apri un altro riquadro con una radio che non è sullo schermo
multi-pick-radio = Radio ▼
multi-pick-radio-tip = Metti una radio in questa riga. Gli anelli indicano le radio con il collegamento aperto.
multi-new-radio = Nuova radio…
multi-asked-station-another-radio = Chiesta alla stazione un'altra radio…
multi-client-cannot-open-connection = Questo client non può aprire una connessione.

## net (src/app/net.rs)

net-fail = ERRORE
net-will-retry = {" "}(riprova)
net-full-resync-downloading = Risincronizzazione completa: download in corso…
net-downloading = Download in corso…
net-full-resync-log-already-matches = Risincronizzazione completa: il log corrisponde già a Wavelog
net-full-resync-review-changes-below = Risincronizzazione completa: controlla le modifiche qui sotto
net-full-resync-discarded-nothing-changed = Risincronizzazione completa scartata; nulla è stato modificato
net-adif-import-failed-file-could = Importazione ADIF non riuscita: il file non può essere letto
net-channel-import-failed-file-could = Importazione dei canali non riuscita: il file non può essere letto
net-channel-import-nothing-read-chirp = Importazione dei canali: nulla da leggere — un file CSV di CHIRP inizia con una riga di intestazione che nomina le colonne, una delle quali deve essere Frequency

## widgets (src/app/panels/widgets.rs)

widgets-free-text = testo libero
widgets-hashed-callsign-not-yet-heard = nominativo con hash, non ancora sentito per intero
widgets-entity-unknown = entità sconosciuta
widgets-band = questa banda
widgets-new-entity-never-worked-any = Nuova entità — mai lavorata, su nessuna banda
widgets-new-grid-square = Nuovo quadrato locatore
widgets-not-worked-before = Non lavorato prima
widgets-worked-before-but-not-band = Lavorato prima, ma non su questa banda
widgets-call-queue = Nella coda delle chiamate
widgets-image = Immagine

## pi4 (src/app/panels/pi4.rs)

pi4-receptions = RICEZIONI
pi4-save-pi4-reception-list-csv = Salva l'elenco delle ricezioni PI4 in CSV
pi4-next-generation-beacon = Radiofaro di nuova generazione
pi4-time-left-one-minute-beacon = Tempo rimasto in questo ciclo di radiofaro da un minuto. Il messaggio PI4 viene cercato quando la finestra ha avuto il tempo di riempirsi oltre il punto in cui finirebbe.
pi4-decoding = decodifica…
pi4-band = Banda
pi4-last-cycle = Ultimo ciclo
pi4-nothing-heard = nulla di sentito
pi4-receive-only-tune-so-beacon = Solo ricezione. Sintonizza in modo che l'identificazione CW del radiofaro cada a 800 Hz audio — la convenzione della rete stessa — e i quattro toni PI4 cadano dove questo decodificatore li cerca per impostazione predefinita.

## awards (src/app/awards.rs)

awards-awards = PREMI
awards-band = Banda
awards-all = Tutte
awards-dxcc = DXCC
awards-grids = Locatori
awards-worked-all-states = Worked All States
awards-cq-zones-waz = Zone CQ (WAZ)
awards-dxcc-entities = Entità DXCC

## acars (src/app/panels/acars.rs)

acars-acars = ACARS
acars-audio-decoder-s-passband = Audio nella banda passante del decodificatore.
acars-blocks-framed-blocks-whose-check = Blocchi incorniciati, e blocchi la cui sequenza di controllo è fallita.
acars-starting-acars-receiver = avvio del ricevitore ACARS…
acars-channel = CANALE
acars-tune-dial-here = Sintonizza qui la sintonia.
acars-nothing-yet-acars-quiet-between = Ancora nulla. ACARS è silenzioso tra un aereo e l'altro — lascialo su un canale e aspetta un burst.
acars-check-failed = controllo fallito
acars-block-check-sequence-did-not = La sequenza di controllo del blocco non corrisponde — il testo potrebbe essere sbagliato.
acars-no-text = (nessun testo)

## login (src/login.rs)

login-sdr-oxide = SDR OXIDE
login-username = Nome utente
login-password = Password
login-remember-device = Ricorda su questo dispositivo
login-signs-without-asking-next-time = Accede senza chiedere la volta successiva, e fa entrare la scheda della vista 3D senza chiedere affatto. La password viene conservata in chiaro, quindi lascia questo disattivato su una macchina che usano altre persone.
login-keeps-password-device-clear = Conserva la password su questo dispositivo, in chiaro.
login-remember-station = Ricorda questa stazione
login-remember-me = RICORDAMI
login-station-keeps-sign-in = È la stazione a conservare il tuo accesso, non questa pagina: niente da compilare sulle altre radio e niente da dimenticare quando esci.
login-not-this-time = Non questa volta: il browser lo dimentica alla chiusura, quindi ti verrà richiesto nella prossima sessione e sul dispositivo non resta nulla di questo accesso.
login-checking = {" "}VERIFICA…{" "}
login-sign = {" "}ACCEDI{" "}
login-wrong-password-shuts-server-s = Una password sbagliata chiude la porta del server per alcuni secondi, quindi può volerci un momento.

## help (src/help.rs)

help-find = CERCA
help-find-manual = cerca nel manuale
help-previous-match-shift-enter-shift = Risultato precedente (Maiusc+Invio, Maiusc+F3)
help-next-match-enter-f3 = Risultato successivo (Invio, F3)
help-no-matches = nessun risultato
help-clear-search-esc = Cancella la ricerca (Esc)
help-sdroxide-manual = MANUALE DI SDROXIDE
help-contents = INDICE

## aismap (src/ais_map.rs)

aismap-t-1-station-heard-waiting-report = 1 stazione sentita — in attesa che riporti una posizione
aismap-double-click-reframe = doppio clic per reinquadrare
aismap-zoom-out-wheel-pinch-do = Zoom indietro — anche la rotella o il pizzico fanno questo
aismap-zoom-about-middle-chart = Zoom avanti sul centro della carta
aismap-frame-everything-being-tracked-again = Inquadra di nuovo tutto ciò che viene tracciato — anche un doppio clic lo fa

## specview (src/widgets/spectrum_view.rs)

specview-waiting-spectrum = in attesa dello spettro…
specview-hounds = HOUNDS

## spectrum (src/app/spectrum.rs)

spectrum-window-has-no-gpu-renderer = Questa finestra non ha un renderer GPU, quindi la waterfall resta alla sua larghezza standard.
spectrum-machine-drawing-without-gpu-every = Questa macchina disegna senza GPU — ogni colonna della waterfall è lavoro del processore, che lo condivide già con la radio.
spectrum-window-renders-through-opengl-which = Questa finestra disegna tramite OpenGL, che è il percorso di compatibilità di sdroxide — un Raspberry Pi, un vecchio chip grafico, o un browser senza WebGPU. Lì una waterfall più larga non vale il frame rate.
spectrum-wider-than-renderer-can-draw = Più larga di quanto questo renderer possa disegnare.

## macros (src/app/panels/macros.rs)

macros-nocall = NOCALL
macros-each-button-sends-whole-text = Ogni pulsante invia tutto il suo testo in una volta. F2–F10 premono i primi nove, purché nulla sullo schermo abbia la tastiera.
macros-remove-button = Rimuovi questo pulsante

## gridtracker (src/app/grid_tracker.rs)

gridtracker-grid-tracker = GRID TRACKER
gridtracker-shade-squares-heard-live-decode = Colora i quadrati sentiti nell'elenco delle decodifiche in tempo reale, non solo quelli nel log — ciò che è in aria ora, in ciano sotto i quadrati lavorati.
gridtracker-drag-pan-wheel-zoom-amber = Trascina per spostarti, rotella per lo zoom. Ambra = lavorato, verde = confermato, ciano = sentito.

## remote (src/remote.rs)

remote-server-busy-another-client-connected = server occupato — un altro client è collegato
remote-transmitting-but-client-s-microphone = In trasmissione, ma il microfono di questo client non produce alcun audio — non viene modulato nulla. In un browser, verifica che alla pagina sia stato concesso il microfono e che nessun'altra applicazione lo tenga occupato.
remote-connection-closed = connessione chiusa

## solarwin (src/solar3d/mod.rs)

solarwin-sdroxide-solar-system = sdroxide — sistema solare

## solarbtn (src/app/solar.rs)

solarbtn-solar-system-3d-view-sun = Vista 3D del sistema solare — Sole, Terra, Luna, macchie solari e CME (finestra separata)
solarbtn-solar-system-3d-view-sun-2 = Vista 3D del sistema solare — Sole, Terra, Luna, macchie solari e CME (nuova scheda del browser)

## widespec (src/widgets/wide_spectrum.rs)


## speech (src/app/speech.rs)

speech-loading-voice = Caricamento della voce…
speech-browser-client-cannot-speak-yet = il client nel browser non può ancora parlare

## freqdisplay (src/widgets/freq_display.rs)

freqdisplay-double-click-type-frequency = Doppio clic per scrivere una frequenza
freqdisplay-tap-type-frequency = Tocca per scrivere una frequenza

## solarstate (src/solar3d/state.rs)

solarstate-home = HOME
solarstate-dwarf-planets = PIANETI NANI

## adsbmap (src/adsb_map.rs)

adsbmap-double-click-reframe = doppio clic per reinquadrare

## aprsmap (src/aprs_map.rs)

aprsmap-double-click-reframe = doppio clic per reinquadrare

## solarnet (src/solar3d/net.rs)

solarnet-connection-closed = connessione chiusa

## alerts (src/app/alerts.rs)

alerts-could-not-start-alert-thread = impossibile avviare il thread degli avvisi

## hfdlmap (src/hfdl_map.rs)

hfdlmap-no-aircraft-located-yet = nessun aereo localizzato ancora

## worldmap (src/widgets/worldmap.rs)

worldmap-double-click-reframe = DOPPIO CLIC PER REINQUADRARE

## chrome (src/chrome.rs)


## duopower (src/app/duo_power.rs)

duopower-sets-fdm-duo-s-own = Imposta la potenza propria dell'FDM-DUO (menu 34 TX POWER). Pilotaggio e Accordo impostano poi la potenza al suo interno, e SDRoxide invia di nuovo l'impostazione ogni volta che si collega.

## Band conditions window: a peak date.

# $month is 1–12.
bands-peak-date = { $day } { $month ->
        [1] gen
        [2] feb
        [3] mar
        [4] apr
        [5] mag
        [6] giu
        [7] lug
        [8] ago
        [9] set
        [10] ott
        [11] nov
       *[12] dic
    }

## download (src/download.rs)

download-browser-could-not-read-file = il browser non è riuscito a leggere quel file

## morse (src/app/morse.rs)

morse-translate = TRADUCI
morse-practice = ESERCIZIO
morse-learn = IMPARA
morse-morse = MORSE
morse-text-morse = Testo → Morse
morse-morse-text = Morse → Testo
morse-per-character-space-between-characters = `.` e `-` per carattere, uno spazio tra i caratteri, ` / ` tra le parole. L'alfabeto ITU, le cifre, la punteggiatura e i prosigni comuni.
morse-reference-translator-available-desktop-build = Il traduttore di riferimento è disponibile nella versione desktop.
morse-play-what-you-type-your = Riproduci ciò che scrivi, sui tuoi altoparlanti
morse-play = RIPRODUCI
morse-stop = STOP
morse-every-character-learned-well-done = Ogni carattere imparato — ben fatto.
morse-drill-needs-sound-which-desktop = L'esercizio richiede il suono, che la versione desktop ha.
morse-reveal = MOSTRA
morse-reset = RIPRISTINA
morse-replay = RIASCOLTA
morse-what-did-you-hear = Cosa hai sentito?
morse-check = VERIFICA
morse-press-new-hear-character = Premi NUOVO per sentire un carattere.
morse-tone = Tono
morse-speed = Velocità
morse-spacing = Spaziatura
morse-characters-go-out-speed-above = I caratteri partono alla velocità qui sopra; questa è la velocità con cui vengono inviati gli *intervalli*, così un principiante sente la lettera nel suo insieme prima di poterla inviare a quella velocità.
morse-playback-available-desktop-build = La riproduzione è disponibile nella versione desktop.

## ADS-B panel: aircraft detail rows.

adsb-row-altitude = altitudine
adsb-row-track = rotta
adsb-row-turning = in virata
adsb-row-squawk = squawk
adsb-row-range = distanza
adsb-row-position = posizione
adsb-row-signal = segnale
adsb-row-frames = frame
rade-transmitting = in trasmissione
decodes-odd = DISPARI
decodes-new = NUOVA
js8-new = NUOVA

## AIS panel: vessel detail rows.

ais-row-status = stato
ais-row-speed = velocità
ais-row-course = rotta
ais-row-heading = prua
ais-row-turning = in virata
ais-row-altitude = altitudine
ais-row-size = dimensione
ais-row-draught = pescaggio
ais-row-destination = destinazione
ais-row-range = distanza
ais-row-signal = segnale
ais-row-type = tipo
ais-row-position = posizione
ais-row-messages = messaggi
ism-off = OFF
textmodem-frequency-shifted = a spostamento di frequenza
winlink-to-heading = A

## adsbmap: messages with values (src/adsb_map.rs)

adsbmap-ft-min =
    {""}
    { $vr } ft/min

## aismap: messages with values (src/ais_map.rs)

aismap-heard-no-position-newer-than = { $heard } sentite — nessuna posizione più recente di { $drop_map_s }s
aismap-stations-heard-waiting-position-report = { $heard } stazioni sentite — in attesa di un rapporto di posizione
aismap-km-across = { $lon_span }° · ~{ $km } km di larghezza

## alerts: messages with values (src/app/alerts.rs)

alerts-alarming = allarme su { $device }
alerts-alarm-device-unavailable = dispositivo di allarme non disponibile: { $error }

## awards: messages with values (src/app/awards.rs)

awards-worked = { $w } lavorati
awards-confirmed = { $value } confermati

## bands: messages with values (src/app/bands.rs)

bands-forecast-hamqsl-com-old = · previsione da HAMQSL.com, di { $value }
bands-ncdxf-iaru-next-beacon-s = NCDXF/IARU · prossimo radiofaro tra { $now } s
bands-hearing-means-path-open-each = { $callsign } su { $location } — sentirlo significa che il percorso sulla { $band } è aperto. Ogni radiofaro sale di una banda ogni 10 s, quindi questa riga cambia a ogni slot.
bands-radiants-placed = radianti posizionati per { $lat }°, { $lon }°
bands-zhr = ZHR { $zhr }
bands-radiant = radiante { $alt_deg }° { $az_deg }
bands-radiant-down = radiante sotto ({ $alt_deg }°)
bands-km-s-parent-peak-rate =
    { $value } ({ $code }) — { $velocity_kms } km/s, genitore { $parent }. Frequenza massima ZHR { $zhr } intorno a { $peak }.

    Un radiante sopra l'orizzonte significa che le scie possono raggiungerti; uno sciame veloce lascia scie ionizzate più durature per la dispersione meteorica sui 6 m e sui 2 m, e le brevi aperture sui 10 m.

## drm: messages with values (src/app/drm.rs)

drm-not-decoding = { $channel } non decodificato
drm-symbols = { $value } simboli · { $frame }
drm-not-decodable = { $value } — non decodificabile
drm-broadcaster-s-clock-utc = orologio del trasmettitore { $year }-{ $month }-{ $day } { $hour }:{ $minute } UTC

## duopower: messages with values (src/app/duo_power.rs)

duopower-max-fdm-duo-s-full =
    MAX: la potenza piena dell'FDM-DUO, circa 8 W. L'impostazione da usare con l'HR500 in linea.

    { $base }
duopower-t-5-w-hr500-line-will =
    5 W. Con l'HR500 in linea a questa impostazione non raggiungerà la piena potenza sulla maggior parte delle bande: usa MAX, e Pilotaggio per abbassarla.

    { $base }
duopower-w-running-barefoot-qrp =
    { $code } W, per lavorare senza amplificatore o in QRP.

    { $base }

## frame: messages with values (src/app/frame.rs)

frame-re-enables-transmit-after-1 = Riabilita la trasmissione dopo { $swr }:1. Controlla prima l'antenna: se il guasto c'è ancora, anche la trasmissione successiva si ferma.
frame-sdroxide-has-been-released = SDRoxide { $version } è stato rilasciato — questo è { $sdroxide_version }.
frame-reconnecting-s = Riconnessione tra { $value } s…

## gridtracker: messages with values (src/app/grid_tracker.rs)

gridtracker-worked = { $worked } lavorati
gridtracker-confirmed = { $confirmed } confermati
gridtracker-heard = { $heard } sentiti

## hd: messages with values (src/app/hd.rs)

hd-decode = Decodifica { $value } ({ $codec_label })

## hfdl: messages with values (src/app/hfdl.rs)

hfdl-khz-already-has-chip = { $cur_khz } kHz ha già un chip
hfdl-save-current-channel-khz-one = Salva il canale attuale ({ $cur_khz } kHz) come uno dei tuoi chip
hfdl-aircraft = { $aircraft } aerei
hfdl-bursts = { $bursts } burst
hfdl-decodes-2 = { $decodes } decodifiche
hfdl-fixed = { $fec } corretti

## ism: messages with values (src/app/ism.rs)

ism-not-decoded-yet = { $hint } — non ancora decodificato
ism-listen-tunes-radio-there-one = Ascolta su { $value } — sintonizza lì la radio. Una banda alla volta: sono troppo distanti perché un ricevitore ne copra due insieme.
ism-watch-hz-around-band-centre = Osserva { $value }Hz attorno al centro della banda, qualunque cosa la banda avrebbe richiesto. Più stretto va bene per un ricevitore che non può fornire la larghezza propria della banda; più largo raggiunge dispositivi più lontani dal centro. Il tuo ricevitore deve fornirne circa un terzo in più di questo.
ism-rtl-433 = rtl_433 — { $why }
ism-rtl-433-khz = rtl_433 { $value } · { $khz } kHz
ism-your-own = { $flex } propri
ism-decoders-decoded = { $decoders } decodificatori ({ $flex }), { $decodes } decodificate
ism-handled-rtl-433 = gestiti da rtl_433: { $names }
ism-flex = flex: { $error }
ism-window-mhz-mhz-wide = finestra { $mhz } MHz, larga { $e62 } MHz
ism-bursts-decoded = { $bursts } burst, { $decodes } decodificati
ism-devices = { $ism_reports } dispositivi
ism-id-mhz-db-heard-first =
    { $protocol } id { $device }
    { $e6 } MHz { $snr_db } dB sentito { $value }×
    primo ascolto { $first_at } fa

## logbook: messages with values (src/app/logbook.rs)

logbook-confirmations-downloaded-newly-confirmed = Conferme: { $recs } scaricate, { $matched } appena confermate
logbook-qso = { $qso_log } QSO
logbook-worked-before = ⚠ GIÀ LAVORATO ({ $dupe_band })
logbook-utc-qso = { $oldest }–{ $newest } UTC · { $value } QSO

## app: messages with values (src/app/mod.rs)

app-radio = radio { $id }

## morse: messages with values (src/app/morse.rs)

morse-could-not-start-morse-audio = impossibile avviare il thread audio Morse: { $error }
morse-no-audio-output = nessuna uscita audio: { $error }
morse-about-s = circa { $farnsworth_wpm } s
morse-characters-play = Caratteri in gioco: { $set }
morse-score-run = Punteggio: { $correct } / { $total }. Serie: { $streak } di { $value }.
morse-correct = corretto — { $value }
morse-was = era { $value }

## net: messages with values (src/app/net.rs)

net-synced-z-new-already-log = Sincronizzato alle { $when }Z: { $added } nuovi, { $matched } già nel log
net-wavelog-2 = Wavelog: { $line }
net-full-resync-stopped-nothing-changed = Risincronizzazione completa interrotta, nulla è stato modificato: { $error }
net-full-resync-applied-added-updated = Risincronizzazione completa applicata: { $add } aggiunti, { $update } aggiornati, { $remove } rimossi, { $mark } dei tuoi segnati come presenti in Wavelog
net-wavelog-3 = Wavelog: { $line }
net-adif-import-failed = Importazione ADIF non riuscita: { $error }
net-not-unicode-read = {" "}(non Unicode; letto come { $enc })
net-received-reports-swl-left-out = , { $n } rapporti di ricezione (SWL) esclusi
net-adif-import-added-duplicates-skipped = Importazione ADIF: { $added } aggiunti, { $skipped } duplicati saltati{ $reports }{ $assumed }
net-channel-import-failed = Importazione dei canali non riuscita: { $error }
net-line-s-skipped = , { $n } riga/e saltata/e
net-not-unicode-read-2 = {" "}(non Unicode; letto come { $enc })
net-channel-import-channel-s-read = Importazione dei canali: { $n } canale/i letto/i{ $skipped }{ $assumed }

## acars: messages with values (src/app/panels/acars.rs)

acars-frames-bad = { $frames } frame · { $bad } errati
acars-mode = modo { $mode }

## adsb: messages with values (src/app/panels/adsb.rs)

adsb-aircraft-3 = { $aircraft } aerei
adsb-frames = { $frames } frame
adsb-preambles = { $preambles } preamboli
adsb-bad-crc = { $bad_crc } CRC errati
adsb-tune = SINTONIZZA { $mhz }
adsb-ft-barometric = { $ft } ft barometrici

## ais: messages with values (src/app/panels/ais.rs)

ais-vessels-3 = { $vessels } navi
ais-messages = { $messages } messaggi
ais-slots = { $bursts } slot
ais-bad-fcs = { $bad_fcs } FCS errati
ais-ais-mhz = AIS { $value } ({ $mhz } MHz) — { $value2 }
ais-ais-mhz-slots-messages-floor = AIS { $value } ({ $mhz } MHz) — { $bursts } slot, { $messages } messaggi, fondo { $floor_dbfs } dBFS
ais-ais = AIS { $value }
ais-tune = SINTONIZZA { $mhz }
ais-ships-heard-are-khz-off = le navi sentite sono fuori frequenza di { $khz } kHz ({ $mhz } ppm a 162 MHz) — imposta la correzione di frequenza del front-end, oppure le trasmissioni oltre circa 5 kHz smetteranno di decodificarsi
ais-over-ground = { $value }° sul fondo
ais-dbfs-db-snr = { $rssi_dbfs } dBFS, { $snr_db } dB SNR
ais-last-type-ais = { $messages } (ultimo tipo { $last_type }, AIS { $channel })
ais-ais-2 = AIS { $ch }
ais-mhz-marine-channel-ship-alternates = { $mhz } MHz — canale marittimo { $ch }. Una nave alterna i due, quindi disattivarne uno dimezza quanto spesso viene sentita ogni nave.

## aprs: messages with values (src/app/panels/aprs.rs)

aprs-object = oggetto da { $reported_by }
aprs-via = via { $via }
aprs-object-reported = oggetto riportato da { $reported_by }
aprs-digit-s-blanked = ± { $ambiguity } cifra/e oscurata/e
aprs-wind-m-s = vento { $value }° { $value2 } m/s
aprs-gust = raffica { $value }
aprs-rain-mm-h = pioggia { $value } mm/h
aprs-frame-s-last-ago = { $packets } frame, l'ultimo { $last_heard } fa
aprs-via-2 = via { $via }
aprs-bad = { $bad_frames } errati
aprs-other = { $non_aprs } altri
aprs-sent = inviato ×{ $tries }
aprs-stn = { $stations } staz
aprs-queued = { $tx_queue } in coda

## atchat: messages with values (src/app/panels/atchat.rs)

atchat-master = master { $value }
atchat-last-heard-s-ago-click = { $status } — ultimo ascolto { $age_s }s fa · clicca per aprire una scheda di messaggi diretti
atchat-close-tab = chiudi la scheda di { $peer }
atchat-direct-message = messaggio diretto a { $value }
atchat-cannot-display = impossibile mostrare { $path }

## cw: messages with values (src/app/panels/cw.rs)

cw-click-decoder-neural-one-copies =
    { $hint }

    Clicca per il decodificatore { $neural }. Quello neurale copia più in profondità e legge il CW inviato a mano che un adattamento di temporizzazione non accetterebbe; quello a temporizzazione è l'unico che copia le lettere accentate — Ä, Ö, Å, Ü, É — perché il modello non ha una classe di uscita per esse.
cw-idle-s = INATTIVO { $idle }s

## decodes: messages with values (src/app/panels/decodes.rs)

decodes-save-decodes-list-adif-received = Salva le { $n } decodifiche nell'elenco come ADIF — rapporti ricevuti, non contatti. Le decodifiche che non indicano un mittente vengono escluse.
decodes-save-decodes-list-csv-one = Salva le { $n } decodifiche nell'elenco come CSV, una riga ciascuna
decodes-session-qso = Sessione: { $session } QSO
decodes-save-whole-logbook-qso-adif = Salva l'intero logbook ({ $logged } QSO) come ADIF
decodes-save-whole-logbook-qso-text = Salva l'intero logbook ({ $logged } QSO) come testo
decodes-hz-slots = { $audio_hz } Hz · { $odd } slot
decodes-your-slot-timing-against-stations =
    La temporizzazione dei tuoi slot rispetto alle stazioni che stai sentendo.
    { $after }

    Copre l'intero percorso di ricezione, quindi una catena audio o di rete lenta conta come un orologio sbagliato. Sotto 0,5 s è confortevole.
decodes-pile-up = PILE-UP { $fox_queue }
decodes-queue = CODA { $call_queue }
decodes-db-hz-click-remove =
    { $waiting } · { $snr_db } dB · { $audio_hz } Hz{ $unwrap_or_default }
    Clicca per rimuovere

## js8: messages with values (src/app/panels/js8.rs)

js8-every-speed-being-decoded-marks = Ogni velocità viene decodificata; ▸ segna { $speed }, quella su cui trasmetti
js8-every-speed-being-decoded-click = Ogni velocità viene decodificata. Clicca per trasmettere invece a { $speed }
js8-transmit-decode = Trasmetti e decodifica a { $speed }
js8-beaconing-every-min-click-stop = Radiofaro ogni { $hb_min } min — clicca per fermare
js8-last-beacon-went-out-hz = L'ultimo radiofaro è partito a { $hz } Hz — uno slot libero nella sotto-banda heartbeat da { $value }–{ $value2 } Hz, scelto perché cada lontano dai segnali decodificati.
js8-frames-s = { $tx_frames_pending }/{ $tx_frames_total } frame · { $left }s
js8-decoded = Decodificato a { $speed }
js8-frames = … ({ $frames } frame)
js8-reply-2 = Rispondi a { $value }: “{ $value2 }”
js8-address-composer = Indirizza il compositore a { $value }

## macros: messages with values (src/app/panels/macros.rs)

macros-f-sends = F{ $i }: invia “{ $text }”
macros-sends = Invia “{ $text }”
macros-most = { $value } è il massimo

## navtex: messages with values (src/app/panels/navtex.rs)

navtex-repaired-lost = { $repaired } riparati · { $lost } persi
navtex-characters-lost = { $lost } caratteri persi

## packet: messages with values (src/app/panels/packet.rs)

packet-bad = { $bad_frames } errati
packet-link-layer = Il livello di collegamento è in { $link }
packet-via = {" "}via { $via }
packet-unacked = { $unacked } non confermati
packet-retry = tentativo { $retries }

## pi4: messages with values (src/app/panels/pi4.rs)

pi4-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] radiofaro
       *[other] radiofari
    }
pi4-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } fa

## rade: messages with values (src/app/panels/rade.rs)

rade-snr-db = SNR { $snr_db } dB
rade-offset-hz = offset { $freq_offset_hz } Hz
rade-heard = sentito { $call }
rade-samples-dropped-between-receiver-decoder = ⚠ { $dropped } campioni persi tra il ricevitore e il decodificatore — questa macchina non sta al passo con la decodifica neurale.

## textmodem: messages with values (src/app/panels/text_modem.rs)

textmodem-char-s = { $chars_per_sec } car/s
textmodem-char-s-hz-wide = { $value } — { $chars_per_sec } car/s, larga { $bandwidth_hz } Hz, { $keyed }

## vdl2: messages with values (src/app/panels/vdl2.rs)

vdl2-frames = { $frames } frame
vdl2-bursts = { $bursts } burst
vdl2-sync = { $syncs } sincro
vdl2-hdlc-bad = { $hdlc_bad } HDLC errati
vdl2-bad-fcs = { $fcs_bad } FCS errati
vdl2-mhz-bursts-frames-noise-floor = { $mhz } MHz, { $what } — { $bursts } burst, { $frames } frame, rumore di fondo { $floor_dbfs } dBFS
vdl2-tune = SINTONIZZA { $mhz }
vdl2-held-2 = BLOCCATO +{ $since }
vdl2-use-worldwide-where-every-link = { $what } — in uso in tutto il mondo, ed è dove inizia ogni collegamento. Quello da tenere se ne tieni uno solo.
vdl2-assigned-which-channels-carry-anything = Assegnato a un { $what }. Quali canali portano qualcosa dipende da dove sei: lasciali tutti attivi a meno che tu non sappia altrimenti.
vdl2-mhz-db-snr-dbfs-evm = { $mhz } MHz · { $snr_db } dB SNR · { $rssi_dbfs } dBFS · EVM { $evm_deg }° · { $freq_err_hz } Hz · { $rs_corrected } simboli RS corretti
vdl2-characters = { $parity_errors } caratteri
vdl2-parameter-sdroxide-does-not-read = { $unknown } { $unknown ->
        [one] parametro
       *[other] parametri
    } che SDRoxide non legge — mostrati sopra come identificatore e byte

## wefax: messages with values (src/app/panels/wefax.rs)

wefax-lines = { $lines } righe
wefax-lines-held = { $live_size } righe trattenute
wefax-pixels-per-line = { $width } pixel per riga
wefax-shift-picture-pixels = Sposta l'immagine di { $px } pixel
wefax-charts-are-saved-click-copy =
    Le carte vengono salvate { $place }

    Clicca per copiare il percorso
wefax-finished-charts-are-saved-collect = Le carte completate vengono salvate { $place } e si raccolgono qui.
wefax-pixels-right-click-delete =
    { $when_full }
    { $size } × { $size2 } pixel
    { $value }

    Clic destro per eliminare
wefax-older-store = { $older } più vecchie nell'archivio
wefax-older-load-more = { $older } più vecchie — carica altre
wefax-published-carrier-khz-dial-khz = Portante pubblicata { $f } kHz → sintonia { $value } kHz USB
wefax-delete-chart-2 = Elimina questa carta { $dir }

## widgets: messages with values (src/app/panels/widgets.rs)

widgets-cq-zone-itu-zone = { $continent } · zona CQ { $cq_zone } · zona ITU { $itu_zone }
widgets-new-entity = Nuova entità su { $band_label }
widgets-worked-before = Lavorato prima su { $band_label }
widgets-calling-cq-includes-you = Chiama CQ { $target } — include te
widgets-calling-cq-not-aimed-you = Chiama CQ { $target } — non rivolto a te

## wspr: messages with values (src/app/panels/wspr.rs)

wspr-rx-heard-us = { $heard_us } rx · { $heard_us2 } ci hanno sentiti
wspr-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] radiofaro
       *[other] radiofari
    }
wspr-beaconing-duty = radiofaro — { $wspr_power_dbm }, ciclo { $wspr_tx_percent }%
wspr-listening-duty = in ascolto (ciclo { $wspr_tx_percent }%)
wspr-beacon-one-two-minute-slot = Radiofaro in uno slot da due minuti su ogni { $pct } — il { $pct2 }% di essi, distribuiti in modo uniforme — a { $wspr_power_dbm }. Lo slot in cui inizia il ciclo è scelto dal tuo nominativo, così due stazioni che usano questo programma non partono insieme.
wspr-transmitting-set-general-tab-settings = Trasmette come { $call } in { $sent } — impostato nella scheda Generale delle Impostazioni.
wspr-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } fa

## publicsdr: messages with values (src/app/publicsdr.rs)

publicsdr-antenna =
    { $address }
    { $device }
    antenna: { $antenna }
    { $e3 }
publicsdr-receivers-fetched-ago = { $entries } ricevitori · { $per_net } · scaricati { $age } fa
publicsdr-showing-search-narrow = mostrati { $shown } di { $n } — cerca per restringere
publicsdr-match = { $n } corrispondenza
publicsdr-map-status = { $shown } sulla mappa · { $unplaced } senza posizione
publicsdr-opening-another-radio = Apertura di { $entry } come un'altra radio…

## qo100: messages with values (src/app/qo100.rs)

qo100-khz-3 = +{ $park_lo_hz } … +{ $park_hi_hz } kHz
qo100-before-switching-tune-so-beacon = Prima di attivare: sintonizza in modo che i due lobi del radiofaro cadano tra +{ $park_lo_hz } e +{ $park_hi_hz } kHz nella striscia (la corsia ombreggiata).
qo100-receiver-currently-listening-around-mhz = Il ricevitore sta ascoltando attorno a { $mhz } MHz — ben lontano dal radiofaro a 10489.750 MHz, quindi qui non c'è nulla da disegnare. Il decodificatore continua comunque a cercare.
qo100-receiver-currently-covers-only-khz = il ricevitore copre qui solo ±{ $khz } kHz — il resto della striscia resta vuoto
qo100-null-db-sym-snr-db = nullo { $est_null_depth_db } dB sim { $est_symmetry } snr { $est_snr_db } dB
qo100-hz-s-hz-s-de = { $est_drift_hz_s } Hz/s { $est_drift_accel_hz_s2 } Hz/s² (derotato prima della decodifica)
qo100-hz-s-de-rotated-before = { $est_drift_hz_s } Hz/s (derotato prima della decodifica)
qo100-found-empty = { $est_updates } trovate, { $est_misses } vuote
qo100-last-s-ago = {"   "}ultimo { $auto_last_hz } ({ $auto_last_unix }s fa)
qo100-over-correction = { $auto_total_hz } su { $auto_applies } { $auto_applies ->
        [one] correzione
       *[other] correzioni
    }{ $last }
qo100-sync-32-err = sincro { $sync_bit_errors } / 32 err ×{ $sync_matches }
qo100-frame-buffer = buffer dei frame { $frame_fill }%
qo100-blocks-tried-decoded = { $blocks_tried } blocchi provati, { $blocks_locked } decodificati
qo100-last-applied-s-ago-hz = ultimo applicato { $ago }s fa: { $old } → { $new } Hz ({ $old2 } Hz)
qo100-locked-block-tried-locked = agganciato — { $blocks_tried } { $blocks_tried ->
        [one] blocco
       *[other] blocchi
    } provati, { $blocks_locked } agganciati
qo100-searching-block-tried-none-locked = ricerca in corso — { $blocks_tried } { $blocks_tried ->
        [one] blocco
       *[other] blocchi
    } provati, nessuno ancora agganciato

## rds: messages with values (src/app/rds.rs)

rds-auto = Automatico ({ $standard })
rds-type = tipo { $class }
rds-groups = · { $groups } gruppi
rds-blocks-error = · { $ber } % dei blocchi con errore
rds-clean-corrected-lost = { $blocks_ok } puliti · { $blocks_corrected } corretti · { $blocks_bad } persi

## sat: messages with values (src/app/sat.rs)

sat-utc-local = { $unix } UTC (ora locale { $offset })
sat-aos = AOS { $rise_unix } { $rise_unix2 }
sat-los = LOS { $set_unix } { $set_unix2 }
sat-above-horizon = { $rise_unix } sopra l'orizzonte
sat-min = tra { $value } min
sat-h-min = tra { $value } h { $value2 } min
sat-searching = ricerca… { $next } di { $entries }
sat-locked-2 = ● AGGANCIATO — { $t }
sat-pass-until-max = Passaggio fino alle { $set_unix } · massimo { $max_el }°
sat-next-pass-rises-max = Prossimo passaggio { $rise_unix } { $rise_unix2 } ({ $now }) · sorge a { $rise_az }° { $rise_az2 } · massimo { $max_el }°
sat-rigctld-client-connected-satellite-software = ⚠ { $clients } { $clients ->
        [one] client rigctld collegato
       *[other] client rigctld collegati
    } — il software satellitare che sposta lì la sintonia correggerebbe il Doppler due volte
sat-rotator-antenna-az-el = Rotore: antenna a { $az }° az { $el }° el
sat-rotator-2 = Rotore: { $error }
sat-up-now-2 = ● visibile ora, { $el }°
sat-inv = { $mode } · inv
sat-norad-2 = NORAD { $norad_id }

## scanner: messages with values (src/app/scanner.rs)

scanner-holding-mhz = in attesa su { $here } MHz
scanner-scanning-mhz = scansione · { $here } MHz

## spectrum: messages with values (src/app/spectrum.rs)

spectrum-renderer-will-not-hold-texture = Questo renderer non può contenere una texture più larga di { $max_texture_dim } pixel.
spectrum-mem = Mem: { $f } / { $value }
spectrum-mem-2 = Mem: { $value }

## speech: messages with values (src/app/speech.rs)

speech-voice-2 = Voce: { $voice }
speech-speech-unavailable = Il parlato non è disponibile: { $error }

## spots: messages with values (src/app/spots.rs)

spots-transmissions = { $stations } trasmissioni

## winlink: messages with values (src/app/winlink.rs)

winlink-last-session-failed = ultima sessione non riuscita — { $err }
winlink-last-session-received-sent = ultima sessione: { $last_received } ricevuti, { $last_sent } inviati
winlink-empty = { $to_lowercase } è vuota
winlink-showing = mostrati { $entries } di { $total }
winlink-fetching = recupero di { $mid }…
winlink-from-2 = da { $msg } { $date }
winlink-attachment-bytes = allegato: { $att } ({ $data } byte)

## aprsmap: messages with values (src/aprs_map.rs)

aprsmap-object =
    {""}
    oggetto da { $reported_by }
aprsmap-heard-ago =
    {""}
    sentito { $age } fa

## download: messages with values (src/download.rs)

download-reading = lettura di { $path }: { $error }

## help: messages with values (src/help.rs)

help-missing-image = {"["}immagine mancante: { $path }]

## hfdlmap: messages with values (src/hfdl_map.rs)

hfdlmap-mhz =
    {""}
    { $e3 } MHz
hfdlmap-fix =
    {""}
    { $fixes } fix{ $es }
hfdlmap-heard-ago =
    {""}
    sentito { $last_at } fa

## multi: messages with values (src/multi.rs)

multi-radio = Radio { $n }
multi-could-not-add-radio = Impossibile aggiungere una radio: { $error }
multi-was-closed-station = { $value } è stata chiusa sulla stazione.
multi-could-not-open = Impossibile aprire { $peer }: { $error }
multi-has-tab-own-now = { $value } ha ora una scheda propria.

## remote: messages with values (src/remote.rs)

remote-protocol-error = errore di protocollo: { $error }

## solarnet: messages with values (src/solar3d/net.rs)

solarnet-bad-message = messaggio non valido: { $error }
solarnet-solar-protocol-mismatch-server-client = protocollo solare non corrispondente: server { $proto }, client { $value }

## smeter: messages with values (src/widgets/smeter.rs)

smeter-forward-power-w-measured-radio =
    {""}

    Potenza diretta — { $w } W, misurata dal sensore proprio della radio.
smeter-po-radio-s-own-power =
    {""}

    PO — il misuratore di potenza in uscita proprio della radio, al { $clamp } % del suo fondo scala. Una posizione dell'ago come la riporta la radio, non una potenza in watt calibrata; ciò che conta è se cala mentre il Pilotaggio resta fermo.
smeter-swr-1-match-antenna-presenting =
    {""}

    ROS — { $swr }:1, l'adattamento che l'antenna presenta, come lo misura la radio. Sotto 2:1 è normale; oltre 3:1 la scala diventa rossa.
smeter-received-signal-strength-s9-db = Intensità del segnale ricevuto: S9 + { $over } dB ({ $s_dbm } dBm nella banda passante).
smeter-received-signal-strength-s-dbm = Intensità del segnale ricevuto: S{ $value } ({ $s_dbm } dBm nella banda passante).
smeter-radio-reports-c =
    {""}

    La radio riporta { $t } °C.
smeter-ps-puresignal-correcting-db-compression =
    {""}

    PS — PureSignal sta correggendo { $correction_db } dB di compressione{ $held } (la retroazione corrispondeva alla trasmissione al { $score }).
smeter-ps-puresignal-switched-but-has =
    {""}

    PS −− — PureSignal è attivo ma non ha trovato la trasmissione nel percorso di retroazione (corrispondenza migliore { $score }), quindi il trasmettitore esce senza correzione. Verifica che un campione dell'uscita dell'amplificatore arrivi davvero a un ingresso di ricezione che il commutatore T/R non toglie in trasmissione, e che il suo attenuatore non lo stia assorbendo. Il ciclo ascolta solo questo ricevitore — un accoppiatore collegato a un secondo ricevitore (RX2, ADC2) non viene mai letto come retroazione.

## specview: messages with values (src/widgets/spectrum_view.rs)

specview-rit-hz = RIT { $hz } Hz
specview-xit-hz = XIT { $hz } Hz

## decodes: nested messages

decodes-you-transmit-s-everyone-else = Trasmetti { $off } s { $after } tutti gli altri — il motivo abituale per cui le chiamate restano senza risposta. Sincronizza l'orologio del computer.

## publicsdr: nested messages

publicsdr-noise-floor-score = punteggio del rumore di fondo { $snr }
publicsdr-up-khz-i-q = fino a { $khz } kHz di I/Q

## txaudio: more messages

txaudio-gr-gate-de-ess-comp = GR gate { $gate_gr_db } de-ess { $deess_gr_db } comp { $comp_gr_db } lim { $limiter_gr_db }

## amp: more messages

amp-h-step = ~{ $l_step } µH · passo { $l_step2 }
amp-pf-step = ~{ $c_step } pF · passo { $c_step2 }

## sstv: more messages

sstv-bit-octets-chunks-first-pass = { $from } · { $filename } · { $width }×{ $height } { $bits_per_pixel }-bit · { $media_type } / { $content_encoding } · { $encoded_size } ottetti in { $chunk_count } blocchi ({ $chunks_first_pass } al primo passaggio) · sessione { $session }
sstv-baud-cpfsk-hz-khz-occupied = { $value } — { $symbol_rate } baud CPFSK, ±{ $deviation_hz } Hz, larghezza di banda occupata { $bandwidth_hz } kHz

## Clock, and a few short labels with values.

time-utc-and-local = { $utc } UTC · { $local } locale
sat-in-seconds = tra { $s } s
winlink-to-list = a { $to }
winlink-cc-list = cc { $cc }
spots-n-of-total = { $n } di { $total }
wefax-n-of-total = { $n } di { $total }
adsb-range-bearing = { $km } km a { $bearing }°
ais-range-bearing = { $km } km a { $bearing }°

## More values from sdroxide-types, as pickers name them (src/i18n/labels.rs).
rtl-sdr-hf-mode-off = Disattivato (solo sintonizzatore, da 24 MHz in su)
rtl-sdr-hf-mode-auto = Automatico
rtl-sdr-hf-mode-direct-q = Campionamento diretto (ramo Q)
rtl-sdr-agc-manual = Manuale (nessun AGC)
rtl-sdr-agc-tuner = AGC del sintonizzatore
rtl-sdr-agc-rtl = AGC digitale RTL
rtl-sdr-agc-both = AGC sintonizzatore + RTL
mode-control-cat = CAT
mode-control-radio = Controllato dalla radio
line-state-none = Nessuna
line-state-high = Alta
line-state-low = Bassa
elad-tx-input-usb-audio = Audio USB
elad-tx-input-mic = Microfono
elad-tx-input-auto = Automatico
elad-tx-input-leave = Lascia come impostato sulla radio
tr-usdx-audio-one-cable = Un cavo (audio nel flusso CAT)
tr-usdx-audio-sound-card = Scheda audio USB (jack 3,5 mm)
spy-server-format-uint8 = 8 bit (minor larghezza di banda)
spy-server-format-int16 = 16 bit
spy-server-format-float32 = Float a 32 bit
sound-format-iq = IQ (stereo)
sound-format-demod-audio = Audio demodulato
sdr-play-duo-tuner-tuner1 = Sintonizzatore 1 (50 Ohm / Hi-Z)
sdr-play-duo-tuner-tuner2 = Sintonizzatore 2 (50 Ohm)
sdr-play-duo-role-diversity = Una seconda antenna (diversità / soppressione QRM)
sdr-play-duo-role-second-radio = Una seconda radio, sulla propria frequenza
sdr-play-agc-off = Disattivato (guadagno IF manuale)
sdr-play-agc-hz5 = 5 Hz
sdr-play-agc-hz50 = 50 Hz
sdr-play-agc-hz100 = 100 Hz
rotator-protocol-rotctld = Hamlib rotctld (TCP)
rotator-protocol-pst-rotator = PSTRotator (UDP)
pluto-ptt-off = Disattivato
pluto-ptt-gpo01 = GPO0 = RX, GPO1 = TX
pluto-ptt-gpo23 = GPO2 = RX, GPO3 = TX
pluto-duplex-fdd = FDD (entrambe insieme)
pluto-duplex-tdd = TDD (una alla volta)
pluto-agc-manual = Manuale
pluto-agc-slow-attack = Attacco lento
pluto-agc-fast-attack = Attacco veloce
pluto-agc-hybrid = Ibrido
parity-none = Nessuna
parity-even = Pari
parity-odd = Dispari
panadapter-tap-antenna = Antenna (condivisa con la radio)
panadapter-tap-if-output = L'uscita I.F. della radio
panadapter-audio-attached = Il ricevitore collegato
panadapter-audio-transceiver = Il ricetrasmettitore
kenwood-send-ts2000 = Stile TS-2000 (TX;)
kenwood-send-ts590 = Stile TS-590 (TX1;)
digi-mode-usb = USB
digi-mode-data = DIGI
digi-mode-radio = Controllato dalla radio
cw-keying-cat = Manipolatore dell'apparato (CAT)
cw-keying-audio = Scheda audio (MCW)
audio-route-both = Entrambi i canali
audio-route-left = Solo sinistro
audio-route-right = Solo destro
amp-model-hr500 = Hardrock-500
amp-model-hr50 = Hardrock-50 (non testato)
alert-sound-ding = Campanella
alert-sound-two-tone = Due toni
alert-sound-triplet = Terzina
alert-sound-warble = Trillo
alert-sound-digital = Digitale
alert-reply-tone = Tono
alert-reply-voice = Voce
alert-reply-both = Tono + voce
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
icom-model-other = Altro (imposta l'indirizzo a mano)
backend-auto = Rilevamento automatico (SoapySDR / CAT)
backend-soapy = SoapySDR
backend-cat = CAT / Audio
backend-hpsdr = HPSDR (rete)
backend-tci = TCI (rete)
backend-icom-net = Icom LAN (rete)
backend-smart-sdr = SmartSDR / FlexRadio (rete)
backend-pluto = PlutoSDR (rete)
backend-rtl-sdr = RTL-SDR (USB)
backend-rtl-tcp = RTL-SDR tramite rtl_tcp (rete)
backend-spy-server = SpyServer (rete)
backend-spy-server-vfo = SpyServer VFO+FFT, bassa larghezza di banda (rete)
backend-kiwi-sdr = KiwiSDR / Web-888 (rete)
backend-rx888 = RX-888 (USB)
backend-airspy-hf = Airspy HF+ (USB)
backend-airspy = Airspy R2 / Mini (USB)
backend-hack-rf = HackRF One / Pro (USB)
backend-sdr-play = SDRplay RSP (USB)
backend-elad = ELAD FDM-DUO / FDM-S (USB)
backend-lime = LimeSDR + LimeRFE (LimeSuite)
backend-hydra-sdr = HydraSDR RFOne (USB)
backend-fobos = RigExpert Fobos SDR (USB)
backend-usb-audio = Radio USB audio (scheda audio)
backend-none = Non configurata
cat-family-xiegu = Xiegu
cat-family-icom = Icom
cat-family-yaesu = Yaesu
cat-family-kenwood = Kenwood
cat-family-elecraft = Elecraft
cat-family-elad = ELAD
cat-family-qrp-labs = QRP Labs
cat-family-rs-hfiq = RS-HFIQ
cat-family-tr-usdx = (tr)uSDX
cat-family-rigctld = Hamlib rigctld (rete)
cat-family-flrig = flrig (rete)

## Values from sdroxide-types drawn on chips, menus and lists (src/i18n/labels.rs).
agc-mode-off = Off
agc-mode-slow = Lento
agc-mode-med = Medio
agc-mode-fast = Veloce
airspy-gain-linearity = Linearità (migliore con segnali forti)
airspy-gain-sensitivity = Sensibilità (migliore con segnali deboli)
ais-kind-class-a = Classe A
ais-kind-class-b = Classe B
ais-kind-base-station = stazione base
ais-kind-sar-aircraft = aereo SAR
ais-kind-aid-to-navigation = ausilio alla navigazione
ais-kind-sart = SART / MOB / EPIRB
ais-kind-craft = unità associata
alert-event-called = Una stazione mi sta chiamando
alert-event-cq = Un CQ a cui potrei rispondere
alert-event-new-dxcc = Nuova entità DXCC
alert-event-new-dxcc-band = Nuova entità su questa banda
alert-event-new-grid = Nuovo quadrato locatore
audio-routing-both = Sinistra + destra
audio-routing-left = Solo sinistra
audio-routing-right = Solo destra
button-mode-momentary = Tieni premuto
button-mode-toggle = Alterna
contest-mode-none = Nessuno
contest-mode-eu-vhf = EU VHF Contest
converter-tx-off = Disattivato durante la conversione
converter-tx-transverter = Attraverso lo stesso convertitore
converter-tx-own = Offset proprio
cw-engine-neural = NEURALE
cw-engine-timing = TEMPORIZZAZIONE
decode-sort-none = Nessuno
decode-sort-signal = SNR
decode-sort-distance = Dist
decode-sort-country = Paese
diversity-mode-cancel = Cancella — annulla una sorgente di rumore
diversity-mode-combine = Combina — ricezione in diversità
dxped-mode-normal = Normale
dxped-mode-hound = Hound
dxped-mode-fox = Fox
fail-safe-refuse-tx = Rifiuta di trasmettere
fail-safe-warn-only = Trasmetti comunque, avvisa
ftp-security-plain = FTP (senza cifratura)
ftp-security-explicit-tls = FTPS esplicito (AUTH TLS)
ftp-security-implicit-tls = FTPS implicito
hpsdr-filter-board-none = Nessuna — le uscite restano spente
hpsdr-filter-board-n2adr = Scheda filtri N2ADR
hpsdr-filter-board-alex = Codice banda Alex / Hermes (Zeus, HiQSDR, Quisk)
hpsdr-filter-board-custom = Personalizzata — la mia tabella qui sotto
hpsdr-io-rx-input-radio = Ingresso proprio della radio
hpsdr-io-rx-input-io-board = Scheda IO J9
hpsdr-io-rx-input-io-board-pure-signal = Scheda IO J9, PureSignal in trasmissione
hydra-sdr-gain-linearity = Linearità (migliore con segnali forti)
hydra-sdr-gain-sensitivity = Sensibilità (migliore con segnali deboli)
icom-rx-source-af = AF — la radio demodula
icom-rx-source-if12k = IF a 12 kHz — sdroxide demodula
icom-scope-span-radio = Come impostato sulla radio
icom-scope-span-khz2-5 = 5 kHz (±2,5k)
icom-scope-span-khz5 = 10 kHz (±5k)
icom-scope-span-khz10 = 20 kHz (±10k)
icom-scope-span-khz25 = 50 kHz (±25k)
icom-scope-span-khz50 = 100 kHz (±50k)
icom-scope-span-khz100 = 200 kHz (±100k)
icom-scope-span-khz250 = 500 kHz (±250k)
icom-scope-span-khz500 = 1 MHz (±500k)
ism-family-weather = METEO
ism-family-meter = CONTATORI
ism-family-home-auto = CASA
ism-family-lo-ra = LORA
ism-family-unidentified = SCONOSCIUTO
ism-protocol-la-crosse-it = LaCrosse IT+
ism-protocol-fine-offset = Fine Offset
ism-protocol-bresser = Bresser
ism-protocol-z-wave = Z-Wave
ism-protocol-homematic = Homematic
ism-protocol-rtl433 = rtl_433
ism-protocol-unidentified = non identificato
js8-speed-normal = NORMALE
js8-speed-fast = VELOCE
js8-speed-turbo = TURBO
js8-speed-slow = LENTO
lime-aux-role-off = Non usata
lime-aux-role-diversity = Una seconda antenna (diversità / soppressione QRM)
lime-aux-role-pure-signal = Retroazione di trasmissione (predistorsione PureSignal)
lookup-provider-none = Off
lookup-provider-qrz = QRZ.com
lookup-provider-ham-qth = HamQTH
mail-folder-inbox = POSTA IN ARRIVO
mail-folder-outbox = POSTA IN USCITA
mail-folder-sent = INVIATA
mail-folder-archive = ARCHIVIO
memory-sort-name = Nome
memory-sort-freq = Freq
memory-sort-band = Banda
memory-sort-stored = Memorizzata
mouse-button-middle = Centrale
mouse-button-extra1 = Indietro (extra 1)
mouse-button-extra2 = Avanti (extra 2)
nr-level-off = Off
nr-level-low = Bassa
nr-level-medium = Media
nr-level-high = Alta
nr-level-rnn-low = RNN Bassa
nr-level-rnn-med = RNN Media
nr-level-rnn-high = RNN Alta
nr-level-spec-low = SPEC Bassa
nr-level-spec-med = SPEC Media
nr-level-spec-high = SPEC Alta
nr-level-nr2-low = NR2 Bassa
nr-level-nr2-med = NR2 Media
nr-level-nr2-high = NR2 Alta
nr-level-df-low = DFNR Bassa
nr-level-df-med = DFNR Media
nr-level-df-high = DFNR Alta
nr-strength-low = Bassa
nr-strength-med = Media
nr-strength-high = Alta
orbit-rings-none = nessuno
orbit-rings-curated = selezionati
orbit-rings-all = tutti
qso-step-idle = Inattivo
qso-step-wait-cq = Attesa CQ
qso-step-calling-cq = Chiamata CQ
qso-step-tx-grid = TX Locatore
qso-step-tx-report = TX Rapporto
qso-step-tx-r-report = TX R+Rapporto
qso-step-tx-rr73 = TX RR73
qso-step-tx73 = TX 73
qso-step-confirming = Conferma
rds-standard-auto = Automatico
rds-standard-rds = RDS
rds-standard-rbds = RBDS
region-r1 = Regione 1 — Europa, Africa, Medio Oriente, Asia settentrionale
region-r2 = Regione 2 — Americhe
region-r3 = Regione 3 — Asia meridionale e orientale, Oceania, Pacifico
relative-mode-absolute = Assoluto (cursore)
relative-mode-sign-magnitude = Relativo: segno/modulo
relative-mode-twos-complement = Relativo: complemento a due
relative-mode-binary-offset = Relativo: centrato su 64
relay-link-off = Nessuno
relay-link-serial = Scheda relè USB (seriale)
relay-link-serial-lines = Linea seriale RTS/DTR
relay-link-hid = Scheda relè USB (HID)
relay-link-cm108 = GPIO scheda audio CM108/CM119
relay-link-gpio = Linea GPIO Linux
relay-link-command = Comando esterno
relay-role-unused = Non usato
relay-role-sdr-antenna = Metti a massa l'antenna SDR
relay-role-amplifier = Comanda un amplificatore / relè T-R
relay-role-aux = Ausiliario
relay-role-band-decoder = Decodificatore di banda (filtro / transverter)
rfe-channel-wb1000 = Banda larga 1 – 1000 MHz
rfe-channel-wb4000 = Banda larga 1 – 4 GHz
rfe-channel-ham0030 = HF, fino a 30 MHz
rfe-channel-ham0070 = 6 m / 4 m (50 – 70 MHz)
rfe-channel-ham0145 = 2 m (140 – 150 MHz)
rfe-channel-ham0220 = 1,25 m (220 – 225 MHz)
rfe-channel-ham0435 = 70 cm (400 – 450 MHz)
rfe-channel-ham0920 = 33 cm (902 – 928 MHz)
rfe-channel-ham1280 = 23 cm (1220 – 1420 MHz)
rfe-channel-ham2400 = 13 cm (2,3 – 2,5 GHz)
rfe-channel-ham3500 = 9 cm (3,3 – 3,7 GHz)
rfe-channel-cell-band01 = Banda cellulare 1
rfe-channel-cell-band02 = Banda cellulare 2
rfe-channel-cell-band03 = Banda cellulare 3
rfe-channel-cell-band07 = Banda cellulare 7
rfe-channel-cell-band38 = Banda cellulare 38
rfe-channel-not-selected = Nessuno selezionato
rfe-link-off = Non collegato
rfe-link-serial = Il proprio cavo USB (seriale)
rfe-link-board = Tramite il LimeSDR (GPIO / I²C)
rfe-mode-control-auto = Automatico (segue il cablaggio)
rfe-mode-control-rx = Ricezione sempre
rfe-mode-control-tx = Trasmissione sempre
rfe-mode-control-tx-rx = Entrambe sempre (solo connettori separati)
rifp-encoding-auto = Automatico
rifp-encoding-png = PNG
rifp-encoding-jpeg = JPEG
rifp-encoding-raw = Grezzo
rifp-encoding-rle8 = RLE8
rifp-encoding-zlib = Zlib
rifp-encoding-group4 = G4
rifp-encoding-group3 = G3
scan-kind-memories = MEM
scan-kind-range = INTERVALLO
scan-resume-carrier = PORTANTE
scan-resume-timed = A TEMPO
scan-resume-manual = MANUALE
sense-line-off = Non collegata
sense-line-cts = CTS
sense-line-dsr = DSR
sense-line-dcd = DCD (rilevamento portante)
shift-simplex = SIMPLEX
shift-minus = −
shift-plus = +
speed-slow = Lento
speed-medium = Medio
speed-fast = Veloce
speed-faster = Più veloce
speed-fastest = Velocissimo
tls-verify-system = Archivio di fiducia di sistema
tls-verify-pinned = Impronta fissata
tls-verify-off = Non verificare
tone-mode-off = OFF
tone-mode-ctcss = CTCSS
tone-mode-dcs = DCS
tx-voice-stage-gate = Gate
tx-voice-stage-eq = EQ
tx-voice-stage-de-ess = De-esser
tx-voice-stage-comp = Compressore
upload-naming-sequence = image1 … imageN (in rotazione)
upload-naming-original = Nome file originale
wheel-action-zoom = Zoom
wheel-action-tune = Sintonia
wheel-action-none = Nulla

# Undocked windows (from sdroxide-brown)
frame-detached-title = { $flavor } — { $name }
frame-dock-band-menu-hint = ACCORPA — riporta il selettore di banda e modo nella finestra principale, come chiudere questa senza cercare il suo pulsante di chiusura.
frame-dock-panel-hint = ACCORPA — rimette questo pannello sotto il waterfall
frame-dock-spectrum-hint = ACCORPA — riporta questo spettro nella finestra principale, come chiudere questa senza cercare il suo pulsante di chiusura.
frame-every-module-own-window = ogni modulo è nella sua finestra
frame-module-aux-panadapter = panadapter aux
frame-module-band-mode = banda e modo
frame-module-controls = controlli
frame-module-panadapter = panadapter
frame-module-panel = pannello
multi-dock-all-windows = ACCORPA TUTTE LE FINESTRE
multi-module-panel = Pannello operativo
multi-undock-all-windows = SGANCIA TUTTE LE FINESTRE
settings-ui-dock-all-hint = Riporta ogni finestra sganciata in questa.
settings-ui-docked = Accorpato in questa finestra
settings-ui-module-aux = AUX SP (secondo spettro)
settings-ui-module-band-menu = Selettore di banda e modo
settings-ui-module-controls = Controlli
settings-ui-module-panadapter = Panadapter
settings-ui-undock-all-hint = Metti ogni modulo in una finestra propria: la finestra principale resta con lo spettro e i decodificatori. Un modulo senza nulla da mostrare, come il pannello operativo in un modo voce, non apre alcuna finestra.
settings-ui-undock-hint =
    Sganciato, questo modulo si sposta nella sua finestra e la finestra principale restituisce lo spazio a ciò che resta: metti il waterfall su un secondo monitor, o i controlli su un terzo. Chiudere la sua finestra lo riaccorpa.

    Su Wayland la dimensione viene ripristinata ma è il compositor a decidere dove va una finestra: rendila flottante e fissala a un monitor con una regola di finestra che corrisponda all'app-id "{ $app_id }".
settings-ui-undocked = Sganciato — finestra propria
settings-ui-window-label = finestra { $module }
frame-window-chip-hint = FINESTRA — sposta questa finestra in una finestra del sistema operativo a sé, per metterla su un altro monitor
frame-dock-panel-pane-hint = ACCORPA — riporta questo pannello nel riquadro sotto il waterfall
frame-panel-window-title = { $mode } — pannello
frame-roll-up-hint = Arrotola — lascia la barra del titolo e restituisce la finestra al waterfall
frame-unroll-hint = Srotola — mostra di nuovo il pannello
frame-window-panel-hint = FINESTRA — dà a questo pannello una finestra propria, da mettere su un altro monitor
topbar-dock-all = ACCORPA TUTTO
topbar-dock-all-hint = Riporta ogni finestra di modulo in questa finestra
topbar-module-windows-title = Finestre dei moduli
topbar-undock-all = SGANCIA TUTTO
topbar-undock-all-hint = Metti ogni modulo in una finestra propria: questa finestra resta con lo spettro e i decodificatori
topbar-windows-chip-hint =
    ▣  Sinistro: riporta a casa ogni finestra di modulo
    Destro: l'elenco delle finestre dei moduli
topbar-wname-aux = SP2 (AUX) — un secondo spettro
topbar-wname-band-menu = Banda e modo
topbar-wname-controls = Controllo RX
topbar-wname-panadapter = SP1 — lo spettro
topbar-wname-panel = Pannello operativo
topbar-wstate-float = Solleva { $name } dal riquadro e fallo fluttuare sopra questa finestra
topbar-wstate-floating-panel-only = Flottante è lo stato proprio del pannello operativo: uno spettro o una console ha un solo posto in questa finestra
topbar-wstate-here-now = { $name } è qui adesso
topbar-wstate-no-panel = Nessun pannello operativo in { $mode }: si applica in un modo che ne ha uno
topbar-wstate-own-window = Dai a { $name } una finestra propria, su qualsiasi monitor
topbar-wstate-put-back = Rimetti { $name } in questa finestra
frame-workspace-missing = Nessuno spazio di lavoro { $n } salvato: salvane uno in Impostazioni › UI › Spazi di lavoro e questo tasto lo richiamerà.
settings-ui-workspaces = Spazi di lavoro
settings-ui-workspaces-note = Una disposizione di finestre con nome: quali moduli sono nelle proprie finestre e dove si trovano. Salva quella che hai, o ripristinane una salvata. Fino a dieci.
settings-ui-workspaces-none = Ancora nessuno spazio di lavoro salvato.
settings-ui-workspace-restore-home = Ripristina questa disposizione — ogni modulo nella finestra principale
settings-ui-workspace-restore-out = Ripristina questa disposizione — { $n } modulo/i nella/e propria/e finestra/e
settings-ui-workspace-forget = Dimentica questo spazio di lavoro
settings-ui-workspace-name = dai un nome a questa disposizione
settings-ui-workspace-full = Dieci spazi di lavoro è il limite che SDRuno conserva; eliminane uno prima
settings-ui-workspace-save = SALVA ATTUALE
settings-ui-workspace-save-hint = Salva la disposizione attuale con questo nome
settings-ui-workspace-name-first = Scrivi prima un nome
topbar-band-window2-hint = FINESTRA — apre il selettore in una finestra propria, larga quanto vuoi, su qualsiasi monitor
tnc-link = COLLEGAMENTO
tnc-connected = CONNESSO
tnc-calling = CHIAMATA
tnc-idle = IN ASCOLTO
tnc-tx = TX
tnc-busy = OCCUPATO
tnc-busy-hint = C'è qualcun altro sul canale. Una chiamata adesso gli finirebbe sopra.
tnc-setup = ⚙ CONFIGURA
tnc-peer = Corrispondente
tnc-width = Larghezza
tnc-speed = Velocità
tnc-snr = S/R
tnc-bytes = Byte
tnc-queued = In coda
tnc-host = Host
tnc-ports = Porte
tnc-ports-off = spente
tnc-nobody = nessuno
tnc-holder-panel = questo pannello
tnc-holder-winlink = la finestra MAIL
tnc-holder-tcp = un programma su { $addr }
tnc-tune = SINTONIA
tnc-abort = INTERROMPI
tnc-abort-hint = Chiudere subito il collegamento, chiunque lo stia gestendo. L'altro capo andrà in timeout invece di essere avvisato.
tnc-terminal = TERMINALE
tnc-callsign = nominativo
tnc-connect = CONNETTI
tnc-disconnect = DISCONNETTI
tnc-disconnect-hint = Chiudere come si deve: prima esce ciò che è in coda, poi l'altro capo conferma.
tnc-held-by = Il modem è in uso da { $holder }.
tnc-no-callsign = Imposta prima un nominativo (⚙ CONFIGURA)
tnc-clear = CANCELLA
tnc-clear-hint = Svuotare la trascrizione. Il collegamento resta com'è.
tnc-nothing-yet = Ancora niente. Scrivi un nominativo sopra e premi CONNETTI, oppure attiva l'ascolto in ⚙ CONFIGURA per ricevere chiamate.
tnc-type-here = scrivi qui, Invio spedisce
tnc-not-connected = non connesso
tnc-send = INVIA
setup-tnc-bandwidth = Larghezza di banda
setup-tnc-listen = Rispondere alle chiamate
setup-tnc-listen-check = Ascoltare dal pannello
setup-tnc-listen-hint = Accettare le chiamate in arrivo nel terminale del pannello. Senza effetto finché le porte TCP dell'host sono attive, perché un programma collegato lì vuole le chiamate.
setup-tnc-host = Host TCP
playback-radio-n = { " " }· radio { $n }
playback-no-stamp = nessuna data nel nome — viene riprodotto nel modo in cui ti trovi
playback-title = RIPRODUZIONE — audio registrato
playback-find-hint = modo, MHz, data, nome…
playback-refresh-hint = Rileggi la cartella delle registrazioni
playback-asking = Richiesta delle registrazioni alla stazione…
playback-folder = Cartella: { $dir }
playback-none = Ancora nessuna registrazione. Premi REC → MP3 mentre c'è qualcosa in aria; il file compare qui, con nome composto da ora, frequenza e modo.
playback-on = ▶ ATTIVO
playback-play = ▶ RIPRODUCI
playback-iq-hint = Un'acquisizione I/Q, non audio. La riproduzione prende l'audio scritto dal registratore; un'acquisizione I/Q si riproduce con --file.
playback-not-while-tx = Non durante la trasmissione
playback-play-hint = Riproduci questa registrazione nei decodificatori dall'inizio
playback-busy-playing = In riproduzione — fermala prima di eliminarla
playback-busy-recording = Ancora in registrazione — ferma REC prima di eliminarla
playback-delete-ask = ELIMINARE?
playback-delete-hint-armed = Premi di nuovo per eliminare definitivamente questo file. Non si può annullare.
playback-delete-hint = Elimina questa registrazione dalla cartella (chiede ancora una volta prima)
playback-strip-label = ▶ RIPRODUZIONE
playback-not-live = non in diretta — la radio è in pausa
playback-carry-on = Continua a riprodurre
playback-pause = ⏸ PAUSA
playback-pause-hint = Pausa — i decodificatori sentono silenzio finché non riprendi
playback-again = ↺ DA CAPO
playback-again-hint = Torna all'inizio — un altro tentativo su un'immagine o una decodifica
playback-loop = LOOP
playback-loop-hint = Ricomincia dall'inizio quando il file finisce
playback-stop = ■ STOP
playback-stop-hint = Termina la riproduzione e rimette la radio com'era
playback-files = FILE
playback-end-of-file = fine del file
playback-paused = in pausa
playback-recorded-on-mode = Registrato su { $mhz } MHz in { $mode } — la sintonia e il modo sono stati impostati dal nome del file.
playback-recorded-on = Registrato su { $mhz } MHz — la sintonia e il modo sono stati impostati dal nome del file.
playback-no-dial-mode = Il nome non riporta né sintonia né modo, quindi viene riprodotto nel modo in cui ti trovi — scegli il decodificatore dal menu dei modi.
playback-format-stereo = { $rate } Hz, stereo — viene riprodotto il canale sinistro (ricezione).
playback-format-mono = { $rate } Hz, mono — viene riprodotto l'unico canale.
playback-stopped = Interrotto: { $fault }
playback-intro = Riproduci una registrazione nei decodificatori — SSTV, CW, RTTY, PSK, FT8 e ogni altro modo che decodifica dall'audio. La radio resta in pausa durante la riproduzione, la sintonia e il modo passano a quelli su cui è stata fatta la registrazione, e STOP rimette la radio com'era. Cambia modo durante la riproduzione per provare un altro decodificatore.
topbar-stereo-yields-hint = Lo stereo WFM è disattivato mentre la riduzione del rumore o l'auto-notch sono attivi — impasterebbero i due canali. Disattivali per sentire lo stereo
topbar-stereo-playback-hint = Stereo: la registrazione viene riprodotta in entrambe le orecchie, come registrata. Clicca per forzare il mono
topbar-stereo-mono-recording-hint = Questa registrazione è mono — non c'è un secondo canale da riprodurre
topbar-caption-play-back = Riproduci
topbar-recordings-chip = ▶ REGISTRAZIONI…
topbar-recordings-hint = Riproduce una registrazione nei decodificatori — un'immagine SSTV persa, un debole segnale RTTY o CW, uno slot FT8 — con il waterfall e il log di decodifica, come se fosse in aria. La radio si fa da parte mentre riproduce.
playback-find = Cerca
playback-refresh = AGGIORNA
wxsat-title = SATELLITI METEO
sat-wxsat-hint = SATELLITI METEO — l'immagine che un satellite NOAA (APT) o Meteor (LRPT) trasmette su 137 MHz, disegnata man mano che arriva, in una finestra staccabile
sat-wxsat = SATELLITI METEO
wxsat-decode = Decodifica
wxsat-channel = Satellite
wxsat-tune = Sintonizza qui
wxsat-tune-hint = Attiva il decodificatore e porta la frequenza su questo canale, in modo che il segnale sia dentro la finestra di campionamento.
wxsat-clear = Cancella
wxsat-save = Salva PNG
wxsat-flip = Capovolgi
wxsat-flip-hint = Capovolge l'immagine: un satellite che vola verso sud viene disegnato a testa in giù.
wxsat-state-off = Decodificatore spento
wxsat-level = Livello { $db } dBFS
wxsat-locked = Sincronismo di riga agganciato
wxsat-searching = Ricerca del sincronismo di riga
wxsat-lines = { $n } righe
wxsat-outside-window = Il canale è fuori dalla finestra di campionamento, o troppo vicino al bordo per l'intero segnale. Usa Sintonizza qui, o una finestra più ampia.
wxsat-waiting = Ancora nessuna immagine. Attiva il decodificatore quando un satellite è sopra l'orizzonte; un passaggio dura circa dieci minuti.
wxsat-signal = Segnale
wxsat-kind-apt = APT
wxsat-kind-lrpt = LRPT
wxsat-image-channel = Canale { $n }
wxsat-rows = { $n } file
wxsat-frame-locked = Sincronismo di trama agganciato
wxsat-frames = { $n } trame buone
wxsat-waiting-lrpt = Ancora nessuna immagine. Il LRPT di Meteor è digitale: l'immagine inizia pochi secondi dopo aver sentito il satellite, fila per fila, e ogni canale è un'immagine separata.
wxsat-searching-frame = Ricerca del sincronismo di trama
sondemap-no-sonde-yet = Nessuna radiosonda ricevuta finora
sonde-decoding = DECODIFICA
sonde-decoding-off = DECODIFICATORE SPENTO
sonde-count = Sonde: { $n }
sonde-frames = Frame: { $n }
sonde-channel = Canale
sonde-channel-hover = La frequenza della sonda da ascoltare (400–406 MHz). Anche la radio si sintonizza lì, così il canale è dentro la finestra.
sonde-col-serial = Sonda
sonde-col-alt = Quota
sonde-col-climb = Salita
sonde-col-speed = Velocità
sonde-col-battery = Batteria
sonde-col-heard = Ricevuta
sonde-scan = Scansione
sonde-scan-hover = Cerca sonde in tutta la finestra di campionamento e ne decodifica fino a quattro insieme, invece di ascoltare un solo canale. La finestra deve coprirle: il campo Canale centra la radio.
sonde-online = Online (SondeHub)
sonde-online-hover = Mostra anche le sonde ricevute da altre stazioni, dall'elenco pubblico di SondeHub. Solo lettura: vengono inviati il locatore arrotondato a 0,1° e un raggio di 400 km; non viene caricato nulla. Disattivato finché non lo attiva.
sonde-online-count = { $n } online
sonde-online-no-grid = Imposti il suo locatore nella configurazione dei modi digitali per vedere le sonde online.
sonde-online-failed = SondeHub non ha risposto.
sonde-export = Esporta il volo
sonde-export-kml-hover = Salva il volo di questa sonda come traccia KML (Google Earth): ora, longitudine, latitudine, quota in metri
sonde-export-csv-hover = Salva il volo di questa sonda come tabella CSV per un foglio di calcolo: ora, latitudine, longitudine, quota in metri
sonde-export-none = Nessuna posizione ancora conservata per questa sonda.
sonde-col-temp = Temp.
sonde-col-humidity = Umidità
sonde-col-pressure = Pressione
sonde-online-why-signin = La stazione richiede prima l'accesso.
sonde-online-why-busy = La stazione interroga già SondeHub il più spesso possibile. Riprovi tra un minuto.
sonde-online-why-upstream = La stazione non è riuscita a raggiungere SondeHub.
sonde-online-why-unreadable = La risposta non era un elenco di sonde.
sonde-online-why-no-station = Nessuna stazione da interrogare, e questa pagina non può raggiungere SondeHub da sola.
sonde-online-why-http = Risposta: HTTP { $n }.
sonde-card-source = Origine
sonde-card-source-radio = Questa radio
sonde-card-source-hub = SondeHub (sentita da un'altra stazione)
sonde-card-model = Modello
sonde-card-maker = Costruttore
sonde-card-variant = Variante
sonde-card-position = Posizione
sonde-card-from-you = Da lei
sonde-card-heading = Rotta
sonde-card-satellites = Satelliti
sonde-card-snr = Rapporto segnale/rumore
sonde-card-station = Sentita da
sonde-card-station-position = Posizione della stazione
sonde-card-antenna = Antenna
sonde-card-software = Software
sonde-card-frames = Frame ricevuti
sonde-card-open = Apri su { $site }
sonde-card-close = Chiudi
sonde-card-source-both = Radio + SondeHub (la ricevono entrambe)
sonde-card-stale = Stato
sonde-card-stale-value = Non più nell'elenco: ultima posizione nota
sonde-card-detach = Stacca
sonde-card-attach = Riaggancia
sonde-card-listen-hover = Fai clic per sintonizzare la radio su questo canale: la sintonia si sposta lì e il decodificatore ascolta solo quello.
sonde-card-decoding = Decodificata da questa radio
sonde-card-waiting = In ascolto su questo canale, ancora nulla di decodificato
sonde-card-click-to-listen = Fai clic sulla frequenza per ascoltare
sonde-type-all = Tutti i tipi
sonde-type-hover = Cerca un tipo di sonda: sceglierne uno avvia la scansione, che sposta la finestra lungo la banda finché si sente quel tipo. Solo quel tipo è elencato e disegnato.
sonde-export-saved = { $name } salvato.

## Keyboard modes: MT63 (src/app/panels/text_modem.rs)
textmodem-mt63-long-hint = MT63-{ $label }, larghezza { $bw } Hz — l'interleave lungo distribuisce una raffica su { $chars } caratteri
textmodem-mt63-short-hint = MT63-{ $label }, larghezza { $bw } Hz, interleave corto — l'interleave corto distribuisce una raffica su { $chars } caratteri
textmodem-mt63-not-offered = 1K e 500L non ancora offerti
textmodem-mt63-not-offered-hint = MT63-1K e l'interleave lungo a 500 Hz si leggono con il decodificatore batch ma non ancora attraverso il ricevitore in tempo reale: l'aggancio cerca la fase dei simboli e la polarità ma non il centro banda, e una ricerca che lo faccia si è rivelata scambiare una variante con un'altra.

## Keyboard modes: MFSK (src/app/panels/text_modem.rs)
textmodem-mfsk-hint = { $label } — { $baud } baud, { $tones } toni, larghezza { $bw } Hz{ $caveat }
textmodem-mfsk-reverse-hint = Inverte l'ordine dei toni. Una stazione ricevuta sulla banda laterale opposta a quella con cui è stata trasmessa arriva speculare rispetto al suo centro e viene letta come rumore finché questa opzione non è attiva.

## Keyboard modes: DominoEX (src/app/panels/text_modem.rs)
textmodem-domino-hint = { $name } — { $baud } baud, 18 toni, modulazione incrementale di frequenza senza correzione d'errore in avanti
textmodem-ifk-reverse-hint = Inverte i passi dei toni. IFK+ porta i dati nel passo tra toni successivi, quindi una stazione ricevuta sulla banda laterale opposta ha ogni passo negativo e viene letta come rumore finché questa opzione non è attiva.

## Keyboard modes: PSK families (src/app/panels/text_modem.rs)
textmodem-psk-family = Famiglia
textmodem-psk-family-plain-hint = La famiglia propria del PSK31: una portante differenziale, nessuna correzione d'errore in avanti
textmodem-psk-family-bpsk63f-hint = Il codice robusto a 62,5 baud, senza interleaver: l'unico modo codificato che fldigi trasmette senza
textmodem-psk-family-robust-hint = Una portante con il codice robusto e un interleaver 2x2xN
textmodem-psk-family-multi-hint = Più portanti, varicode del PSK31 e **nessun** codice: più larghi e veloci, e richiedono un segnale pulito
textmodem-psk-family-robust-multi-hint = Più portanti **e** il codice robusto: il più largo e il più robusto dei modi PSK
textmodem-psk-baud-fast-hint = { $baud } baud — si legge dopo circa un secondo: a questa velocità il tempo di aggancio del ricevitore è un terzo di un messaggio
textmodem-psk-baud-hint = { $baud } baud
textmodem-psk-bpsk-hint = Un bit per simbolo, rivelazione differenziale
textmodem-psk-coded = codificato
textmodem-psk-no-code = senza codice
textmodem-psk-coded-hint =
    { $baud } baud, { $n } { $n ->
            [one] portante
           *[other] portanti
        }, { $code }, larghezza { $bw } Hz
