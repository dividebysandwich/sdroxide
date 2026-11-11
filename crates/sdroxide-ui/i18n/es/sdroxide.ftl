# sdroxide — traducción al español de la interfaz. La referencia es
# en-US/sdroxide.ftl; lo que falta aquí se muestra en inglés.

## The language's own name, as the language picker lists it.
language-name = Español

## Values from sdroxide-types, as the screen names them (src/i18n/labels.rs).

layout-mode-auto = Automático
layout-mode-desktop = Escritorio
layout-mode-tablet = Tableta
layout-mode-small = Pantalla pequeña
layout-mode-phone = Teléfono

ui-theme-default = Predeterminado
ui-theme-light = Claro
ui-theme-high-contrast = Alto contraste
ui-theme-green-phosphor = Fósforo verde
ui-theme-amber-phosphor = Fósforo ámbar
ui-theme-teal-orange = Turquesa / naranja
ui-theme-rainbow = Arcoíris
ui-theme-nord = Nord
ui-theme-nord-dark = Nord oscuro
ui-theme-gruvbox = Gruvbox
ui-theme-everforest = Everforest
ui-theme-solarized-dark = Solarized oscuro
ui-theme-solarized = Solarized
ui-theme-dracula = Dracula
ui-theme-catppuccin-mocha = Catppuccin mocha
ui-theme-catppuccin-latte = Catppuccin latte
ui-theme-modern-minimalist = Minimalista moderno

chrome-style-angled = Angular
chrome-style-rectangular = Rectangular
chrome-style-rounded = Redondeado
chrome-style-gradient = Degradado
chrome-style-bevel = Biselado 3D
chrome-style-terminal = Terminal

font-size-small = Pequeño
font-size-medium = Mediano
font-size-large = Grande

bandplan-kind-ham = Radioaficionado
bandplan-kind-cw = CW
bandplan-kind-digi = Digital
bandplan-kind-phone = Voz
bandplan-kind-beacon = Baliza
bandplan-kind-broadcast = Radiodifusión
bandplan-kind-am = AM / OL / OM
bandplan-kind-cb = CB

speech-verbosity-terse = Conciso
speech-verbosity-normal = Normal
speech-verbosity-full = Completo

speech-freq-style-dial = Dial (14 punto 074)
speech-freq-style-digits = Cada dígito
speech-freq-style-full = Dial con unidades

speech-callsign-style-phonetic = Fonético (kilo uno alfa)
speech-callsign-style-letters = Letras (ka uno a)
speech-callsign-style-as-is = Tal como está escrito

## Settings window, UI tab (src/app/settings/ui_tab.rs).

settings-ui-display = Pantalla
settings-ui-moved-note = El detalle del panadaptador, la reacción del espectro y la velocidad de desplazamiento de la cascada se ajustan desde el botón SPEC de la franja de pantalla — el menú DISP en una ventana estrecha.
settings-ui-language = Idioma
    .hint = El idioma de la interfaz. El manual de F1 y los avisos de voz están en inglés sea cual sea el idioma elegido aquí.
settings-ui-language-system = Sistema ({ $language })
settings-ui-layout = Disposición
settings-ui-theme = Tema
settings-ui-button-style = Estilo de los botones
settings-ui-window-style = Estilo de las ventanas
settings-ui-frame-rate = Frecuencia de refresco de pantalla
settings-ui-palette = Paleta de la cascada
settings-ui-tuning-buttons = Botones de sintonía
settings-ui-tuning-buttons-show = Mostrar en teléfono y tableta
    .hint = Una fila de botones de paso abajo / paso / paso arriba bajo la franja de control en una pantalla táctil. No hay rueda con la que desplazar un dígito ni dial, así que sin ellos la única forma de moverse un paso conocido es escribir la frecuencia entera. Nunca se dibujan en un escritorio.
settings-ui-tuning-step = Paso: { $step }
settings-ui-tuning-snap = La primera pulsación se ajusta al paso
    .hint = Un dial que ha quedado entre dos múltiplos del paso va al siguiente en la dirección pulsada — 7 074 300 sube a 7 075 000 con un paso de 1 kHz y baja a 7 074 000. Después los botones se mueven por el paso como de costumbre. Desactivado, cada pulsación se mueve exactamente el paso.
settings-ui-smoothing = Suavizado de la cascada
settings-ui-smoothing-interpolate = Interpolar
    .hint = Mezcla cada píxel de la pantalla con los bins y filas que lo rodean, de modo que una señal se ve continua donde la pantalla es más ancha que la transformada. Desactívalo para una cascada rectangular — un bloque por bin, uno por fila — que es lo que hace falta para leer la forma de una señal en la imagen: una señal interpolada no se distingue de una realmente más ancha. Una FFT mayor (el chip FFT) es la otra mitad de eso.
settings-ui-clock = Reloj
settings-ui-clock-show = UTC + hora local en el panadaptador
    .hint = Un reloj en la parte superior derecha del panadaptador, en UTC y en tu hora local.
settings-ui-background = Fondo del espectro
settings-ui-background-gradient = Degradado
settings-ui-background-top = arriba
settings-ui-background-bottom = abajo
settings-ui-spot-colours = Colores de las etiquetas de spots
    .hint = El tono que lleva cada fuente de spots: los recuadros a lo largo del borde inferior de la cascada, las insignias de la lista SPOTS y los puntos del mapamundi.
settings-ui-spot-colour = Color de los spots de { $kind }
settings-ui-spot-colours-reset = Devolver todos los colores de spots a su valor predeterminado
settings-ui-reset = Restablecer
settings-ui-bandplan-colours = Colores del plan de bandas
    .hint = El tono con que se pinta cada clase de asignación en la franja del plan de bandas, en el borde inferior de la cascada. Los bloques se dibujan semitransparentes sobre la cascada, por lo que allí quedan más oscuros que en la muestra de aquí.
settings-ui-bandplan-colour = Color de las asignaciones de { $kind }
settings-ui-bandplan-colours-reset = Devolver todos los colores del plan de bandas a su valor predeterminado
settings-ui-skimmer-font = Tamaño de letra del skimmer
settings-ui-waterfall-font = Tamaño de letra de cascada / espectro
settings-ui-interface-font = Tamaño de letra de la interfaz
    .hint = Escala toda la interfaz — menús, diálogos, ventanas, las pestañas de radio, la barra superior y sus botones. Los dos tamaños anteriores son relativos a este.
settings-ui-cities = Ciudades en los mapas
    .hint =
        Dibuja las ciudades del mundo — un punto por lugar, con su nombre al lado cuando hay sitio — en los mapas planos: FT8/WSPR, APRS, ADS-B y AIS.

        Son casi todo lo que dice *dónde* está un punto en un mapa del mundo entero, y también lo más cargado que hay en él. Desactívalas si los nombres estorban a las estaciones que estás leyendo. El globo 3D no se ve afectado — sus ciudades son luces del lado nocturno y no marcadores, y nada allí se escribe sobre un contacto.
settings-ui-cities-show = mostrar las ciudades y sus nombres
settings-ui-update-check = Buscar actualizaciones al iniciar
    .hint = Pregunta a sdroxide.com una vez por inicio si se ha publicado una versión más reciente, y lo indica en un aviso descartable sobre la cascada. No se envía nada aparte de la propia petición.
settings-ui-band-conditions-download = Descargar el pronóstico de condiciones de banda
    .hint = Obtiene de HAMQSL.com el pronóstico de condiciones de las bandas de HF, como mucho una vez por hora, para colorear el menú de bandas. Desactivado, el menú de bandas no muestra pronóstico salvo que la vista 3D esté abierta. No se envía nada aparte de la propia petición.
settings-ui-broadcast-schedule-download = Descargar el horario de radiodifusión
    .hint = Descarga el horario de radiodifusión en onda corta EiBi (alrededor de 1 MB) de eibispace.de cuando empieza una nueva temporada. Desactivado, se sigue usando el horario incluido en sdroxide (o el último descargado), y Ajustes → Spots → Descargar el horario ahora lo sigue descargando a petición.
settings-ui-3d-view = Vista 3D
settings-ui-clouds = Renderizado de nubes
settings-ui-clouds-layered = Por capas
settings-ui-clouds-volumetric = Volumétrico
settings-ui-clouds-note = Cómo dibuja el tiempo la capa NUBES de la vista 3D. «Por capas» apila láminas a través de la troposfera y es la opción barata. «Volumétrico» hace pasar un rayo a través de ella, de modo que el Sol proyecta las cimas de las nubes sobre la cubierta inferior y los rayos brillan a través de la tormenta que los produce, en lugar de solo iluminar su exterior — a un coste por píxel varias veces mayor.

## Settings window, UI tab: voice announcements.

speech-title = Avisos de voz
speech-enabled = Anunciar los cambios del equipo
    .hint = Lee en voz alta lo que ha cambiado, de modo que el equipo pueda manejarse sin verlo
speech-voice = Voz
speech-voice-shipped = Voz incluida
speech-speed = Velocidad
    .hint = La voz estira o comprime su propia fraseología, así que el tono no cambia. A partir de aproximadamente 2× deja de acortarse.
speech-volume = Volumen
speech-output = Salida
speech-detail = Detalle
speech-duck = Atenuar el receptor
speech-duck-while = Mientras habla
speech-test = Probar
speech-what = Qué anunciar
speech-cat-frequency = Frecuencia
speech-cat-mode-band = Modo y banda
speech-cat-vfo-split = VFO y split
speech-cat-agc-gain = AGC y ganancia
speech-cat-levels = Excitación, sintonía y micrófono
speech-cat-ptt = Transmisión y recepción
speech-cat-rit-xit = RIT y XIT
speech-cat-memory-scan = Memorias y barrido
speech-cat-band-edge = Salir de una banda de aficionado
speech-cat-notices = Advertencias y mensajes
speech-cat-filters = Filtros, silenciador y reducción de ruido
    .hint = Desactivado de forma predeterminada: se mueven constantemente al perseguir una señal
speech-decoded = Mensajes decodificados
speech-ft8-to-me = Llamadas FT8 para mí
speech-ft8-cq = CQ de FT8 que podría contestar
    .hint = Una tarde concurrida en veinte metros son cien de estos por minuto
speech-js8 = Mensajes JS8 para mí
speech-js8-allcall = También JS8 @ALLCALL
speech-fsq = Mensajes FSQ para mí
speech-include-snr = Incluir el informe
    .hint = Solo cuando el mensaje no trae uno propio — una decodificación que ya informa de un número no recibe además el nuestro
speech-ft8-qso = Mi propio intercambio FT8
    .hint = Lo que el secuenciador está a punto de enviar, y el final del contacto
speech-reading-text = Lectura en voz alta del texto decodificado
speech-text-cw = CW
speech-text-rtty-psk = RTTY, PSK, Olivia, THOR, FSQ
speech-text-cw-locked = CW solo mientras el decodificador está enganchado
    .hint = Leer la salida de un decodificador sin enganchar es peor que el silencio
speech-text-note = Ambos están desactivados de forma predeterminada. Un decodificador produce texto más deprisa de lo que la voz lo lee, así que lo que se retrasa demasiado respecto al audio en directo se descarta en lugar de ponerse en cola.
speech-tuning-up = Sintonización
speech-swr-while-tuning = Leer la ROE mientras se mantiene TUNE pulsado
speech-swr-every = Cada
speech-swr-summary = Informar de la mejor adaptación al soltar
speech-swr-alarm = Avisar de ROE alta durante cualquier transmisión
speech-how-read = Cómo se leen las cosas
speech-frequencies = Frecuencias
speech-callsigns = Indicativos
speech-quiet-on-tx = Callar mientras se transmite
    .hint = La voz sale por tus altavoces y, por tanto, entra en tu micrófono. Los avisos de ROE alta siguen pasando.
speech-device-note = Los avisos se reproducen en su propio dispositivo de sonido, así que nunca se graban ni se envían a nadie que escuche en remoto. Las teclas para decir el estado, repetir el último aviso y detenerse a media frase están en la pestaña Controles.

## Settings window (src/app/settings/mod.rs)

settings-waiting-station-s-network-configuration = Esperando la configuración de red de la estación…
settings-device-reports = según informa el dispositivo
settings-transverters = Transversores
settings-one-row-per-box-while = Una fila por equipo. Mientras el dial está dentro de la banda de una fila, la radio se sintoniza a dial + desplazamiento y se aplican la regla de transmisión y el límite de excitación de esa fila; fuera de todas las filas la radio está en sus propias bandas. Las filas se prueban de arriba abajo.
settings-name = Nombre
settings-band-low = Banda, inferior
settings-band-high = Banda, superior
settings-offset = Desplazamiento
settings-transmit = Transmisión
settings-max-drive = Excitación máx.
settings-off-takes-transverter-out-line = «Desactivado» saca este transversor de la cadena sin perder la fila — la banda vuelve a lo que la radio alcanza por sí sola.
settings-t-2-m = 2 m
settings-what-you-call-shown-log = Cómo lo llamas. Se muestra en el registro cuando el dial lo selecciona.
settings-bottom-band-transverter-works-dial = El extremo inferior de la banda en que trabaja este transversor, en el dial. 144 para 2 m.
settings-top-band-dial-148-2 = El extremo superior de la banda, en el dial. 148 para 2 m en las Regiones 2 y 3, 146 en la Región 1.
settings-radio-ends-up-dial-offset = La radio acaba en dial + desplazamiento, así que un transversor que baja una banda a una FI es negativo: 2 m a una FI de 28 MHz es -116, porque 144 - 116 = 28. Arrastra para ajustarlo hercio a hercio si el oscilador está ligeramente desviado.
settings-off-while-converting-receive-only =
    Desactivado al convertir: solo recepción — la fila es un conversor, no un transversor, y no se pone nada en transmisión en esta banda.

    Por el mismo conversor: funciona en ambos sentidos y la transmisión usa el mismo desplazamiento.
settings-ceiling-transmit-drive-while-transverter =
    Un techo para la excitación de transmisión mientras este transversor está seleccionado, como porcentaje del máximo. La entrada de FI de un transversor admite milivatios y la excitación correcta para las bandas propias de la radio lo destruirá, así que este es el campo más importante de la fila. 100 % es sin límite.

    Tu ajuste de Excitación se mantiene por debajo de este valor en lugar de modificarse, así que el número que usas en HF vuelve cuando el dial sale de la banda.
settings-remove = QUITAR
settings-add-transverter = AÑADIR TRANSVERSOR
settings-takes-effect-apply-reconnect-like = Surte efecto con Aplicar / reconectar, igual que el desplazamiento del conversor de arriba.
settings-maximum-transmit-drive = Excitación máxima de transmisión
settings-hard-limit-drive-tune-controls = Un límite estricto que los controles de Excitación y TUNE no pueden superar, aplicado después de la calibración por banda de abajo para que nada pueda subir la excitación por encima. Fíjalo donde la radio da su potencia nominal y todo el control de Excitación pasa a ser utilizable — en un transmisor que modula sus propias muestras (HPSDR, LimeSDR, PlutoSDR, HackRF) la excitación completa es el transmisor a tope, lo que en un amplificador de alta ganancia queda muy por encima de lo que soportan sus finales. Desactivado significa que los controles llegan a la excitación completa.
settings-limit-drive = Limitar la excitación a
settings-per-cent-full-drive-key = Porcentaje de la excitación completa. Transmite sobre una carga ficticia y baja esto hasta que el medidor marque la potencia para la que está dimensionado el amplificador.
settings-applies-immediately-every-band-tune = Se aplica de inmediato, en todas las bandas y también a TUNE.
settings-transmit-drive-band = Excitación de transmisión por banda
settings-every-amplifier-has-different-gain = Cada amplificador tiene una ganancia distinta en cada banda, así que un único ajuste de Excitación da una potencia diferente en cada una. Mide la salida en cada banda y luego recorta las que salgan altas hasta igualarlas todas: ajusta la Excitación para la banda que necesita más (normalmente la más alta) y baja las demás hasta ella. Decibelios de potencia de salida, aplicados por igual a voz, digital y TUNE. Cero en todas — el valor predeterminado — no cambia nada.
settings-other = Otras
settings-clear-all = BORRAR TODO
settings-back-no-calibration-drive-setting = Vuelta a sin calibración: el ajuste de Excitación llega entero a todas las bandas.
settings-applies-immediately-band-you-would = Se aplica de inmediato, y a la banda en la que transmitirías — detrás de un transversor es la banda del dial, y el límite de excitación propio del transversor sigue imponiéndose a cualquier valor fijado aquí.
settings-settings = Ajustes
settings-enter-server-s-address-first = introduce primero la dirección del servidor
settings-enter-receiver-s-address-first = introduce primero la dirección del receptor, o elige uno de SDR públicos
settings-no-radio-selected-press-discover = ninguna radio seleccionada — pulsa Descubrir, o introduce una dirección
settings-enter-radio-s-address-first = introduce primero la dirección de la radio
settings-general = General
settings-radio = Radio
settings-ui = Interfaz
settings-alerts = Alertas
settings-controls = Controles
settings-spots = Spots
settings-freedv = FreeDV
settings-uploads = Subidas
settings-winlink = Winlink
settings-servers = Servidores
settings-t-r-switch = Conmutador T/R
settings-amplifier = Amplificador
settings-remote = Remoto
settings-tle = TLE
settings-profiles = Perfiles
settings-station = Estación
settings-enter-digital-mode-ft8-sstv = Entra una vez en un modo digital (FT8 / SSTV / …) para cargar los valores guardados.
settings-callsign = Indicativo
settings-grid-square = Locátor
settings-your-callsign-grid-shared-across = Tu indicativo y locátor, compartidos entre FT8/FT4/FT2, las cabeceras de imagen SSTV y el diario de guardia. También editables desde el diálogo de configuración de FT8 / SSTV.
settings-iaru-region = Región IARU
settings-which-radio-interface-server-uses = La interfaz de radio que usa el servidor se configura en la máquina donde se ejecuta.
settings-radio-interface = Interfaz de radio
settings-waiting-machine-radio-attached-interface = Esperando a la máquina a la que está conectada la radio. Sus ajustes de interfaz están abajo y mientras tanto pueden cambiarse desde aquí.
settings-converter = Conversor
settings-manual-type-offset-below = Manual — escribe un desplazamiento abajo
settings-what-box-reads-when-offset = Lo que muestra esta casilla cuando el desplazamiento no es uno de los preajustes. Se fija escribiendo en el campo Desplazamiento de abajo; aquí no hay nada que seleccionar.
settings-frequency-converter-front-receiver-pick = Un conversor de frecuencia delante del receptor. Elige uno, o escribe un desplazamiento a su lado.
settings-how-far-converter-moves-signal =
    Cuánto desplaza un conversor la señal en su camino hacia el receptor, en Hz — el mismo número y signo que indica la documentación de cualquier conversor y todos los demás programas SDR. La radio acaba en dial + desplazamiento: positivo para un conversor ascendente (un Ham It Up es 125000000), negativo para cualquier cosa que baje una banda a una FI — un LNB de satélite, o un transversor de 2 m a una FI de 28 MHz, que es -116000000 (28 − 144). 0 = sin conversor.

    Arrastra para ajustarlo hercio a hercio, que es lo que necesita un conversor cuyo oscilador está ligeramente desviado.

    Este es el camino de recepción. Lo que hay en la línea de transmisión es la fila de abajo.

    Surte efecto con Aplicar.
settings-what-transmit-line-while-converter =
    Qué hay en la línea de transmisión mientras hay un conversor configurado.

    Desactivado al convertir: no se transmite nada — el valor seguro por omisión, y correcto para un accesorio solo de recepción.

    Por el mismo conversor: una caja convierte en ambos sentidos, un transversor. La transmisión toma el desplazamiento de arriba y lo sigue cuando se ajusta.

    Desplazamiento propio: la línea de transmisión es distinta de la de recepción. Cero es el caso común — un conversor (un LNB, un Ham It Up) en recepción con el transmisor en su propia antena, que es la estación de QO-100.

    La comprobación de banda de aficionado sigue aplicándose, sobre la frecuencia en la que transmites.

    Surte efecto con Aplicar.
settings-how-far-transmit-line-moves =
    Cuánto desplaza la línea de transmisión la señal, en Hz, con la misma regla de signo que el desplazamiento de recepción: la radio transmite en dial + desplazamiento.

    0 = el transmisor está conectado a su propia antena y trabaja en la frecuencia que dice el dial — el enlace ascendente de 2,4 GHz de una estación de QO-100.

    Un conversor de transmisión que lleva una FI *hacia arriba* es un número negativo: la radio trabaja por debajo del dial.
settings-rx-range = Rango RX
settings-which-frequencies-radio-receives-mhz =
    Qué frecuencias recibe esta radio, en MHz: 144-146, 430-440. Déjalo vacío para usar lo que el dispositivo informe de sí mismo.

    Los botones de banda fuera del rango aparecen atenuados y el dial no irá allí.

    Son frecuencias de dial. Con un conversor configurado, indica la banda que sintonizas — 144-148 para un transversor de 2 m — no la FI en la que está realmente la radio.

    Surte efecto con Aplicar.
settings-tx-range = Rango TX
settings-which-frequencies-radio-transmits-mhz =
    En qué frecuencias transmite esta radio, en MHz: 144-146, 430-440. Déjalo vacío para usar lo que informe el dispositivo — y si no informa de nada, se toma al pie de la letra al controlador y se permite cualquier frecuencia.

    Frecuencias de dial, como el rango de recepción de arriba.

    Es un límite que tú fijas, no una licencia: transmitir fuera de las bandas de aficionado se rechaza en cualquier caso salvo que lo hayas desactivado en config.toml. Tampoco da un transmisor a un dispositivo de solo recepción.

    Surte efecto con Aplicar.
settings-antenna = La antena está
settings-where-radio-listens-station-antenna =
    Desde dónde escucha esta radio.

    En la estación: la antena es tuya, así que tu propio locátor dice dónde está. El valor predeterminado, y correcto para todo lo que está en la estación.

    En otro lugar: un receptor en línea, o uno tuyo montado en una colina. Lo que oye se informa a PSK Reporter, WSPRnet y FreeDV Reporter desde el locátor de al lado, nunca desde el tuyo — y ADS-B también sitúa los aviones respecto a él.

    Elegir un receptor en «SDR públicos» rellena esto por ti.

    Surte efecto con Aplicar.
settings-receiver-s-maidenhead-locator-jn88ec =
    El locátor Maidenhead del receptor — JN88ec, DO30db. Cuatro o seis caracteres.

    Déjalo vacío si no sabes dónde está el receptor: entonces no se informa de nada, que es la única respuesta honesta. Informar desde tu propio cuadrado pondría la recepción de otra persona en el continente equivocado.
settings-audio = Audio hacia
settings-gain-per-band = Ganancia por banda
settings-remember-receive-gain-per-band = Recordar la ganancia de recepción por banda
settings-each-band-keeps-receive-gain = Cada banda conserva la ganancia de recepción que fijaste en ella por última vez y la recupera cuando el dial vuelve — un botón de banda, un arrastre a través del borde de una banda o el mando de la propia radio. La cobertura general se divide en HF, VHF, UHF y superiores. Una banda en la que no has fijado ganancia conserva la que esté vigente. Desactivado: una sola ganancia en todas partes.
settings-s-meter-calibration = Calibración del S-metro
settings-added-radio-s-s-meter =
    Se suma a la lectura del S-metro de esta radio. El medidor de un SDR empieza leyendo dBFS — nivel respecto al fondo de escala del conversor — así que un receptor con mucha ganancia puede mostrar S9 sin antena conectada. Fíjalo a la diferencia entre lo que muestra el medidor y un nivel que conozcas: un generador de señal, o una banda tranquila que sepas que ronda S2–S3. Se aplica al instante.

    Se suma a cal_offset_db de config.toml, que es una única cifra para toda la estación.
settings-t-0-db = 0 dB
settings-antenna-somewhere-else-no-locator = La antena está en otro lugar y no hay locátor configurado, así que nada de lo que oye esta radio se informa a PSK Reporter, WSPRnet ni FreeDV Reporter. Escribe arriba el locátor del receptor para informar desde él.
settings-ranges-are-mhz-low-high = Los rangos están en MHz, bajo-alto, separados por comas: 144-146, 430-440 — es decir, 144000000-146000000 Hz y 430000000-440000000 Hz. Son frecuencias de dial: un desplazamiento de conversor no las mueve. El desplazamiento del conversor de arriba es el campo en hercios. Deja un rango vacío para usar lo que el dispositivo informe de sí mismo; un dispositivo que no informa de nada se toma al pie de la letra.
settings-careful-rtl-sdr-blog-v4 = Cuidado con un RTL-SDR: el Blog V4 convierte hacia arriba por sí solo por debajo de 28,8 MHz, así que un desplazamiento negativo que deje el hardware allí desplaza dos veces.
settings-transmit-audio = Audio de transmisión
settings-use = en uso
settings-every-stage-off = todas las etapas desactivadas
settings-open-tx-audio = Abrir Audio TX…
settings-microphone-processing-chain-gate-eq = La cadena de procesado del micrófono — puerta, ecualizador, de-esser, compresor y limitador — con una grabación de prueba para ajustarla de oído sin transmitir. Solo modos de voz. También se abre haciendo clic en Mic en el recuadro TX.
settings-pick-radio-interface-above-configuration = Elige arriba una interfaz de radio (esta configuración usaba el modo de autodetección, que se ha eliminado).
settings-configuration-used-removed-auto-detect = Esta configuración usaba el modo de autodetección, que se ha eliminado. Elige una interfaz arriba cuando la máquina a la que está conectada la radio haya respondido.
settings-radio-has-no-interface-yet = Esta radio aún no tiene interfaz — elige una arriba y pulsa Aplicar / reconectar.
settings-radio-has-no-interface-yet-2 = Esta radio aún no tiene interfaz. Se podrá elegir una arriba cuando la máquina a la que está conectada la radio haya respondido.
settings-apply-reconnect = Aplicar / reconectar
settings-switch-interface-now-no-restart = Cambiar ahora a esta interfaz — sin reiniciar
settings-switch-server-s-radio-interface = Cambia ahora la radio del servidor a esta interfaz — sin reiniciar. La anterior sigue funcionando si no se puede abrir la nueva.
settings-switches-live-radio-without-restarting = Cambia la radio en vivo sin reiniciar.
settings-everything-above-server-s-own = Todo lo de arriba es la configuración propia del servidor, y los cambios se guardan allí. La mayoría de los ajustes se aplican al cambiarlos; los que se fijan al abrir el dispositivo — la frecuencia de muestreo, una dirección, la propia interfaz — necesitan este botón.
settings-dx-cluster-telnet = Clúster DX (telnet)
settings-enabled = Activado
settings-host = Servidor
settings-port = Puerto
settings-login-call = Indicativo de acceso
settings-commands = Comandos
settings-one-per-line-e-g = uno por línea, p. ej. SET/FT8
settings-reverse-beacon-network = Reverse Beacon Network
settings-read-world-s-cw-rtty = Lee los skimmers de CW/RTTY de todo el mundo y alimenta con ellos el mapa de propagación. Es lo que hace que el mapa muestre bandas que esta radio no está escuchando. Activado de forma predeterminada: no emite nada, no requiere cuenta y usa el indicativo de la pestaña General. Los spots de RBN no aparecen en la lista de spots — son mediciones, no invitaciones.
settings-t-7000-cw-rtty-feed-7001 = 7000 es el flujo de CW/RTTY, 7001 el de FT8/FT4
settings-one-per-line-e-g-2 = uno por línea, p. ej. set/filter cont=eu
settings-sent-after-login-place-narrow = Se envía tras iniciar sesión. El lugar para acotar el flujo — sin filtro son todos los skimmers de la Tierra.
settings-rbn-paths-are-placed-country = Los trayectos de RBN se sitúan a partir de centros de país, no de locátores — precisos para un país pequeño, muy desviados para uno grande. Son una capa aparte en el mapa de propagación y pueden desactivarse allí.
settings-pota-sota-psk-reporter = POTA / SOTA / PSK Reporter
settings-pota-activator-spots = Spots de activadores POTA
settings-sota-spots = Spots SOTA
settings-psk-reporter-current-band = PSK Reporter (banda actual)
settings-upload-my-ft8-ft4-ft2 = Subir mis decodificaciones de FT8/FT4/FT2/JS8
settings-report-what-station-hears-pskreporter = Informa a pskreporter.info de lo que oye esta estación, para que aparezca allí como receptor. Usa el indicativo y el locátor de la pestaña General.
settings-antenna-2 = Antena
settings-collector = Colector
settings-t-4739-live-collector-14739-test = 4739 es el colector real, 14739 el de pruebas
settings-max-age-s = Antigüedad máx. (s)
settings-wsprnet = WSPRnet
settings-upload-my-wspr-decodes = Subir mis decodificaciones de WSPR
settings-send-every-wspr-reception-wsprnet = Envía cada recepción de WSPR a wsprnet.org. Activado de forma predeterminada: no emite nada, e informar de lo que oyes es lo que hace de un receptor WSPR parte de la red en lugar de una curiosidad privada. También se informa de una ranura que no decodificó nada, que es como la red distingue una banda cerrada de un receptor apagado.
settings-download-who-heard-me = Descargar quién me ha oído
settings-ask-wsprnet-org-which-stations = Pregunta a wsprnet.org qué estaciones decodificaron esta. WSPR no tiene ningún tipo de acuse de recibo, así que esta es la única forma en que una baliza transmisora se entera de su alcance.
settings-ask-every = Preguntar cada
settings-looking-back = mirando atrás
settings-apply = {" "}APLICAR{" "}
settings-persist-re-connect-feeds = Guardar y (re)conectar los flujos
settings-broadcast-stations = Emisoras de radiodifusión
settings-winlink-account = Cuenta de Winlink
settings-password = Contraseña
settings-winlink-account-password-not-gateway = La contraseña de la cuenta de Winlink, no la de la pasarela. Distingue mayúsculas de minúsculas — introdúcela exactamente como se emitió.
settings-locator = Locátor
settings-how-connect = Cómo conectar
settings-route = Ruta
settings-internet = Internet
settings-forward-cms-over-internet = Reenviar con el CMS por internet
settings-radio-packet = Radio (packet)
settings-call-rms-gateway-air-radio = Llama a una pasarela RMS por el aire. La radio debe estar en PACKET o PACKET-HF.
settings-gateway = Pasarela
settings-via = Vía
settings-digipeaters-order-separated-spaces-usually = Digipeaters, en orden, separados por espacios. Normalmente vacío — una pasarela que puedes oír directamente es una pasarela a la que deberías llamar directamente.
settings-speed = Velocidad
settings-most-rms-gateways-answer-1200 = La mayoría de las pasarelas RMS responden a 1200. Llamar a una pasarela de 1200 a 9600 suena exactamente igual que una pasarela fuera del aire, así que fíjalo según lo que publique el responsable de la pasarela en lugar de adivinar. 9600 también necesita el puerto de datos de la radio en ambos extremos. 300 es HF.
settings-frequency = Frecuencia
settings-clear = BORRAR
settings-zero-leaves-dial-alone-which = Cero deja el dial en paz, que es lo que quieres cuando te quedas en un solo canal. Cualquier otro valor sintoniza la radio al iniciar la sesión.
settings-my-gateways = Mis pasarelas
settings-winlink-s-published-gateway-list = La lista de pasarelas que publica Winlink necesita una clave de API que sdroxide no tiene, así que lleva tu propia lista. Las dos o tres pasarelas alcanzables desde un lugar se averiguan probando y rara vez cambian.
settings-use-2 = USAR
settings-call-one-next-connect = Llamar a esta en la próxima conexión
settings-current-dial = dial actual
settings-forget = OLVIDAR
settings-forget-gateway = Olvidar esta pasarela
settings-add-gateway = + AÑADIR PASARELA
settings-save-gateway-above-list = Guardar en la lista la pasarela de arriba
settings-internet-gateway = Pasarela de internet
settings-cms-address = Dirección del CMS
settings-client-name = Nombre del cliente
settings-winlink-s-production-servers-only = Los servidores de producción de Winlink solo aceptan nombres de cliente que conocen, y a uno desconocido le responden «Unknown client types are not allowed on production servers». Hasta que sdroxide esté registrado en el Winlink Development Team, conectar necesita un nombre que reconozcan.
settings-automatic-connection = Conexión automática
settings-connect-timer = Conectar con un temporizador
settings-every = Cada
settings-persist-winlink-account = Guardar la cuenta de Winlink
settings-callsign-lookup = Consulta de indicativos
settings-provider = Proveedor
settings-auto-fill-name-qth-grid = Rellenar nombre/QTH/locátor al hacer clic en un spot y en el QSO
settings-choose-provider-enter-login = Elige un proveedor para introducir su acceso.
settings-qrz-user = Usuario QRZ
settings-qrz-pass = Clave QRZ
settings-hamqth-user = Usuario HamQTH
settings-hamqth-pass = Clave HamQTH
settings-one-hamqth-account-does-both = Una cuenta de HamQTH sirve para ambos — este acceso es el que usa la pestaña de subida a HamQTH.
settings-upload = Subir
settings-auto-upload-each-new-qso = Subir automáticamente cada QSO nuevo
settings-master-switch-which-services-qso = El interruptor general. A qué servicios va un QSO se configura en cada pestaña de abajo.
settings-turn-auto-upload-each-new = Activa primero «Subir automáticamente cada QSO nuevo» — un servicio marcado aquí no recibe nada hasta entonces.
settings-auto-upload-off-above-so = La subida automática está desactivada arriba, así que las marcas de los servicios están inhabilitadas y todavía no se envía nada automáticamente — el botón UP de cada QSO en el diario sigue funcionando.
settings-qrz-log-key = Clave de log de QRZ
settings-qrz-logbook-api-key-your = La clave de API del diario de QRZ, de los ajustes de tu diario en QRZ — no el acceso de consulta XML.
settings-eqsl-user = Usuario eQSL
settings-eqsl-pass = Clave eQSL
settings-eqsl-qth = QTH de eQSL
settings-qth-nickname-eqsl-profile-upload = El apodo de QTH del perfil de eQSL al que subir y del que descargar — solo hace falta cuando la cuenta tiene más de uno; si no, déjalo en blanco.
settings-same-login-hamqth-callsign-lookup = El mismo acceso que la consulta de indicativos de HamQTH — rellenar uno rellena los dos.
settings-club-log-email = Correo de Club Log
settings-club-log-pass = Clave de Club Log
settings-club-log-key = Clave de Club Log
settings-wrl-api-key = Clave de API de WRL
settings-developer-api-key-world-radio = La clave de API de desarrollador de World Radio League → Integrations → Developer API. Se muestra una sola vez al generarla, así que cópiala entonces. Los contactos van a tu diario predeterminado — fija uno en WRL si llevas más de uno.
settings-confirmations-download = Confirmaciones (descarga)
settings-lotw-user = Usuario LoTW
settings-lotw-pass = Clave LoTW
settings-lotw-upload-uses-tqsl-export = La subida a LoTW usa TQSL — exporta ADIF desde el diario y fírmalo. Las confirmaciones de LoTW/eQSL se descargan aquí para marcar trabajado frente a confirmado.
settings-sync-confirmations = SINCRONIZAR CONFIRMACIONES
settings-fetching-subscriptions = Obteniendo suscripciones…
settings-spectrum-waterfall-everything-read-off =
    Su espectro, su cascada y todo lo que se lee de ellos están en la pestaña de esa radio, por lo que esta no tiene los suyos. Los ajustes de interfaz de arriba siguen siendo de esta radio y se siguen aplicando — pulsa Aplicar / reconectar tras cambiarlos y las dos radios se reinician juntas.

    Para usarla de nuevo por separado, vacía la casilla Receptor del panadaptador en la página de esa radio.
settings-panadapter = Panadaptador
settings-give-radio-wideband-panadapter-borrowing = Da a esta radio un panadaptador de banda ancha tomando prestado el receptor de otra radio — un SDR en la misma antena, o uno alimentado desde la salida de FI de esta radio. El espectro, la cascada, el subreceptor, los modos digitales y el skimmer provienen de ese receptor; el dial, el modo, el filtro y el transmisor se quedan aquí.
settings-receiver = Receptor
settings-none = Ninguno
settings-which-radio-receives-one-leaves =
    Qué radio recibe para esta. Sale de la tira de pestañas mientras se presta — su etapa de entrada pertenece al motor de esta radio — y vuelve de inmediato cuando se pone en Ninguno.

    Surte efecto con Aplicar.
settings-connected = Conectado a
settings-how-far-above-radio-s =
    A cuánto por encima del dial de esta radio está el receptor, en hercios — 0 para un receptor en la misma antena. Para una toma de FI, la frecuencia intermedia en la que está aparcado el receptor: 8215000 para el KXV3B de un K3, 9000000 para una FI de 9 MHz.

    No es el campo Conversor de arriba, que resintoniza la radio, ni el desplazamiento del centro I/Q de la pestaña CAT, que trata de la propia tarjeta de sonido de esta radio. Nada de lo que se escriba aquí se envía nunca a ninguna de las dos radios.

    Surte efecto con Aplicar.
settings-audio-2 = Audio de
settings-follow-dial = Seguir el dial
settings-keep-receiver-radio-s-dial = Mantiene el receptor en el dial de esta radio y mueve el dial cuando sintonizas en el panadaptador. Desactivado deja el receptor donde está — una vigilancia fija de un segmento mientras la radio va a otra parte.
settings-invert-spectrum = Invertir el espectro
settings-mirror-receiver-s-span-about = Refleja el ancho del receptor respecto a su centro, para una toma cuyo oscilador está por encima de la señal y entrega la banda al revés. Déjalo desactivado salvo que la cascada se llene de señales que están todas del lado equivocado del dial.
settings-mute-transmit = Silenciar al transmitir
settings-silence-receive-audio-while-radio = Silencia el audio de recepción mientras esta radio transmite. Activado de forma predeterminada: con el receptor en la misma antena, o en la FI de esta radio, lo que oye durante un turno es tu propio transmisor. Los turnos activados en la propia radio — el PTT del micrófono, un pedal, MOX — también cuentan, en cuanto el enlace CAT dice que el equipo ha pasado a transmitir.
settings-blank-transmit = Borrar al transmitir
settings-stop-panadapter-waterfall-while-radio = Detiene el panadaptador y la cascada mientras esta radio transmite. Activado de forma predeterminada: un transmisor pintado en todo el ancho borra la banda que hay detrás. Desactívalo para ver la banda — o tu propia señal — durante un turno, lo que merece la pena con el receptor en una antena aparte. Se aplica a los turnos que activa sdroxide: uno activado en la propia radio se recibe de punta a punta, así que la imagen sigue corriendo diga lo que diga esto.
settings-per-mode-offsets-radio-whose = Desplazamientos por modo, para una radio cuya FI se mueve con el modo. Un modo vacío usa el desplazamiento de arriba.
settings-computer = En este ordenador
settings-add-radio-computer = Añadir una radio en este ordenador
settings-radio-s-settings-are-below = Los ajustes de esta radio están abajo. Arrastra para moverla por la tira.
settings-switch-radio-dialog-follows-drag = Cambiar a esta radio (el diálogo la sigue). Arrastra para moverla por la tira.
settings-tx = ● TX
settings-mute-radio-s-audio = Silenciar el audio de esta radio
settings-link = ENLACE
settings-close-radio-configuration-kept = Cerrar esta radio (se conserva su configuración)
settings-close-tab-close-radio-station = Cerrar esta pestaña, o cerrar la radio en su estación
settings-close-tab = Cerrar esta pestaña
settings-transverters-full = { $max } es el máximo que admite esta tabla
settings-dialling = Marcando { $url }…
settings-region-note = Dónde está la estación. Fija todos los planes de banda: los límites de banda, los subsegmentos de CW / datos / voz de la franja de la cascada, dónde escuchan los skimmers, las frecuencias de llamada que se ofrecen y lo que cuenta como fuera de banda para transmitir. En la Región { $region } eso hace que 70 cm sea { $region ->
        [1] 430–440 MHz — así que 446 está fuera de banda, y 40 m termina en 7,200
        [2] 420–450 MHz, y 40 m llega hasta 7,300
       *[3] 430–450 MHz, y 80 m termina en 3,900
    }. Surte efecto al instante y se aplica a todas las radios de esta estación.
settings-rx-grid-reported = Las recepciones a través de esta radio se informan desde { $grid }, no desde tu propio locátor.
settings-rx-grid-invalid = { $grid } no es un locátor Maidenhead, así que nada de lo que oye esta radio se informa. Cuatro o seis caracteres: dos letras, dos dígitos y, opcionalmente, dos letras más.
settings-bypassed = omitida
settings-tx-audio-chain = Cadena { $state }.
settings-gateway-via = { " " }vía { $via }
settings-auto-upload-to = Subir automáticamente cada QSO nuevo a { $service }
settings-tle-updated = { $n } { $n ->
        [one] suscripción actualizada
       *[other] suscripciones actualizadas
    }: { $total } satélites.
settings-tle-updated-failed = Actualizadas { $ok } de { $n }; { $failed } con error — mira las filas de arriba.
settings-panadapter-of = Esta radio es el panadaptador de { $radio }
settings-radio-n = radio { $n }
settings-on-station = En { $station }
settings-add-radio-on-station = Añadir una radio en { $station } — la estación la sirve de inmediato
settings-panadapter-receiver-for = Receptor del panadaptador de { $radio }
settings-hang-up-stays-on = Colgar. La radio sigue en { $station }.
settings-close-on-station = ¿Cerrar { $radio } en { $station }? Su configuración se queda en esa máquina; la pestaña de aquí se cierra con ella, y también la de todos los demás.
settings-close-it-on = Cerrarla en { $station }
settings-locator-hint = locátor

## Settings window, Alerts (src/app/settings/alerts.rs)

settings-alerts-audible-alerts = Alertas sonoras
settings-alerts-sound-alarm-when-decode-matters = Hacer sonar una alarma cuando una decodificación importa
settings-alerts-plays-over-own-audio-output = Se reproduce por su propia salida de audio, así que se oye incluso cuando la banda suena en un altavoz distinto al de esta pantalla.
settings-alerts-volume = Volumen
settings-alerts-output = Salida
settings-alerts-test = Probar
settings-alerts-what-sound = Qué hacer sonar
settings-alerts-each-station-quiet-while-after = Cada estación calla un rato tras una alerta, para que una banda concurrida no suene en cada ranura. Voz y Tono + voz los lee la voz de los avisos hablados — actívala en Ajustes → Interfaz.

## Settings window, Amplifier (src/app/settings/amp.rs)

settings-amp-amplifier = Amplificador
settings-amp-waiting-station-s-amplifier-configuration = Esperando la configuración del amplificador de la estación…
settings-amp-hobbypcb-hardrock-500-hardrock-50 = Un HobbyPCB Hardrock-500 o Hardrock-50 en su puerto serie USB. SDRoxide lee sus medidores, cambia su banda, modo y antena, y maneja su acoplador desde la ventana AMP. El amplificador sigue activándose con la línea PTT de la radio como antes.
settings-amp-enable = Activar
settings-amp-model = Modelo
settings-amp-untested-panel-has-only-been = Sin probar: este panel solo se ha probado con un Hardrock-500. Los límites de los medidores de este modelo se escalan a partir de los del Hardrock-500 y pueden ser erróneos. Vigila la pantalla del propio amplificador.
settings-amp-port = Puerto
settings-amp-choose = — elegir —
settings-amp-baud = Baudios
settings-amp-must-match-amplifier-s-menu = Debe coincidir con el menú del amplificador. El panel de control de HobbyPCB necesita 115200.
settings-amp-follow-radio-s-band = Seguir la banda de la radio
settings-amp-sends-transmit-frequency-amplifier-every = Envía la frecuencia de transmisión al amplificador en cada cambio de banda, de modo que sus filtros siguen a la radio sin un cable CAT propio. Nunca se envía durante una transmisión.
settings-amp-show-power-swr-tx-meter = Mostrar su potencia y ROE en el medidor TX
settings-amp-only-where-radio-does-not = Solo donde la radio no los mide por sí misma, y solo mientras el amplificador está en línea.
settings-amp-off = Desactivado
settings-amp-connecting = Conectando…
settings-amp-status = Estado
settings-amp-apply = {" "}APLICAR{" "}
settings-amp-save-re-open-amplifier-s = Guardar y (re)abrir el puerto del amplificador

## Settings window, Controls (src/app/settings/controls.rs)

settings-controls-keyboard = Teclado
settings-controls-click-shortcut-rebind-then-press = Haz clic en un atajo para reasignarlo y luego pulsa la combinación de teclas (Esc cancela). Las asignaciones se ignoran mientras escribes en un campo de texto.
settings-controls-shortcut = Atajo
settings-controls-does = Hace
settings-controls-step-mode = Paso / modo
settings-controls-accel = Aceler.
settings-controls-on = Act.
settings-controls-press-key = pulsa una tecla…
settings-controls-remove-binding = Quitar esta asignación
settings-controls-add-shortcut = + Añadir atajo
settings-controls-restore-defaults = Restaurar valores predeterminados
settings-controls-bind-hold-talk-space = Asignar el pulsar-para-hablar a la barra espaciadora
settings-controls-hold-space-transmit-releasing-losing = Mantén la barra espaciadora para transmitir; al soltarla — o al perder el foco de la ventana — se desactiva
settings-controls-unkey-held-ptt-after = Desactivar un PTT mantenido tras
settings-controls-backstop-against-stuck-key-controller = Red de seguridad contra una tecla atascada o un controlador que deja de informar. 0 lo desactiva.
settings-controls-panadapter-mouse = Ratón en el panadaptador
settings-controls-wheel = Rueda
settings-controls-wheel-shift = Rueda + Mayús
settings-controls-tune-step = Paso de sintonía
settings-controls-zoom-rate = Velocidad de zoom
settings-controls-click-tune-rounding = Redondeo de la sintonía por clic
settings-controls-invert-wheel-direction = Invertir el sentido de la rueda
settings-controls-left-drag-tunes-well-pans = Arrastrar con el botón izquierdo sintoniza además de desplazar
settings-controls-off-makes-left-drag-pan = Desactivado hace que arrastrar con el izquierdo solo desplace la vista, como con el derecho.
settings-controls-scroll-digit-frequency-readout-tune = Desplazar un dígito de la lectura de frecuencia para sintonizar
settings-controls-both-wheel-actions-are-tune = Las dos acciones de la rueda son Sintonizar — ya no queda forma de hacer zoom.
settings-controls-restore-mouse-defaults = Restaurar los valores predeterminados del ratón
settings-controls-mouse-buttons = Botones del ratón
settings-controls-left-right-buttons-are-reserved = Los botones izquierdo y derecho están reservados para sintonizar y desplazar; el central y los adicionales están libres. Un botón lateral mantenido para PTT se comporta como un pedal.
settings-controls-add-mouse-button = + Añadir botón del ratón
settings-controls-f1-always-opens-manual-even = F1 siempre abre este manual, incluso al escribir, así que no se puede reasignar.
settings-controls-midi-controller = Controlador MIDI
settings-controls-midi-controllers-need-native-app = Los controladores MIDI necesitan la aplicación nativa — el cliente de navegador no tiene acceso a MIDI.
settings-controls-any-class-compliant-midi-surface = Sirve cualquier superficie MIDI class-compliant: la rueda jog de un controlador de DJ hace un buen mando de VFO, sus pads hacen de PTT y botones de banda, sus faders hacen de controles de ganancia.
settings-controls-enable = Activar
settings-controls-controller = Controlador
settings-controls-feedback = Retorno a
settings-controls-rescan-ports = Volver a buscar puertos
settings-controls-not-connected = No conectado.
settings-controls-move-control-see-here = Mueve un control para verlo aquí.
settings-controls-control = Control
settings-controls-reads = Se lee como
settings-controls-led = LED
settings-controls-move = muévelo…
settings-controls-click-then-move-control-you = Haz clic y luego mueve el control que quieres asignar
settings-controls-speed-sensitivity-spin-faster-tune = Sensibilidad a la velocidad: gira más rápido para sintonizar más rápido
settings-controls-send-current-value-back-light = Devolver el valor actual, para encender un LED o mover un fader
settings-controls-add-midi-control = + Añadir control MIDI
settings-controls-clear-all = Borrar todo
settings-controls-endless-jog-encoders-send-relative = Los codificadores sin fin (jog) envían un paso relativo en lugar de una posición, en una de tres codificaciones que se parecen en los movimientos pequeños. APRENDER lo adivina con un giro en sentido horario; si luego el mando sintoniza al revés, marca «inv».
settings-controls-none = — ninguno —

## Settings window, General (src/app/settings/general.rs)

settings-general-system-default = Predeterminado del sistema
settings-general-remote-access = Acceso remoto
settings-general-what-remote-client-browser-page = Lo que un cliente remoto — la página del navegador, u otro sdroxide iniciado con --connect — tiene que presentar para que esta estación le deje operar. Se aplica en modo servidor (--server); el siguiente inicio de sesión recoge el cambio, sin reiniciar.
settings-general-username = Usuario
settings-general-password = Contraseña
settings-general-clients-must-give-password-leaving = Los clientes deben dar la contraseña. Dejar el usuario vacío está bien.
settings-general-clients-must-sign = Los clientes deben iniciar sesión.
settings-general-empty-anyone-who-can-reach = ⚠ Vacío: cualquiera que alcance el puerto del servidor puede operar esta radio, con tu indicativo. Fija una contraseña antes de redirigir el puerto.
settings-general-stored-clear-config-toml-like = Guardada en claro en config.toml, como las demás contraseñas que guarda sdroxide.
settings-general-receive-audio-gain = Ganancia del audio de recepción
settings-general-t-0-db = 0 dB
settings-general-extra-gain-everything-radio-sends =
    Ganancia extra sobre todo lo que esta radio envía a los altavoces, además del control de volumen. Déjala en 0 dB salvo que la radio suene baja a volumen máximo: el tope del control de volumen es el audio tal como llega, así que puede bajar una radio pero nunca subirla, y la salida de sonido USB de algunos transceptores queda muy por debajo del fondo de escala.

    Sube de 6 en 6 dB. Demasiada recorta — el audio se limita en el fondo de escala en lugar de dejar que dé la vuelta, así que pasarse suena áspero más que fuerte. Las grabaciones se toman antes de esto y no se ven afectadas.
settings-general-reload-band-plan = RECARGAR PLAN DE BANDAS
settings-general-re-read-bandplan-json-machine = Vuelve a leer bandplan.json en la máquina a la que está conectada la radio y lo aplica — límites de banda, subsegmentos y ventanas del skimmer — sin reiniciar.
settings-general-running-station-s-own-band = Usando el plan de bandas propio de la estación.
settings-general-running-built-iaru-tables = Usando las tablas IARU integradas.
settings-general-settings-file = Archivo de ajustes
settings-general-settings-are-machine-radio-attached = Los ajustes están en la máquina a la que está conectada la radio. Expórtalos allí.
settings-general-export = EXPORTAR…
settings-general-write-every-setting-station-radios = Escribe todos los ajustes de esta estación — las radios, los modos, los servidores, las memorias, el plan de bandas — en un único archivo que puedes llevar a otra instalación. Tu diario y cualquier contraseña de servidor guardada se quedan aquí.
settings-general-import = IMPORTAR…
settings-general-replace-station-s-settings-ones = Sustituye los ajustes de esta estación por los de un archivo exportado desde otra instalación. Reinicia sdroxide después.
settings-general-import-overwrites-what-here-file = Una importación sobrescribe lo que hay aquí, archivo por archivo, y surte efecto la próxima vez que se inicie sdroxide — de lo contrario los ajustes que ya están en memoria se volverían a escribir encima. Lo que el archivo no menciona se deja como está, así que un paquete de una estación con una sola radio no elimina una segunda radio de aquí. Tu diario nunca está en el archivo: expórtalo como ADIF desde la ventana LOG si quieres llevarte también eso.
settings-general-sdroxide-settings = Ajustes de sdroxide
settings-general-swr-guard = Protección de ROE
settings-general-stop-transmitting-high-swr = Dejar de transmitir con ROE alta
settings-general-trip = Saltar en
settings-general-stops-transmission-when-radio-reports =
    Detiene la transmisión cuando la radio informa de una ROE igual o superior a esta cifra, y mantiene la transmisión bloqueada hasta que lo confirmes. Detecta una antena desconectada, una línea de alimentación averiada o un conmutador que se ha dejado en el puerto equivocado.

    La sintonización se trata de otro modo, porque alimentar una desadaptación es justo su objetivo: un acoplador de antena recibe el doble del límite y unos cinco segundos antes de que la protección se aplique siquiera. Un acoplador manual que tarde más que eso necesita la protección desactivada durante la sesión.

    Requiere un equipo que informe de la ROE por CAT. Ignora la primera quinta parte de segundo de cada transmisión, y no espera a que haya mucha potencia.
settings-general-per-mode-settings = Ajustes por modo
settings-general-reset-every-mode = RESTABLECER TODOS LOS MODOS
settings-general-forget-every-mode-s-remembered = Olvida el AGC, el silenciador, la reducción de ruido, el notch y los interruptores de estéreo recordados de cada modo, y devuelve los valores predeterminados propios de cada modo.
settings-general-selecting-mode-lays-mode-s = Seleccionar un modo aplica al receptor los valores iniciales propios de ese modo para AGC, silenciador, reducción de ruido, notch e interruptores de estéreo, y cambiar uno lo recuerda solo para ese modo. El chip de flecha circular al final de la fila de filtro/ruido del receptor lista lo que se ha cambiado y devuelve el modo actual a su estado inicial.
settings-general-your-audio-speakers-microphone = Tu audio (altavoces / micrófono)
settings-general-output = Salida
settings-general-channels = Canales
settings-general-which-speaker-channel-s-received = A qué canal(es) de altavoz llega el audio recibido. Izquierdo o derecho pone todo, mezclado a mono, solo en ese lado.
settings-general-input = Entrada

## Settings window, network services (src/app/settings/net.rs)

settings-net-operator = Operador
settings-net-callsign-grid-are-set-general = El indicativo y el locátor se fijan en la pestaña General.
settings-net-set-your-callsign-grid-general = ⚠ Fija tu indicativo y locátor en la pestaña General.
settings-net-freedv-reporter = FreeDV Reporter
settings-net-enable = Activar
settings-net-connects-whenever-enabled-your-station = Se conecta siempre que esté activado. Tu estación solo se muestra a los demás mientras la radio está en RADE — en cualquier otro modo sigues conectado pero oculto.
settings-net-station = Estación
settings-net-message = Mensaje
settings-net-receive-only-i-cannot-transmit = Solo recepción (no puedo transmitir)
settings-net-server = Servidor
settings-net-host = Servidor
settings-net-port = Puerto
settings-net-tls-wss = TLS (wss://)
settings-net-not-yet-implemented-freedv-gui = Aún no implementado — FreeDV GUI también usa ws:// sin cifrar.
settings-net-reporting = Informes
settings-net-report-stations-i-decode = Informar de las estaciones que decodifico
settings-net-sends-rx-report-each-callsign = Envía un rx_report por cada indicativo recuperado de una trama de fin de turno de RADE.
settings-net-show-other-reporter-stations-spots = Mostrar otras estaciones de Reporter como spots
settings-net-adds-them-panadapter-overlay-world = Las añade a la superposición del panadaptador, al mapamundi y a la ventana SPOTS bajo el filtro FREEDV.
settings-net-set-your-callsign-grid-general-2 = ⚠ Fija tu indicativo y locátor en la pestaña General. Sin ambos, la conexión es de solo lectura: verás a otras estaciones, pero tú no aparecerás.
settings-net-apply = {" "}APLICAR{" "}
settings-net-persist-re-connect = Guardar y (re)conectar
settings-net-longwave-shortwave-stations-labelled-waterfall = Las emisoras de onda larga y de onda corta etiquetadas en la cascada. El horario de onda corta se descarga de eibispace.de para la temporada de emisión en curso y se vuelve a descargar cuando cambia la temporada.
settings-net-season = Temporada
settings-net-downloading = descargando…
settings-net-using-built-copy = usando la copia integrada
settings-net-your-own-stations-corrections-go = Tus propias emisoras y correcciones van en este archivo. sdroxide nunca lo escribe, y una entrada de aquí sustituye a una del horario con el mismo nombre y frecuencia.
settings-net-reload = Recargar
settings-net-re-read-your-station-file = Volver a leer tu archivo de emisoras tras editarlo
settings-net-download-schedule-now = Descargar el horario ahora
settings-net-fetch-season-s-schedule-again = Obtener de nuevo el horario de esta temporada, sustituyendo la copia en caché
settings-net-broadcast-stations-labelled-waterfall-come = Las emisoras etiquetadas en la cascada proceden de la lista integrada en esta compilación. Editarlas requiere la aplicación de escritorio.
settings-net-ask-service-whether-these-credentials = Pregunta al servicio si estas credenciales funcionan. Aplica primero los ajustes de arriba y no registra nada: solo lee.
settings-net-checking = comprobando…

## Settings window: messages with values filled in.

settings-amp-connected-to = Conectado al { $model }
settings-controls-last-midi = Último mensaje: { $message }  valor { $value }
settings-general-band-plan-file = Cada límite de banda y subsegmento procede de { $path }, escrito a partir de las tablas IARU integradas la primera vez y editable por ti después — estrecha una banda a tu licencia y sdroxide se negará a transmitir fuera de ella. Las frecuencias están en MHz; borra el archivo para obtener una copia nueva de los valores predeterminados. Esta es la asignación regional, no tu licencia: tus propias condiciones pueden ser más estrechas, y los planes nacionales difieren dentro de una región.
settings-general-exported = Exportado { $note } — elige dónde guardarlo.
settings-general-export-failed = Error al exportar: { $error }
settings-general-import-failed = Error al importar: { $error }
settings-general-imported = { $summary } — reinicia sdroxide para usarlos.
settings-general-import-skipped = Omitido { $path }: { $why }
settings-general-swr-tripped = ⚠ Saltó en { $swr }:1 — la transmisión está bloqueada.
settings-net-call-grid-set-on-general = { $call } / { $grid } — fijado en la pestaña General
settings-net-reporting-as = Informando como { $call } / { $grid } — SDRoxide { $version }
settings-net-schedule-download-failed = ⚠ Error de descarga: { $error }. Usando el horario integrado.
settings-net-test-service = Probar { $service }

## Settings window (src/app/settings/mod.rs), continued

settings-transmit-off-while-converter-set = La transmisión está desactivada mientras hay un conversor configurado — indica arriba qué hay en la línea de transmisión para reactivarla.
settings-transmit-goes-through-same-converter = La transmisión pasa por el mismo conversor, así que la radio trabaja con el mismo desplazamiento respecto al dial en ambos sentidos.
settings-transmit-not-converted-radio-transmits = La transmisión no se convierte: la radio transmite en la frecuencia que muestra el dial, mientras que la recepción pasa por el conversor.
settings-transmit-takes-own-offset-separate = La transmisión toma su propio desplazamiento, aparte del de recepción.
settings-no-enabled-subscriptions-update = No hay suscripciones activadas que actualizar.
settings-add-radio-here-station-you = Añadir una radio — aquí, o en una estación a la que estés conectado

## Settings window, General, continued

settings-general-every-band-edge-sub-segment = Cada límite de banda y subsegmento procede de bandplan.json en la máquina a la que está conectada la radio. Esta es la asignación regional, no tu licencia: tus propias condiciones pueden ser más estrechas, y los planes nacionales difieren dentro de una región.

## Settings window, picture upload (src/app/settings/picup.rs)

settings-picup-received-pictures-ftp = Imágenes recibidas → FTP
settings-picup-uploads-every-received-sstv-picture = Sube cada imagen SSTV recibida (y, si está marcado, cada carta meteorológica) al guardarla, organizada como lo hace QSSTV para que las páginas web escritas para QSSTV funcionen sin cambios.
settings-picup-upload-received-pictures = Subir las imágenes recibidas
settings-picup-server = Servidor
settings-picup-port = Puerto
settings-picup-security = Seguridad
settings-picup-user = Usuario
settings-picup-password = Contraseña
settings-picup-certificate = Certificado
settings-picup-trusted-when-computer-s-operating = Se confía en él cuando el sistema operativo de este ordenador confía en él, como lo haría un navegador aquí. Se puede añadir una CA privada abajo (PEM).
settings-picup-extra-ca = CA adicional
settings-picup-only-certificate-fingerprint-accepted-way = Solo se acepta el certificado con esta huella — la forma de confiar en un servidor con certificado autofirmado. PROBAR muestra la del servidor.
settings-picup-any-certificate-accepted-link-encrypted = ⚠ Se acepta cualquier certificado. El enlace está cifrado, pero cualquiera en el trayecto puede hacerse pasar por el servidor y leer la contraseña.
settings-picup-plain-ftp-sends-password-pictures = ⚠ El FTP sin cifrar envía la contraseña y las imágenes sin cifrar.
settings-picup-sstv = SSTV
settings-picup-sstv-dir = Carpeta SSTV
settings-picup-weather-fax = Fax meteorológico
settings-picup-wefax-dir = Carpeta WEFAX
settings-picup-empty-login-directory = Vacío es el directorio de acceso.
settings-picup-file-names = Nombres de archivo
settings-picup-format = Formato
settings-picup-add-extension = añadir extensión
settings-picup-image1-png-rather-than-qsstv = image1.png en lugar de image1 a secas como QSSTV
settings-picup-caption-bar = Barra de leyenda
settings-picup-time-frequency-mode-sender-s = Hora, frecuencia, modo y el ID FSK del remitente en la parte superior, como lo dibuja QSSTV. Solo se marca la copia subida.
settings-picup-fit-960-768 = Ajustar a 960 × 768
settings-picup-qsstv-s-heartbeat-tiny-picture = El latido de QSSTV: una imagen diminuta cuya altura es el minuto, subida junto a cada una, y que se hace de dos píxeles de ancho al cerrar el programa.
settings-picup-nat-workaround = Apaño para NAT
settings-picup-ignore-private-address-server-s = Ignora una dirección privada en la respuesta en modo pasivo del servidor y usa la propia del servidor — para un servidor tras NAT que no conoce su dirección pública.
settings-picup-apply = {" "}APLICAR{" "}
settings-picup-test = PROBAR
settings-picup-connect-settings-above-applied-not = Conecta con los ajustes de arriba — aplicados o no —, inicia sesión y escribe y borra un archivo pequeño en cada directorio de subida.
settings-picup-checking = comprobando…
settings-picup-trust-certificate = Confiar en este certificado
settings-picup-pin-switch-pinned-fingerprint-one = Fíjalo: cambia a «Huella fijada» con esta. Compárala antes con la del servidor — esto es lo que dijo el servidor, no una prueba de quién es. Luego APLICAR.
settings-picup-last-upload = Última subida

## Settings window, Profiles (src/app/settings/profiles.rs)

settings-profiles-profiles-station-s-saved-setups = Perfiles: las configuraciones guardadas de la estación
settings-profiles-profile-captures-whole-working-setup = Un perfil captura toda la configuración de trabajo — los diales y VFO, el modo y los filtros, las ganancias, la excitación y las antenas, tu indicativo y plantillas de mensajes, y los registros por banda — y la aplica de nuevo con un clic. El hardware (motor, dispositivos de audio, conversores) se deja deliberadamente aparte: un perfil es la forma en que trabajas la estación, no la radio en la que se ejecuta.
settings-profiles-save-current-setup = Guardar la configuración actual
settings-profiles-profile-name = nombre del perfil
settings-profiles-save = Guardar
settings-profiles-applies-whatever-sdroxide-attached-dials = Se aplica a aquello a lo que esté conectado sdroxide: diales y VFO, modo y filtros, niveles, ganancias, excitación, antenas, la identidad digital y las pilas de banda.
settings-profiles-no-profiles-saved-yet-name = Aún no hay perfiles guardados — ponle nombre a uno arriba y pulsa Guardar.
settings-profiles-saved-profiles = Perfiles guardados
settings-profiles-apply = Aplicar

## Settings window, T/R switch (src/app/settings/relay.rs)

settings-relay-transmit-receive-switch = Conmutador de transmisión/recepción
settings-relay-waiting-station-s-t-r = Esperando la configuración del conmutador T/R de la estación…
settings-relay-closes-contact-while-station-transmits = Cierra un contacto mientras esta estación transmite — para desconectar y poner a masa la entrada de antena del SDR, y para activar un amplificador o un relé T/R externo en secuencia con ello. Funciona con las placas de relés USB baratas (LCUS, KMtronic, Numato), con una línea serie RTS/DTR hacia cualquier interfaz que quiera un cierre de PTT, y con cualquier otra cosa mediante un comando.
settings-relay-hardware = Hardware
settings-relay-board = Placa
settings-relay-lcus-four-byte-a0-command = LCUS es la placa de comando A0 de cuatro bytes que se vende con una docena de nombres; a KMtronic y Numato también se les puede preguntar cómo están configurados
settings-relay-port = Puerto
settings-relay-choose = — elegir —
settings-relay-device = Dispositivo
settings-relay-gpio-pin = Pin GPIO
settings-relay-pin-3-every-homebrew-plan = El pin 3 en todos los planos caseros y en todas las placas comerciales — es el del extremo del encapsulado al que se puede soldar un cable. Si el relé no hace clic, la tarjeta es un clon que no implementa estos pines en absoluto.
settings-relay-gpio-chip = Chip GPIO
settings-relay-linux-only-gpiodetect-lists-chips = Solo Linux. `gpiodetect` lista los chips de la máquina.
settings-relay-transmit = En transmisión
settings-relay-receive = En recepción
settings-relay-nothing-switched = No se conmuta nada.
settings-relay-starting-program-costs-unpredictable-amount = Lanzar un programa cuesta una cantidad de tiempo impredecible, y nada de ella se cuenta en la antelación — así que deja aquí un margen más amplio que con una placa de relés. Los números de la tabla de contactos no significan nada para este enlace: tus comandos se ejecutan una vez al pulsar y otra al soltar.
settings-relay-contacts = Contactos
settings-relay-contact-1-rts-contact-2 = El contacto 1 es RTS y el contacto 2 es DTR.
settings-relay-no = N.º
settings-relay-gpio = GPIO
settings-relay-name = Nombre
settings-relay-job = Función
settings-relay-tx-closes = TX cierra
settings-relay-lead = Antelación
settings-relay-hold = Retención
settings-relay-line-s-offset-chip-bcm = El desplazamiento de la línea en este chip — el número BCM en un conector de Raspberry Pi, que no es el número físico del pin
settings-relay-sdr-antenna = Antena del SDR
settings-relay-decides-default-timings-what-log = Decide los tiempos predeterminados y cómo lo llama el registro — y, para un decodificador de banda, que el contacto sigue la tabla de bandas de abajo
settings-relay-transmitting-energises-coil-off-transmitting =
    Activado: transmitir energiza la bobina. Desactivado: transmitir la libera.

    Esto es una decisión de cableado antes que un ajuste — mira la nota de abajo.
settings-relay-how-long-before-rf-contact = Cuánto tiempo antes de la RF pasa este contacto de la palabra RX de su banda a la palabra TX. Un cambio de sintonía lo mueve al instante; esto es para el pulsado, cuando un filtro solo de transmisión tiene que estar en el circuito antes que la potencia.
settings-relay-closed-long-before-rf-small = Cerrado este tiempo antes de la RF. Un relé coaxial pequeño conmuta en 5 a 15 ms. La transmisión espera al más largo de ellos, así que un valor grande es un hueco audible en el audio de recepción; todo lo que pase de 250 ms se ignora.
settings-relay-how-long-after-rf-stops = Cuánto tiempo después de que cese la RF vuelve este contacto de su palabra TX a su palabra RX.
settings-relay-opened-long-after-rf-stops = Abierto este tiempo después de que cese la RF. Más largo que la antelación a propósito: devolver la antena un momento tarde no cuesta nada, y devolverla pronto cuesta una etapa de entrada. Cero es una respuesta real, y la correcta para la línea de activación de un amplificador.
settings-relay-test = {" "}PROBAR{" "}
settings-relay-not-while-station-air = No mientras la estación está en el aire
settings-relay-close-contact-half-second-so = Cierra este contacto durante medio segundo, para que oigas el relé y compruebes el cableado con el transmisor en frío
settings-relay-remove = Quitar
settings-relay-add-contact = + añadir un contacto
settings-relay-if-will-not-answer = Si no responde
settings-relay-switch-exists-protect-receiver-not = Un conmutador que existe para proteger un receptor no vale la pena si deja de protegerlo en silencio — por eso negarse es lo predeterminado. Elige la otra opción donde un cable USB suelto que acabe con tu concurso sea el peor resultado.
settings-relay-order-timings-longest-lead-closes = El orden son los tiempos: la antelación más larga cierra primero, y la retención más larga abre la última. Así que da al relé de antena la antelación y la retención más largas, y al amplificador las más cortas — entonces la antena siempre se conmuta antes de que se active el amplificador, y el amplificador siempre se desactiva antes de que vuelva la antena.
settings-relay-transmit-sense-input = Entrada de detección de transmisión
settings-relay-when-transceiver-keys-itself-own = Cuando un transceptor se activa solo — su propio botón del micrófono, pedal, VOX o manipulador —, sdroxide se entera preguntando por CAT, y esa respuesta llega unos cientos de milisegundos después de empezar el turno. Cablea la línea SEND del equipo, a través de un optoaislador, a una entrada de handshake de este mismo puerto y se ve en pocos milisegundos. El medidor y el enclavamiento de transmisión se vuelven más rápidos con ello.
settings-relay-line = Línea
settings-relay-transmitting = Transmitir es
settings-relay-high-line = una línea alta
settings-relay-most-opto-isolated-interfaces-pull = La mayoría de las interfaces optoaisladas bajan la línea cuando el equipo se activa, así que esto suele estar desactivado
settings-relay-belongs-radio = Pertenece a la radio
settings-relay-which-radio-tab-sensed-transceiver = Qué pestaña de radio es el transceptor detectado. En una estación con una sola radio es 1 y nada depende de ello.
settings-relay-needs-serial-port-handshake-lines = Necesita un puerto serie — las líneas de handshake están en él.
settings-relay-wiring-only-fail-safe-survives = El cableado es la única protección que sobrevive a este programa.
settings-relay-choose-tx-closes-so-de = Elige «TX cierra» de modo que el contacto *desenergizado* sea el estado que quieres cuando la placa está desenchufada, el ordenador apagado o sdroxide no se está ejecutando. Cableado de forma que desenergizado signifique a masa, un relé muerto deja el SDR sordo — molesto, pero seguro. Cableado al revés, un relé muerto más una transmisión es una etapa de entrada muerta. Con un amplificador de verdad detrás, toma la primera.
settings-relay-cannot-protect-first-instant-over = Esto no puede proteger el primer instante de un turno que inicias en la radio. Activa el equipo desde sdroxide y los contactos siempre preceden a la RF; cablea la entrada de detección de arriba y la siguen en milisegundos. Para una etapa de entrada que de verdad merezca protección, usa además un conmutador T/R de hardware con detección de RF — ningún programa de un ordenador puede ser la respuesta completa.
settings-relay-contacts-transmit = contactos en TRANSMISIÓN
settings-relay-not-open = No abierto.
settings-relay-status-unknown-press-apply = Estado desconocido — pulsa APLICAR.
settings-relay-station-air-apply-will-take = La estación está en el aire — APLICAR surtirá efecto tras este turno.
settings-relay-band-decoder-outputs = Salidas del decodificador de banda
settings-relay-one-control-word-per-band = Una palabra de control por banda: el bit 0 es el contacto 1, el bit 1 el contacto 2, y así sucesivamente, según la numeración de arriba. RX sigue la banda del dial de recepción; TX la de la radio que haya activado, conmutada en la antelación de cada contacto antes de la RF y de vuelta tras su retención — dales el mismo valor para un filtro que no se preocupa del sentido de la RF, y distintos para un bypass de preamplificador solo de recepción o un banco de filtros paso bajo solo de transmisión. Las bandas a 0 no activan nada. Se aplica con Aplicar / reconectar.
settings-relay-band = Banda
settings-relay-rx = RX
settings-relay-tx = TX
settings-relay-outputs-asserted = Salidas activadas
settings-relay-other = Otras
settings-relay-everywhere-outside-amateur-bands-short = Todo lo que queda fuera de las bandas de aficionado — escucha de onda corta, y cualquier lugar donde caiga el dial de un transversor que ninguna banda cubra.
settings-relay-apply = {" "}APLICAR{" "}
settings-relay-persist-re-open-switch = Guardar y (re)abrir el conmutador

## Settings window, Remote (src/app/settings/remote.rs)

settings-remote-connect-server = Conectar a un servidor
settings-remote-drive-sdroxide-running-somewhere-else = Maneja por la red un sdroxide que se ejecuta en otro sitio — la máquina de la estación, un emplazamiento remoto. Ponlo en marcha allí con --server e introduce aquí su dirección. La radio, los decodificadores y el diario se quedan en esa máquina; esta pantalla recibe la cascada y el audio, y sus altavoces y micrófono son los que usas.
settings-remote-address = Dirección
settings-remote-host-name-ip = nombre de host o IP
settings-remote-machine-running-sdroxide-server-host = La máquina que ejecuta sdroxide --server: un nombre de host, una dirección IPv4 o IPv6, o una URL ws:// (o wss://) completa si el servidor está tras un proxy inverso — una URL se usa exactamente como se escribe e ignora el puerto y el interruptor de abajo.
settings-remote-port = Puerto
settings-remote-port-server-listens-server-port = El puerto en el que escucha ese servidor — server_port en su config.toml. 4950 salvo que se le diera --port.
settings-remote-secure = Seguro
settings-remote-wss-tls = wss:// (TLS)
settings-remote-tick-where-server-sits-behind = Marca esto cuando el servidor esté tras algo que termina HTTPS por él — un proxy inverso en el puerto 443, que es la forma habitual de llegar a una estación a través de internet. El servidor propio de sdroxide habla ws:// sin cifrar y no quiere esto.
settings-remote-connect = {" "}CONECTAR{" "}
settings-remote-open-station-radio-tab-own = Abrir esta estación como una pestaña de radio propia
settings-remote-client-cannot-hold-second-connection = Este cliente no puede mantener una segunda conexión.
settings-remote-station-appears-tab-beside-radios =
    La estación aparece como una pestaña junto a las radios ya abiertas aquí, y la tira de pestañas de la parte superior de la ventana conmuta entre ellas (⊞ pone dos una al lado de otra). Ciérrala desde la lista de la parte superior de la pestaña Radio — eso cuelga; no se cambia nada en el servidor.

    Si el servidor pide usuario y contraseña, su pantalla de inicio de sesión aparece en la nueva pestaña. Un enlace ws:// sin cifrar no lleva nada de esto cifrado, así que a través de internet hay que pasarlo por una VPN o un túnel SSH, o terminar HTTPS delante del servidor y marcar Seguro arriba.

## Settings window, Servers (src/app/settings/servers.rs)

settings-servers-wsjt-x-udp-broadcast = Difusión UDP de WSJT-X
settings-servers-waiting-station-s-wsjt-x = Esperando la configuración de difusión WSJT-X de la estación…
settings-servers-sends-decodes-station-status-logged = Envía decodificaciones, estado de la estación y QSO registrados como lo hace WSJT-X, de modo que GridTracker, JTAlert, N1MM+ y Log4OM funcionan con sdroxide sin cambios. Solo salida — nada en este socket puede tocar la radio.
settings-servers-enable = Activar
settings-servers-send = Enviar a
settings-servers-t-127-0-0-1-reaches = 127.0.0.1 llega a los clientes de esta máquina; una dirección de red local o un grupo multicast (224.0.0.1) llega a otros
settings-servers-port = Puerto
settings-servers-t-2237-port-every-client-defaults = 2237 es el puerto que todo cliente usa por omisión
settings-servers-identify = Identificarse como
settings-servers-name-clients-see-some-loggers = El nombre que ven los clientes. Algunos programas de registro solo aceptan tráfico que se identifique como WSJT-X.
settings-servers-n1mm-contactinfo-broadcast = Difusión contactinfo de N1MM+
settings-servers-same-news-n1mm-s-words = La misma noticia con las palabras de N1MM: un datagrama XML por contacto registrado, para los programas de registro que escuchan N1MM en lugar de WSJT-X — el puente de la World Radio League acepta cualquiera. Solo contactos; este protocolo no tiene flujo de decodificaciones. Con su propio interruptor y su propio puerto, porque una estación puede querer ambos.
settings-servers-t-127-0-0-1-reaches-2 = 127.0.0.1 llega a los clientes de esta máquina. El consejo de N1MM para una red de concurso es la dirección de difusión de esta subred — 192.168.1.255 para una red 192.168.1.n — que llega a todos los puestos a la vez.
settings-servers-t-12060-port-n1mm-s-documentation = 12060 es el puerto que recomienda la documentación de N1MM
settings-servers-station-name = Nombre de la estación
settings-servers-what-n1mm-calls-stationname-name = Lo que N1MM llama StationName: el nombre del ordenador que envió el paquete. Los programas de registro lo muestran para distinguir un puesto de operación de otro.
settings-servers-decoded-cw-over-udp = CW decodificado por UDP
settings-servers-cw-panel-s-copy-plain = La copia del panel de CW como texto plano, un datagrama UDP por fragmento a medida que se asienta — para un controlador de estación, la ventana de CW de un programa de registro o un script. Sin protocolo: un oyente simplemente añade lo que llega (prueba nc -ul 9999). Solo se envía el texto asentado, nunca la cola que el decodificador aún está revisando. Cada radio tiene su propio interruptor.
settings-servers-t-127-0-0-1-reaches-3 = 127.0.0.1 llega a los programas de esta máquina; una dirección de red local o la dirección de difusión de esta subred llega a otros.
settings-servers-t-9999-default = 9999 por omisión
settings-servers-apply = {" "}APLICAR{" "}
settings-servers-persist-re-open-broadcast-sockets = Guardar y (re)abrir los sockets de difusión
settings-servers-hamlib-rigctld-server = Servidor rigctld de Hamlib
settings-servers-waiting-station-s-rigctld-configuration = Esperando la configuración de rigctld de la estación…
settings-servers-listen = Escuchar en
settings-servers-t-127-0-0-1-machine = 127.0.0.1 (esta máquina)
settings-servers-t-0-0-0-0-whole = 0.0.0.0 (toda la red)
settings-servers-t-4532-port-every-rigctld-client = 4532 es el puerto que supone todo cliente rigctld
settings-servers-rig-name = Nombre del equipo
settings-servers-reported-clients = informado a los clientes
settings-servers-max-clients = Máx. de clientes
settings-servers-allow-clients-transmit = Permitir que los clientes transmitan
settings-servers-off-refuses-every-key-request = Desactivado rechaza toda petición de activación y deja de anunciar un rango de transmisión, de modo que el propio Hamlib se niega a activar.
settings-servers-rigctld-has-no-authentication-0 = ⚠ rigctld no tiene autenticación. En 0.0.0.0, cualquiera que alcance este puerto puede sintonizar y activar tu transmisor.
settings-servers-t-1-client = 1 cliente
settings-servers-not-running = No en ejecución.
settings-servers-status-unknown-press-apply = Estado desconocido — pulsa APLICAR.
settings-servers-persist-re-bind-server = Guardar y (re)enlazar el servidor
settings-servers-lets-any-hamlib-capable-program = Permite que cualquier programa compatible con Hamlib maneje esta radio: frecuencia, modo, PTT, split, RIT y potencia. En WSJT-X, fldigi o CQRLOG elige el equipo «Hamlib NET rigctl» (modelo 2) y apúntalo a esta dirección; en GPredict y N1MM introduce directamente el host y el puerto. A diferencia del servidor TCI, solo lleva control — sin audio ni IQ.
settings-servers-antenna-rotator = Rotor de antena
settings-servers-waiting-station-s-rotator-configuration = Esperando la configuración del rotor de la estación…
settings-servers-points-motorized-antenna-satellite-you = Apunta una antena motorizada al satélite en el que estás enganchado, mediante un demonio rotctld de Hamlib (ejecuta p. ej. «rotctld -m 603 -r /dev/ttyUSB0» para un rotor GS-232B, o «rotctld -m 1» para probar sin hardware) — un solo demonio cubre todos los rotores que maneja Hamlib — o mediante PSTRotator, con el control UDP activado en su Setup.
settings-servers-program = Programa
settings-servers-host = Servidor
settings-servers-t-4533-rotctld-s-default-pstrotator = 4533 es el valor predeterminado de rotctld. El puerto de control UDP de PSTRotator es 12000 por omisión, y responde a las consultas de posición en el siguiente, 12001 — que tiene que estar libre en esta máquina.
settings-servers-min-elevation = Elevación mín.
settings-servers-below-rotator-parks-instead-tracking = Por debajo de esto el rotor se aparca en lugar de seguir — fíjalo a tu horizonte local o a la línea del tejado
settings-servers-azimuth-offset = Desplazamiento de acimut
settings-servers-added-every-command-rotator-whose = Se suma a cada comando, para un rotor cuyo norte está desviado
settings-servers-min-movement = Movimiento mín.
settings-servers-steps-smaller-than-are-not = No se envían pasos menores que esto — perseguir décimas de grado desgasta los motores para nada
settings-servers-park-position = Posición de aparcamiento
settings-servers-drive-position-when-idle = ir a una posición cuando está inactivo
settings-servers-off-leaves-antenna-wherever-last = Desactivado deja la antena donde terminó el último paso
settings-servers-not-connected = No conectado.
settings-servers-persist-re-connect-daemon = Guardar y (re)conectar con el demonio
settings-servers-waiting-station-s-tci-server = Esperando la configuración del servidor TCI de la estación…
settings-servers-built-tci-server = Servidor TCI integrado
settings-servers-device-name = Nombre del dispositivo
settings-servers-off-leaves-control-receive-streams = Desactivado deja funcionando el control y los flujos de recepción, pero se rechaza toda petición de activación.
settings-servers-tci-has-no-authentication-0 = ⚠ TCI no tiene autenticación. En 0.0.0.0, cualquiera que alcance este puerto puede sintonizar y activar tu transmisor.
settings-servers-tci-client = Cliente TCI
settings-servers-when-you-tune-send-radio = Al sintonizar, envía el VFO de esta radio a otro servidor TCI para que ese equipo sintonice a la vez. Solo control — sin audio ni IQ, y no activa el transmisor.
settings-servers-server = Servidor
settings-servers-same-address-server-above-client = Esta es la misma dirección que la del servidor de arriba. El cliente solo se conecta cuando ese servidor no es el que está escuchando — déjalo desactivado, o en otro puerto, cuando 590Ctrl use este puerto.
settings-servers-tci-client-connected = ● Cliente TCI conectado
settings-servers-tci-client-connecting = Cliente TCI conectando…
settings-servers-tci-client-press-apply = Cliente TCI — pulsa APLICAR.
settings-servers-persist-re-bind-server-connect = Guardar, (re)enlazar el servidor y conectar el cliente TCI
settings-servers-server-lets-tci-capable-programs = El servidor permite que los programas compatibles con TCI manejen esta radio: control de frecuencia y modo, IQ de banda ancha, audio de recepción y audio de transmisión. En WSJT-X elige el tipo de equipo SunSDR (TCI), apúntalo a esta dirección y fija el PTT a TCI. El Cliente TCI es la otra dirección: el dial de sdroxide se envía como comando VFO a 590Ctrl (o a cualquier servidor TCI) en la dirección de arriba. No pasa audio.

## Settings window, TLE (src/app/settings/tle.rs)

settings-tle-satellites-element-sets-frequencies = Satélites: elementos orbitales y frecuencias
settings-tle-waiting-station-s-satellite-configuration = Esperando la configuración de satélites de la estación…
settings-tle-tracker-already-fetches-celestrak-s = El seguimiento ya obtiene por sí solo el grupo de aficionados de CelesTrak. Esto es para todo lo demás: los pájaros meteorológicos de la NOAA, un cubesat demasiado nuevo para estar en el grupo, o un conjunto de elementos más reciente que el que llegó. Los listados se obtienen — y se conservan — donde se ejecuta el motor de radio, así que es el mismo conjunto de satélites dondequiera que se abra la aplicación.
settings-tle-subscriptions = Suscripciones
settings-tle-listings-fetched-kept-current-same = Listados obtenidos y mantenidos al día, con la misma cadencia de seis horas que el conjunto de aficionados. Se actualizan mientras la ventana solar está abierta y con ACTUALIZAR AHORA aquí.
settings-tle-fetch-track-listing = Obtener y seguir este listado
settings-tle-remove-subscription = Quitar esta suscripción
settings-tle-orbits = Órbitas
settings-tle-which-satellites-listing-get-orbit = Qué satélites de este listado reciben un anillo de órbita y una etiqueta. Un grupo entero pide «curated»: noventa anillos a la vez son ilegibles, y ninguno deja noventa puntos anónimos.
settings-tle-nothing-listing-sdroxide-s-curated = Nada de este listado está en la lista curada de sdroxide — esa lista son diez satélites de aficionados, así que esto se comportaría exactamente como «ninguno».
settings-tle-all-satellites = todos los satélites
settings-tle-catalogue-numbers-keep-comma-separated = Números de catálogo que conservar, separados por comas. Vacío sigue todo lo que lleva el listado.
settings-tle-not-fetched-yet = aún no obtenido
settings-tle-subscription = + Suscripción
settings-tle-update-now = {" "}ACTUALIZAR AHORA{" "}
settings-tle-ask-radio-engine-fetch-every = Pide al motor de radio que obtenga ahora todas las suscripciones activadas
settings-tle-celestrak-groups = Grupos de CelesTrak
settings-tle-pasted-element-sets = Conjuntos de elementos pegados
settings-tle-one-off-these-do-not = Para algo puntual. Estos no se actualizan solos, y SGP4 deja de propagar un conjunto de elementos una vez pasadas dos semanas de su época — suscríbete en su lugar para todo lo que quieras conservar.
settings-tle-track-one = Seguir este
settings-tle-show-two-element-lines = Mostrar las dos líneas de elementos
settings-tle-remove = Quitar
settings-tle-paste-two-three-line-element = Pega aquí conjuntos de elementos de dos o tres líneas
settings-tle-add-pasted = + Añadir lo pegado
settings-tle-nothing-paste-box-looked-like = Nada en el cuadro de pegado parecía un conjunto de elementos.
settings-tle-clear-box = Vaciar cuadro
settings-tle-frequencies = Frecuencias
settings-tle-shown-under-pass-table-solar = Se muestran bajo la tabla de pasos en la ventana solar. Una entrada aquí sustituye por completo a la integrada para ese número de catálogo, así que parte de una copia de ella salvo que quieras descartar el resto.
settings-tle-remove-satellite-s-entry = Quitar la entrada de este satélite
settings-tle-link = ENLACE
settings-tle-downlink = ENLACE DESCENDENTE
settings-tle-uplink = ENLACE ASCENDENTE
settings-tle-mode = MODO
settings-tle-note = NOTA
settings-tle-fm-repeater = Repetidor FM
settings-tle-fm = FM
settings-tle-ctcss-67-0-hz = CTCSS 67,0 Hz
settings-tle-link-2 = + Enlace
settings-tle-copy-built = Copiar el integrado
settings-tle-frequency-either-one-number-145 = Una frecuencia es o bien un número (145.800) o bien la banda pasante de un transpondedor escrita 145.950-145.970. Deja una dirección en blanco para una baliza.
settings-tle-norad = NORAD
settings-tle-satellite = + Satélite
settings-tle-frequency-entry-needs-satellite-s = Una entrada de frecuencia necesita el número de catálogo NORAD del satélite.

## Settings window, Wavelog (src/app/settings/wavelog.rs)

settings-wavelog-address = Dirección
settings-wavelog-api-key = Clave de API
settings-wavelog-your-wavelog-s-front-page = La dirección de la página principal de tu Wavelog — https://log.example.org o http://192.168.1.10/wavelog — y una clave de su página de API. Subir requiere una clave de lectura/escritura.
settings-wavelog-plain-http-address-internet-api = http:// sin cifrar hacia una dirección de internet: la clave de API viaja sin cifrar. Bien dentro de tu propia red; usa https:// a través de internet.
settings-wavelog-trusted-cert = Cert. de confianza
settings-wavelog-forget-certificate = Olvidar este certificado
settings-wavelog-go-back-ordinary-checking-against = Volver a la comprobación normal contra las autoridades de certificación públicas — para un servidor que ya tiene un certificado en regla.
settings-wavelog-connection-check-runs-machine-radio = La comprobación de conexión se ejecuta en la máquina a la que está conectada la radio — usa los ajustes de allí para probarla y listar las ubicaciones de estación.
settings-wavelog-server-presented-certificate-sha-256 = El servidor presentó este certificado (SHA-256):
settings-wavelog-compare-your-server-s-e = Compáralo con el de tu servidor — p. ej. `openssl x509 -noout -fingerprint -sha256 -in cert.pem` — y confía en él solo si coinciden. A partir de entonces se acepta exactamente este certificado y se rechaza cualquier otro.
settings-wavelog-trust-certificate = Confiar en este certificado
settings-wavelog-station-location = Ubicación de la estación
settings-wavelog-none = — ninguna —
settings-wavelog-press-test-above-list-station = Pulsa Probar arriba para listar las ubicaciones de estación definidas en tu Wavelog.
settings-wavelog-default = Predeterminada
settings-wavelog-where-qsos-go-when-operating = Adónde van los QSO cuando el perfil de operación no nombra una ubicación.
settings-wavelog-profile = Este perfil
settings-wavelog-use-default = Usar la predeterminada
settings-wavelog-not-set = sin fijar
settings-wavelog-no-station-location-chosen-uploads = No hay ubicación de estación elegida — las subidas fallarán hasta que haya una.
settings-wavelog-qsos-not-wavelog = QSO que no están en Wavelog
settings-wavelog-every-qso-logbook-wavelog = Todos los QSO del diario están en Wavelog.
settings-wavelog-everything-logbook-wavelog-has-not = Todo lo del diario que Wavelog no ha confirmado — incluidos los contactos anteriores a la subida automática. Los que ya tiene vuelven como duplicados y simplemente se marcan como enviados.
settings-wavelog-set-address-key-station-location = Fija primero la dirección, la clave y una ubicación de estación.
settings-wavelog-yes-send-them = Sí, enviarlos
settings-wavelog-cancel = Cancelar
settings-wavelog-download-into-logbook = Descargar a este diario
settings-wavelog-downloading-runs-where-radio-attached = La descarga se ejecuta donde está conectada la radio, hacia el diario de esa máquina — configúralo allí.
settings-wavelog-download-qsos-start-every = Descargar QSO al iniciar y cada
settings-wavelog-downloaded-qsos-join-logbook-so = Los QSO descargados se unen al diario, de modo que FT8 y las listas de spots saben lo que has trabajado desde otros programas y equipos. Cada descarga trae solo lo nuevo.
settings-wavelog-press-test-above-choose-which = Pulsa Probar arriba para elegir qué ubicaciones de estación descargar.
settings-wavelog-sync-now = Sincronizar ahora
settings-wavelog-fetch-what-wavelog-has-added = Obtiene lo que Wavelog ha añadido desde la última descarga. Aplica primero los ajustes de arriba.
settings-wavelog-full-resync = Resincronización completa…
settings-wavelog-download-ticked-locations-whole-compare = Descarga enteras las ubicaciones marcadas y las compara con el registro: los QSO editados o borrados en Wavelog, o de una ubicación que ya no está marcada, se ponen al día. Ves los cambios antes de que pase nada.
settings-wavelog-full-resync-changes-make = Resincronización completa — cambios por hacer
settings-wavelog-qsos-logged-here-imported-file = Los QSO registrados aquí o importados de un archivo nunca se cambian ni se quitan — solo los que se descargaron de Wavelog.
settings-wavelog-apply = Aplicar
settings-wavelog-discard = Descartar
settings-wavelog-live-radio-status = Estado de la radio en vivo
settings-wavelog-send-frequency-mode-wavelog-they = Enviar frecuencia y modo a Wavelog a medida que cambian
settings-wavelog-radio-name = Nombre de la radio
settings-wavelog-pick-radio-wavelog-s-live = Elige esta radio en el registro en vivo de Wavelog y su formulario sigue al dial. Requiere una clave de lectura/escritura. Solo desde la radio principal de la estación.

## Settings window, picture upload: messages with values

settings-picup-server-fingerprint = SHA-256 del certificado del servidor: { $fingerprint }

## Settings window, Profiles: messages with values

settings-profiles-apply-hint = Devuelve la estación a «{ $name }»: diales, VFO, modo, filtros, ganancias, excitación, antenas, identidad y pilas de banda.
settings-profiles-delete = Eliminar «{ $name }»

## Settings window, T/R switch: messages with values

settings-relay-not-usable = Aún no utilizable: { $why }
settings-relay-receiving = recibiendo
settings-relay-rx-tx-words = { $rx } / TX { $tx }

## Settings window, Servers: messages with values

settings-servers-n-clients = { $n ->
        [one] { $n } cliente
       *[other] { $n } clientes
    }
settings-servers-listening = ● Escuchando en { $addr } — { $clients }
settings-servers-failed = Error: { $error }
settings-servers-rotator-at = ● Conectado — antena a { $az }° de acimut, { $el }° de elevación
settings-servers-tci-client-error = Cliente TCI: { $error }

## Settings window, TLE: messages with values

settings-tle-subscription-age = { $count } satélites · { $age } de antigüedad
settings-tle-subscribed = Suscrito a { $name }. Pulsa ACTUALIZAR AHORA para obtenerlo.
settings-tle-norad-too-stale = NORAD { $norad } · { $age } de antigüedad — demasiado viejo para propagar
settings-tle-norad-age = NORAD { $norad } · { $age } de antigüedad
settings-tle-norad-n = NORAD { $norad }
settings-tle-added = { $n ->
        [one] Añadido { $n } conjunto de elementos
       *[other] Añadidos { $n } conjuntos de elementos
    }.
settings-tle-added-refreshed = Añadidos { $added } y actualizados { $r } { $r ->
        [one] conjunto de elementos
       *[other] conjuntos de elementos
    }.
settings-tle-links = { $n } { $n ->
        [one] enlace
       *[other] enlaces
    }
settings-tle-replace-links = Sustituir estos enlaces por los integrados de { $name }

## Settings window, Wavelog: messages with values

settings-wavelog-call-mismatch = La ubicación «{ $location }» es { $station_call }, pero registras como { $call } — Wavelog rechazará estos QSO.
settings-wavelog-grid-mismatch = La ubicación «{ $location }» está en { $station_grid }, pero tu locátor es { $grid } — Wavelog rechazará estos QSO.
settings-wavelog-location-n = ubicación n.º { $id }
settings-wavelog-saved-with-callsign = Guardado con el indicativo ({ $call }) en Ajustes → Perfiles, de modo que un perfil portátil pueda registrar en una ubicación portátil.
settings-wavelog-owed = { $n } { $n ->
        [one] QSO registrado
       *[other] QSO registrados
    } desde que se activó la subida automática aún por enviar — se reintenta cada 5 minutos.
settings-wavelog-send-unsent = Enviar { $n } { $n ->
        [one] QSO
       *[other] QSO
    } que aún no están en Wavelog…
settings-wavelog-send-confirm = ¿Enviar { $n } QSO a { $location }? Los ajustes se aplican primero.
settings-wavelog-plan-add = • { $n } QSO de Wavelog por añadir al registro
settings-wavelog-plan-update = • { $n } QSO descargados modificados en Wavelog por actualizar
settings-wavelog-plan-remove = • { $n } QSO descargados que ya no están en Wavelog (o de una ubicación sin marcar) por quitar
settings-wavelog-plan-mark = • { $n } de tus propios QSO por marcar como presentes en Wavelog

## Settings window, Radio tab (src/app/settings/radio.rs)

settings-radio-antenna = Antena
settings-radio-which-socket-back-radio-using =
    Qué conector de la parte trasera usa la radio — su propio comando ANT, el mismo ajuste que el botón ANT del panel frontal.

    Se aplica al instante, y es un ajuste de la radio y no una copia guardada aquí: el conector en el que está la radio se lee de vuelta al abrirse la sesión.

    La elección se recuerda por banda. Cambia al haz en 2 m y al hilo en 40, y cada uno vuelve la próxima vez que el dial entre en esa banda — y la próxima vez que se inicie sdroxide. Un canal de memoria guardado aquí lleva también su conector.
settings-radio-receive-antenna = Antena de recepción
settings-radio-radio-s-separate-receiving-antenna =
    La antena de recepción independiente de la radio — RX ANT en la parte trasera — conmutada dentro o fuera de la ruta de recepción. Es un ajuste propio, el mismo que el botón RX ANT de la radio.

    La antena del conector principal sigue en transmisión en ambos casos: es una entrada adicional para recibir, un lazo o una beverage, no una elección de conector.

    A diferencia de la antena de arriba, esta *no* se recuerda aquí. La radio la guarda por banda ella misma, así que sdroxide la lee de vuelta tras cada cambio de banda y muestra lo que dice la radio; marcar esta casilla es lo único que la mueve.
settings-radio-receive-path = En la ruta de recepción
settings-radio-radio-power = Alimentación de la radio
settings-radio-switch-radio-itself-off-back =
    Apaga la propia radio y la vuelve a encender, por el enlace de control. Este es el único encendido/apagado de verdad del programa — no el interruptor ENLACE de sdroxide, que cierra el extremo de sdroxide y deja la radio funcionando.

    Para que el encendido llegue a algo, el extremo de control de la radio tiene que seguir despierto mientras está apagada. Por red, eso es lo que hace Network Control. Por cable serie es el puerto CI-V de la radio, que sigue alimentado desde la red eléctrica en un equipo apagado desde el frontal en lugar de desenchufado; sdroxide envía la secuencia de despertar que pide la propia documentación de Icom antes del encendido.

    Apagar termina el audio y los medidores — no hay radio detrás de ellos — y sdroxide mantiene abierto el enlace de control para que el encendido tenga adónde llegar.
settings-radio-on = Encendida
settings-radio-off = Apagada
settings-radio-waiting-configuration-machine-radio-attached = Esperando la configuración de la máquina a la que está conectada la radio.
settings-radio-sound-format = Formato de sonido
settings-radio-invert-spectrum = Invertir el espectro
settings-radio-swap-i-q = Intercambiar I/Q
settings-radio-mirror-panadapter-about-tuned-frequency =
    Refleja el panadaptador respecto a la frecuencia sintonizada, para un equipo que lleva I y Q al revés en su tarjeta de sonido. La señal delatora es una cascada llena de señales convincentes que están todas del lado equivocado del dial, con la BLU saliendo en la banda lateral opuesta — intercambiar los dos cables en la tarjeta de sonido lo arreglaría igual de bien.

    Solo recepción: la transmisión entrega a la radio una sola señal de audio real, que no tiene banda lateral que invertir.
settings-radio-i-q-sample-rate = Frecuencia de muestreo I/Q
settings-radio-how-fast-radio-s-i =
    A qué velocidad funciona la tarjeta de sonido I/Q de la radio — y, por tanto, cuánta banda muestra el panadaptador, porque un flujo en cuadratura abarca toda su frecuencia de muestreo: 48 kHz da ±24 kHz a cada lado del dial, 192 kHz da ±96.

    La tarjeta tiene la última palabra. Una que no puede con la frecuencia elegida aquí se abre con la más cercana que admita, y el panadaptador es otro tanto más estrecho — el registro lo dice al arrancar cuando ocurre.

    Una tarjeta más rápida también es más trabajo: la máquina tiene proporcionalmente menos tiempo para vaciarla, y una que no puede seguir el ritmo pierde muestras. Eso se nota como cortes en el audio mientras la cascada sigue pareciendo perfecta, y también queda registrado. Si lo ves, baja un escalón.
settings-radio-t-48-khz-24-khz = 48 kHz (±24 kHz)
settings-radio-t-96-khz-48-khz = 96 kHz (±48 kHz)
settings-radio-t-192-khz-96-khz = 192 kHz (±96 kHz)
settings-radio-t-384-khz-192-khz = 384 kHz (±192 kHz)
settings-radio-i-q-centre-offset = Desplazamiento del centro I/Q
settings-radio-how-far-radio-s-i =
    Cuánto por encima de su propio dial está centrada la salida I/Q de la radio, para un equipo cuya FI de recepción se ha sacado de cero — en un Elecraft, MENU:RX SHFT puesto en 8.0 en lugar de NOR, lo que saca el dial del pico de continua del mezclador (y evita que una emisora de BLU/AM fuerte y cercana se detecte en AM).

    No es un conversor: la radio sigue mostrando y transmitiendo en la frecuencia real, y aquí no se le envía nada. Esto solo indica dónde caen las muestras en la tarjeta de sonido, y el flujo se vuelve a desplazar sobre el dial a medida que llega.

    Déjalo en 0 salvo que hayas activado una entrada de menú así. Si las señales caen al doble del desplazamiento, el signo es el contrario.
settings-radio-iq-correction = Corrección IQ
settings-radio-cancel-mirror-image-every-signal =
    Cancela la imagen especular de cada señal y el pico de continua en el centro de la cascada.

    La salida I/Q de una radio son dos caminos analógicos que nunca son del todo iguales en ganancia ni están exactamente a 90°, y lo que eso deja es una copia de cada señal reflejada respecto a la frecuencia sintonizada, normalmente 30-40 dB por debajo — lo bastante fuerte para parecer una estación que no existe y decodificarse como tal. Las radios con ajustadores de balance en el panel frontal ajustan exactamente esto; aquí se mide a partir del ruido recibido y no necesita ajuste.

    Déjalo activado. Desactívalo si escuchas AM sintonizado justo en la portadora — la portadora es continua, así que se va con el pico — o para comprobar si una señal es real viendo si sobrevive.
settings-radio-cancel-mirror-images = Cancelar imágenes especulares
settings-radio-dc-notch = Notch de continua
settings-radio-widen-hole-taken-out-middle =
    Ensancha el hueco que se quita del centro del ancho, para una radio cuyo pico central es más ancho que el desplazamiento que tiene debajo.

    0 deja el bloqueador normal, de unas pocas decenas de hercios de ancho, que quita el desplazamiento sin tocar nada más. Súbelo y el extremo inferior del ancho se va con él: un paso alto de primer orden, 3 dB abajo en la cifra fijada aquí y más por debajo.

    Está centrado donde está centrado el I/Q de la radio, que es el dial salvo que el desplazamiento de arriba lo haya movido — así que todo lo sintonizado ahí también se va. Una nota de CW a 600 Hz queda dentro de un ajuste de 600 Hz.
settings-radio-panadapter-bw = AB del panadaptador
settings-radio-cat-family = Familia CAT
settings-radio-rigctld-address = Dirección de rigctld
settings-radio-host-port-running-hamlib-rigctld =
    host:puerto de un rigctld de Hamlib en ejecución — 127.0.0.1:4532 es su valor propio por omisión, en esta máquina.

    Ponlo en marcha con, por ejemplo, `rigctld -m 2028 -r /dev/ttyUSB0 -s 38400`, donde -m es el número de modelo de Hamlib de tu radio (`rigctl -l` los lista).

    Es el comodín: llega a la frecuencia, el modo, el PTT, la potencia, el S-metro y la ROE y a nada más. Cuando una de las familias nativas de arriba encaja con tu radio hace más — activación desde el búfer de texto del propio equipo, el filtro de recepción y escalas de medidor por modelo.
settings-radio-flrig-address = Dirección de flrig
settings-radio-host-port-running-flrig-127 =
    host:puerto de un flrig en ejecución — 127.0.0.1:12345 es su valor propio por omisión, en esta máquina. flrig sirve XML-RPC siempre que se ejecuta; el puerto está en su Config → Setup → Server.

    Como la opción de Hamlib, maneja un demonio en lugar de la radio, pero a través del controlador por modelo del propio flrig — en varios equipos su manejo de potencia y filtro es el más fiel de los dos. Llega a la frecuencia, el modo, el PTT, la potencia de transmisión (en vatios enteros), el ancho de banda de recepción, el S-metro, la ROE y la potencia de salida. El CW se activa a través del puerto cwio del propio flrig, que debe configurarse en el propio flrig.
settings-radio-serial-port = Puerto serie
settings-radio-select = — seleccionar —
settings-radio-baud = Baudios
settings-radio-data-bits = Bits de datos
settings-radio-parity = Paridad
settings-radio-stop-bits = Bits de parada
settings-radio-force-rts = Forzar RTS
settings-radio-force-dtr = Forzar DTR
settings-radio-ptt-method = Método de PTT
settings-radio-how-transmit-keyed = Cómo se activa la transmisión.
settings-radio-how-transmit-keyed-network-link = Cómo se activa la transmisión. Un enlace de red no tiene líneas de control, así que DTR y RTS no activan nada aquí — usa CAT, que pide al demonio que active la radio, o VOX.
settings-radio-mode-control = Control de modo
settings-radio-digimode-mode = Modo para digimodos
settings-radio-what-put-radio-mode-sdroxide =
    En qué poner la radio para un modo que sdroxide modula a través de su tarjeta de sonido. «DIGI» selecciona la posición DATA/PKT del equipo, que toma el audio de transmisión de la entrada USB o ACC sin el procesado de voz de la ruta del micrófono; «USB» lo deja en la banda lateral normal, para una radio ya configurada para modular BLU desde su entrada de datos o una sin posición DATA.

    Cubre todos los modos digitales — FT8, FT4, PSK, RTTY, SSTV y el resto — y el CW enviado como «Tarjeta de sonido», y prevalece sobre el Control de modo para ellos. SSTV es el único cuya banda lateral sigue a la banda, así que ahí significa la posición DATA en esa banda lateral: USB-D por encima de 40 m, LSB-D en 40 m y por debajo. SSTV-FM no forma parte de ello — una portadora FM no tiene banda lateral que elegir.
settings-radio-cw-keying = Manipulación de CW
settings-radio-how-cw-panel-s-keyer =
    Cómo transmite el manipulador del panel de CW. «Manipulador del equipo» pone la radio en CW y le entrega el texto para enviar con su propio manipulador. Usa la velocidad del manipulador del equipo (fijada desde las WPM del panel) y en Yaesu envía por medio de la memoria 1 del manipulador, sobrescribiendo lo que hubiera guardado en ella.

    Necesita break-in activado en la radio, que sdroxide activa por sí mismo con cada mensaje en Icom, Yaesu y Kenwood — un equipo con el break-in desactivado acepta el mensaje y no transmite nada. Se deja activado después, y un break-in completo nunca se baja a semi.

    «Tarjeta de sonido» envía en cambio el tono lateral manipulado como audio (MCW), un tono en dial + tono — y como un equipo en CW ignoraría por completo su tarjeta de sonido, seleccionar CW sigue entonces el ajuste Modo para digimodos (USB, DATA o Controlado por radio) en lugar de poner el equipo en CW. Elige ahí «Controlado por radio» para equipos cuya posición de datos no puede ordenarse por CAT (la U-D de un Xiegu) y aparca tú mismo el equipo en ella, como para FT8.
settings-radio-poll-rate = Frecuencia de sondeo
settings-radio-how-often-radio-asked-what =
    Con qué frecuencia se pregunta a la radio qué está haciendo — su dial, su modo y sus medidores. Es la mitad del enlace de control que va de la radio a sdroxide: gira el mando VFO del propio equipo o cambia su modo en el panel frontal y la lectura, la banda y el panadaptador siguen en un sondeo.

    También es todo el tráfico de control que genera este extremo, y en un Icom moderno ese tráfico no es gratis: el puerto CI-V y la tarjeta de sonido están tras un mismo concentrador USB dentro de la radio, así que las tramas pedidas aquí son tiempo de bus que el audio no recibe — se oyen como cortes que parecen un fallo del DSP y no lo son. El valor predeterminado de 2 Hz va medio segundo por detrás del mando y es lo bastante discreto para no estorbar al audio. Súbelo donde el puerto de control es un dispositivo propio (un adaptador USB-serie aparte, un rigctld de red); bájalo si el audio de la radio se entrecorta.

    Un Icom con CI-V Transceive activado en su propio menú no necesita nada de esto para el dial: informa del mando en el instante en que se mueve, y sdroxide deja de sondear el dial en cuanto ve que lo hace — y lo retoma si la radio llega a moverse sin avisar.

    La frecuencia es la del dial. El modo va incluido solo cada cuarto sondeo, ya que es un ajuste que cambia unas pocas veces en una tarde y no uno que siga a un mando.
settings-radio-send-command = Enviar comando
settings-radio-which-transceiver-generation-keys-rig =
    Qué generación de transceptor activa el equipo, para el método de PTT «CAT». Las dos discrepan sobre lo que significa el parámetro TX y nada en el cable las distingue, así que elige la que corresponda a tu radio.

    «Estilo TS-2000 (TX;)» — TS-480, TS-570, TS-870, TS-2000: el envío normal, en la banda principal.

    «Estilo TS-590 (TX1;)» — TS-590S/SG, TS-890, TS-990: DATA SEND, que activa con la entrada de audio ACC2/USB activa. En estos equipos el envío normal selecciona el micrófono y silencia el audio que transmite sdroxide.

    Mal configurado, un TS-590 transmite silencio — pero un TS-2000 transmite en la subbanda, que es otra banda por completo.
settings-radio-radio = Radio
settings-radio-k3-k3s-kx3-kx2-k4 = K3 · K3S · KX3 · KX2 · K4
settings-radio-one-profile-covers-family-k3 =
    Un perfil cubre la familia: el conjunto de comandos del K3, que responden también el KX3, el KX2 y el K4. Aquí no hay nada que elegir — cuántos vatios abarca el control de Excitación (12 en un KX2 o KX3 básico, 110 con un KPA3 o un KXPA100) se lee de la consulta de módulos opcionales del propio equipo cuando se abre el puerto.

    Fíjate en los baudios de arriba: el K3, K3S, KX3 y KX2 no pasan de 38400, y un equipo configurado por debajo de la velocidad elegida aquí no responde nada.
settings-radio-transmit-input = Entrada de transmisión
settings-radio-where-radio-takes-transmit-audio =
    De dónde toma la radio su audio de transmisión — el comando TI del equipo, que es el menú 32 «TX IN» del panel frontal. Se establece al abrir el puerto.

    «Audio USB» es lo que hace funcionar esta interfaz: el FDM-DUO transmite lo que sdroxide pone en su tarjeta de sonido USB. Una radio dejada en «Micrófono» envía en cambio la sala, y nada en pantalla lo dice.

    «Auto» deja que elija el equipo — el micrófono para una pulsación de PTT en el micrófono, el puerto USB para una activación por CAT o RTS.

    «Dejar como esté en la radio» no envía ningún TI, para un operador que lo fija en el panel frontal.
settings-radio-which-two-sockets-back-receiver =
    En cuál de los dos conectores de la parte trasera escucha el receptor — el comando AN del equipo, que es el menú 31 «ANTENNAS» del panel frontal y el indicador «ANT 1 2» de su pantalla.

    «RTX» es una antena que hace los dos trabajos, en el conector tipo M que también lleva la transmisión. «Solo RX» mueve la recepción al segundo conector y deja la transmisión en RTX — una antena de recepción, un lazo o una beverage, con el haz todavía en el transmisor.

    Se aplica al instante y se lee de vuelta de la radio al abrir el puerto: es un ajuste del propio equipo y no una copia guardada aquí.
settings-radio-one-profile-covers-both-there =
    Un perfil cubre ambos. Aquí no hay nada que elegir — la radio se nombra a sí misma al abrir el puerto.

    Esto es solo la mitad CAT: maneja el dial, el modo, el PTT, el S-metro, la ROE y la potencia de transmisión por el puerto USB CAT del equipo, y toma el audio de su puerto USB Audio. El tercer puerto USB del FDM-DUO lleva I/Q de banda ancha, que esta interfaz no lee — selecciona arriba la interfaz ELAD para eso.

    Fíjate en los baudios: el menú 70 «CAT BAUD» de la radio debe coincidir, y sale de fábrica en 38400.

    El CW se activa con la propia llave o pala de la radio. El FDM-DUO no tiene ningún comando que acepte texto, así que el panel de CW no puede activarlo por CAT.
settings-radio-one-profile-covers-range-qrp =
    Un perfil cubre la gama: el conjunto de comandos propio de QRP Labs, que es un subconjunto del del Kenwood TS-480 con bastantes añadidos. Aquí no hay nada que elegir — la radio se nombra a sí misma y da su versión de firmware al abrir el puerto, y la versión decide si se pide siquiera la lectura de protección de ROE.

    No es el perfil Kenwood: en un QMX el comando PC es el MEDIDOR de potencia y no el control de potencia, así que una radio manejada como Kenwood vería escrita la lectura de su medidor como si fuera un ajuste, y MD8 — un modo Kenwood — es aquí SWR Tune.

    La potencia de transmisión se fija en la radio. No hay comando CAT para ella, así que el control de Excitación solo alcanza el nivel del audio que entra en la tarjeta de sonido.

    El filtro de recepción también es de la radio: informa del ancho que implica su modo (3,2 kHz en Digi, 300 Hz en CW) y no ofrece nada con lo que cambiarlo.
settings-radio-baud-rate-ignored-port-usb = Los baudios se ignoran — el puerto es USB
settings-radio-qmx-serves-own-virtual-com =
    Un QMX sirve sus propios puertos COM virtuales por USB, así que la velocidad fijada arriba no tiene efecto en ninguno de los dos extremos. Ofrece hasta tres; si la radio no responde nada, lo más probable es que sea el puerto equivocado.

    Hay algo que sí importa del puerto: un retorno de carro enviado a él pone la radio en modo terminal el resto de la sesión. sdroxide nunca envía uno — pero un programa de terminal abierto en el mismo puerto sí lo hará, y el CAT deja de funcionar en cuanto lo haga.
settings-radio-i-q-mode-switched-radio = El modo I/Q se activa en la radio
settings-radio-radio-s-sound-card-carries =
    La tarjeta de sonido de la radio lleva o bien audio demodulado o bien el I/Q en bruto que ve su ADC, y sdroxide establece el que pida el Formato de sonido de arriba al abrir el puerto (el comando Q9 de la radio, la entrada de menú «IQ mode»).

    El desplazamiento del centro y la frecuencia de muestreo de arriba se han fijado a lo que necesita un QMX: su receptor es un superheterodino con una FI de 12 kHz, así que el sintetizador queda 12 kHz por debajo del dial y todo lo que hay en la tarjeta está 12 kHz por encima del centro del ancho. La tarjeta funciona a 48 kHz, lo que hace el panadaptador de 48 kHz de ancho.

    En CW la radio mueve esa FI otros ~700 Hz, que una sola cifra no puede seguir — súmalos a mano si usas el panadaptador con la radio en CW. En Digi, donde normalmente está una radio usada como etapa de entrada I/Q, no hay nada que sumar.

    QRP Labs señala que el modo I/Q no es adecuado para WSJT-X ni para otros programas que esperan audio demodulado; aquí la demodulación ocurre de este lado, así que sí lo es.
settings-radio-rs-hfiq-5-w-hf = RS-HFIQ (transceptor de HF de 5 W)
settings-radio-hobbypcb-s-rs-hfiq-there =
    El RS-HFIQ de HobbyPCB. Aquí no hay nada que elegir: el perfil es todo el conjunto de comandos de la radio, y la versión de firmware se registra al abrir el puerto.

    El enlace de control lleva dos cosas y no más — dónde poner el oscilador y si transmitir. No hay comando de modo, potencia, filtro, silenciador ni medidor, porque la radio no tiene ninguno. Todo lo demás es de sdroxide: lo que sale de la tarjeta de sonido es banda base compleja centrada en el dial, así que el modo, el filtro y la modulación ocurren de este lado.

    El comando de frecuencia cubre 3–30 MHz. Pide algo fuera de eso y la radio lo rechaza y se queda donde está; el registro lo dice.

    El CW se activa como audio a través de la cadena de transmisión — esta radio no tiene manipulador que acepte texto. (Tiene un generador de CW interno; sdroxide nunca lo usa, por consejo del propio firmware.)
settings-radio-sound-format-baud-ptt-are = El formato de sonido, los baudios y el PTT son de la radio, no ajustes
settings-radio-all-three-have-been-filled =
    Los tres se han rellenado y ninguno es una preferencia:

    • El formato de sonido es I/Q (estéreo) — la tarjeta lleva banda base compleja y nada más.
    • 57600 baudios, 8N1 — la única velocidad del firmware, sin menú para cambiarla. Cualquier otra es un enlace mudo, así que se fija al abrir el puerto.
    • PTT por CAT — el comando *X es el único interruptor de transmisión que tiene la interfaz.

    El desplazamiento del centro se ha puesto a cero: el sintetizador funciona a cuatro veces el dial en un detector en cuadratura, así que el centro del ancho es el dial. Fija arriba la frecuencia de muestreo a la que realmente funcione tu tarjeta de sonido; eso es lo que hace el panadaptador tan ancho como es.
settings-radio-dl2man-pe1nnz-s-pocket-qrp =
    El transceptor QRP de bolsillo de DL2MAN/PE1NNZ y el firmware abierto uSDX del que surgió. Emula un Kenwood TS-480, pero el subconjunto es escaso: el dial, el modo, el PTT y la puesta a cero de RIT/XIT es todo lo que responde. Sin S-metro, sin ROE, sin control de potencia, sin VFO B, sin split y sin manipulador CAT, así que el control de Excitación y los medidores no hacen nada por CAT — el nivel de audio es el único control de transmisión que hay.

    El CW se activa como audio (MCW) a través de la banda lateral, o con la llave o pala en la radio; no hay comando que acepte texto.
settings-radio-audio = Audio
settings-radio-tr-usdx-has-no-sound =
    Un (tr)uSDX no tiene tarjeta de sonido propia y tiene dos formas de oírse, y cuál es la correcta es un hecho de tu estación y no de la radio.

    «Un cable» toma el audio dentro del propio enlace serie CAT — el flujo de 8 bits de la propia radio, por el mismo cable USB que el control, sin nada más que enchufar. El inconveniente es el firmware: no puede atender un comando CAT mientras su flujo está en marcha (un comando enviado al flujo no lo pausa, lo mata), así que este modo nunca sondea. Los cambios de sintonía y de modo se envían cuando los haces, cada uno pausa el flujo un momento y lo reinicia, así que espera un breve hueco en cada cambio; no se envía nada mientras simplemente escuchas. Tampoco se siguen el dial y el modo propios de la radio — no hay lectura con la que seguirlos —, así que manéjala desde aquí.

    «Tarjeta de sonido USB» toma el audio de una tarjeta de sonido externa conectada al jack de 3,5 mm de altavoz/micrófono de la radio, y se comporta como cualquier otro equipo CAT: el sondeo del dial funciona, así que se siguen el mando y el modo de la propia radio. Elige la tarjeta en Audio de la radio, más abajo.

    Los dos modos abren el puerto con DTR en alto: el DTR del adaptador serie está cableado al reset de la radio, así que abrir el puerto la reinicia y el primer segundo o dos son silenciosos mientras arranca. El DTR nunca se usa para activar.
settings-radio-radio-model = Modelo de radio
settings-radio-which-icom-two-things-ci =
    Qué Icom, para las dos cosas que CI-V no hace igual en todos ellos.

    La dirección del transceptor de abajo se rellena a partir de él: cada modelo sale con una distinta, y una trama enviada a la dirección equivocada simplemente se ignora — una radio que no responde nada, sin ningún error en ninguna parte que diga por qué.

    También decide si sdroxide puede seleccionar el modo DATA. En CI-V, USB y USB-DATA son el mismo byte de modo y un comando aparte los distingue; sin él un turno en modo digital sale por la entrada de micrófono, con el procesado de voz y el filtro de BLU del equipo en la ruta.

    «Otro» te deja la dirección a ti y no envía ningún comando de modo DATA.
settings-radio-radio-id-hex = ID de la radio (hex)
settings-radio-show-radio-s-spectrum-scope = Mostrar el scope de espectro de la radio
settings-radio-scope-span = Ancho del scope
settings-radio-how-wide-sweep-radio-keeps = Qué ancho barrer. La radio conserva el ancho que se eligió por última vez en su propia pantalla — a menudo unos pocos kHz. Fijar aquí un ancho también pone el scope en modo centrado, de modo que sigue al dial. También cambia la pantalla de la propia radio; «Como esté en la radio» lo deja en paz.
settings-radio-press-apply-reconnect-switch-without = Pulsa «Aplicar / reconectar» para cambiar sin reiniciar.
settings-radio-radio-audio-sound-card = Audio de la radio (tarjeta de sonido)
settings-radio-waiting-sound-cards-machine-radio = Esperando las tarjetas de sonido de la máquina a la que está enchufada la radio.
settings-radio-radio-rx = De la radio (RX)
settings-radio-radio-tx = A la radio (TX)
settings-radio-apply-reconnect = Aplicar / reconectar
settings-radio-reopen-cat-rig-these-sound = Reabrir el equipo CAT con estas tarjetas de sonido — sin reiniciar
settings-radio-reconnects-radio-without-restarting = Reconecta la radio sin reiniciar.
settings-radio-receive-radio-pc = Recepción (radio → PC)
settings-radio-sound-card-radio-s-own = La tarjeta de sonido por la que entra el audio propio de la radio — su toma de auriculares a la entrada de línea o de micrófono del ordenador. Todo lo del panadaptador y los decodificadores llega por aquí.
settings-radio-transmit-pc-radio-mic = Transmisión (PC → micrófono de la radio)
settings-radio-sound-card-carries-audio-radio = La tarjeta de sonido que lleva el audio que la radio debe emitir — la salida del ordenador a la toma de micrófono de la radio. La radio se activa sola, por VOX, en el momento en que llega audio aquí, así que este dispositivo está en silencio salvo que se esté transmitiendo por la radio.
settings-radio-radio-has-no-control-cable = Esta radio no tiene cable de control — se activa sola por VOX, así que no hay PTT que ordenar. La transmisión es audio hacia el micrófono de la radio.
settings-radio-reopen-radio-these-sound-cards = Reabrir la radio con estas tarjetas de sonido — sin reiniciar
settings-radio-devices = Dispositivos
settings-radio-discover = Descubrir
settings-radio-no-devices-press-discover = sin dispositivos — pulsa Descubrir
settings-radio-manual-ip = IP manual
settings-radio-optional-e-g-192-168 = opcional, p. ej. 192.168.1.50
settings-radio-sample-rate = Frecuencia de muestreo
settings-radio-receiver-ddc = Receptor (DDC)
settings-radio-protocol-2-board-carries-several = Una placa del Protocolo 2 lleva varios receptores sintonizables de forma independiente (DDC) en una conexión — ejecuta esta radio en DDC1 y otra radio, con la misma dirección, en DDC2. El transmisor pertenece a la radio de DDC1. Las placas del Protocolo 1 solo tienen DDC1.
settings-radio-lna-gain = Ganancia del LNA
settings-radio-front-end-gain-hermes-lite = Ganancia de la etapa de entrada de un Hermes-Lite 2. Surte efecto de inmediato — sin reconectar — y se recuerda como el nivel con que arranca la radio. Demasiado alta recorta el ADC y toda la banda parece distorsionada; demasiado baja y el receptor se queda sordo.
settings-radio-overload-protection = Protección contra sobrecarga
settings-radio-back-lna-gain-off-itself =
    Baja por sí sola la ganancia del LNA mientras la placa informa de que su ADC se desborda, y la deja subir de nuevo cuando deja de hacerlo.

    Un Hermes-Lite 2 muestrea todo 0-38 MHz en un único conversor de 12 bits sin mezclador ni preselector delante, así que una emisora de radiodifusión a una banda de distancia puede llevarlo al desbordamiento mientras la banda que miras no muestra nada raro — el suelo de ruido sube, todo intermodula y los decodificadores se paran. La placa lo sabe, y esto es lo que actúa en consecuencia.

    Desactivado de forma predeterminada: mueve un control que tú fijas. No ocurre nada mientras se transmite.
settings-radio-wind-gain-back-when-adc = Reducir la ganancia cuando el ADC se desborda
settings-radio-applies-apply-reconnect-main-window = Se aplica con Aplicar / reconectar. El control de Ganancia de la ventana principal lee la ganancia con la que realmente funciona la placa, de modo que sigue al lazo y puedes ver lo que hace; el deslizador de arriba es el nivel con que arranca la radio, y el lazo no lo reescribe.
settings-radio-step = {"  "}Paso
settings-radio-how-far-gain-moves-each = Cuánto se mueve la ganancia cada vez, en dB. Uno es el paso del propio registro de ganancia de la placa.
settings-radio-attack-decay = {"  "}Ataque / caída
settings-radio-how-often-gain-may-come =
    Con qué frecuencia puede bajar la ganancia mientras el conversor se desborda, y con qué frecuencia puede volver a subir una vez que ha parado.

    Los dos están deliberadamente a cien veces de distancia. Retroceder de inmediato: cada milisegundo de desbordamiento es un receptor lleno de intermodulación. Volver despacio: lo que lo haya causado — un vecino activando, una emisora que se levanta al anochecer — normalmente no se ha ido, y un lazo que se recuperara tan deprisa como retrocede pasaría la tarde oscilando a través del umbral. 100 ms y 10 s por decibelio es lo que han usado tanto el Auto S-Att de PowerSDR como el HermesIntf de N1GP.
settings-radio-range = {"  "}Rango
settings-radio-lowest-highest-gain-loop-may = La ganancia más baja y más alta que puede usar el lazo, en dB. El techo es lo que le impide decidir lo sensible que debe ser tu receptor; el suelo es donde dices que por debajo de cierto punto la sobrecarga es problema de otro y la respuesta es un filtro, no otros veinte decibelios.
settings-radio-filter-board = Placa de filtros
settings-radio-accessory-board-hermes-lite-2 = Placa accesoria en el conector J16 del Hermes-Lite 2 (o en las salidas de colector abierto de cualquier otra placa openHPSDR — incluido el Protocolo 2). «N2ADR» elige un relé por banda; «Código de banda Alex / Hermes» pone el número de banda en las salidas 1-4, que es lo que esperan la placa Alex de un ANAN, un Zeus SDR, un HiQSDR y Quisk. «Personalizado» abre una tabla debajo donde indicas tú mismo las siete salidas, banda por banda, en recepción y en transmisión — para un conmutador de antena, el decodificador de banda de un amplificador o cualquier otra cosa a la que no encaje ninguno de los preajustes; cualquiera de los dos preajustes puede volcarse en ella como punto de partida. Déjalo en «Ninguna» salvo que realmente haya algo instalado: esos siete pines son salidas de colector abierto de uso general, y los operadores también los cablean al PTT de amplificadores, a relés de antena y a la conmutación de transversores. Manejarlos con datos de banda empezaría a operar lo que esté conectado. Se aplica con Aplicar / reconectar.
settings-radio-io-board-rx-input = Entrada RX de la placa IO
settings-radio-where-n2adr-hl2ioboard-takes-receive = De dónde toma su señal de recepción una N2ADR HL2IOBoard. La propia placa se encuentra automáticamente y no necesita ajuste; este existe solo para operadores que han cableado sus propios conectores SMA. Déjalo en «Entrada propia de la radio» en los demás casos — seleccionar el J9 de la placa IO sin nada conectado deja el receptor sordo. Se aplica con Aplicar / reconectar. Con cualquiera de las dos opciones J9, las entradas también aparecen en el control ANT y se recuerdan por banda.
settings-radio-power-amplifier = Amplificador de potencia
settings-radio-use-hermes-lite-2-s = Usar el PA integrado del Hermes-Lite 2
settings-radio-default-what-you-want-unless = Activado de forma predeterminada, y lo que quieres salvo que se maneje un amplificador externo desde la salida RF1 de baja potencia de la placa. Con esto desactivado la radio sigue activándose — el relé T/R conmuta y cualquier placa accesoria lo sigue — pero el conector de antena no da ninguna potencia, y el relé se mantiene deliberadamente en recepción. Se ignora en placas que no son un Hermes-Lite.
settings-radio-mirror-board-s-spectrum-about = Refleja el espectro de la placa respecto a la frecuencia sintonizada, tanto en transmisión como en recepción. Activado de forma predeterminada: un Hermes-Lite 2 lo necesita. Desactívalo solo si las señales aparecen del lado equivocado del dial y nada decodifica — la señal delatora es una cascada llena de trazas convincentes mientras la BLU cae en la banda lateral equivocada y FT8 no devuelve ninguna decodificación.
settings-radio-frequency-correction = Corrección de frecuencia
settings-radio-crystal-tcxo-error-ppm-applied = Error del cristal/TCXO en ppm, aplicado a RX y TX. Se aplica al instante.
settings-radio-ppm = {" "}ppm
settings-radio-transmit-buffer = Búfer de transmisión
settings-radio-how-far-ahead-real-time = Con cuánta antelación sobre el tiempo real se alimenta de audio de transmisión a la placa por la red, antes de que sdroxide se ralentice para igualarlo. La propia placa no tiene tal búfer — esto solo ensancha el margen propio de sdroxide. Súbelo en un enlace WiFi o una VPN, donde el valor bajo predeterminado (correcto para una conexión cableada directa) no da holgura suficiente frente a la fluctuación y el audio transmitido o el PTT se entrecortan; más alto cuesta latencia de transmisión. Surte efecto con APLICAR, que reconecta con la placa.
settings-radio-puresignal = PureSignal
settings-radio-adaptive-predistortion-linearise-transmitter-sample = Predistorsión adaptativa: linealiza el transmisor a partir de una muestra de lo que realmente emitió, para unos veinte decibelios menos de intermodulación a la misma potencia. El receptor es el camino de realimentación — la placa sigue recibiendo durante un turno —, así que un acoplador direccional y un atenuador tienen que poner una muestra de la salida del amplificador en una entrada que el conmutador T/R no quite al transmitir. En un Hermes-Lite 2 eso significa el conector PureSignal de la placa IO, y la entrada de recepción de arriba fijada en consecuencia; en una placa sin ella, el puerto RX de un ANAN o un relé externo que conmute el acoplador mientras dure el turno. Es este receptor el que se lee — una placa con un segundo ADC no puede usar RX2 como toma de realimentación, por mucho que su propio firmware lo enrute. Sin nada acoplado, el lazo nunca engancha y el transmisor queda exactamente como habría estado. Se aplica con Aplicar / reconectar, y solo en la radio que posee el transmisor (DDC1).
settings-radio-correct-transmitter-receiver-s-own = Corregir el transmisor con la realimentación del propio receptor
settings-radio-table-steps = {"  "}Pasos de la tabla
settings-radio-how-finely-amplifier-s-curve = Con qué finura se modela la curva del amplificador. Más pasos siguen un codo más agudo y tardan más en rellenarse; 32 es un buen comienzo.
settings-radio-adaptation = {"  "}Adaptación
settings-radio-how-fast-correction-follows-what = A qué velocidad sigue la corrección lo que informa el acoplador. Lento es lo correcto — esto promedia la curva de un amplificador, que no se mueve, a partir de un camino de realimentación que tiene ruido.
settings-radio-hold = Mantener
settings-radio-stop-adapting-keep-correction-where = Deja de adaptar y mantiene la corrección donde está — para medir, y para un operador contento con lo que ha aprendido.
settings-radio-manual-ip-overrides-discovery-press = Una IP manual anula el descubrimiento. Pulsa «Aplicar / reconectar» para cambiar sin reiniciar.
settings-radio-open-collector-outputs-band = Salidas de colector abierto por banda
settings-radio-one-control-word-per-band = Una palabra de control por banda, tal como la indica la documentación de tu hardware: el bit 0 es la salida 1, el bit 6 es la salida 7. RX se activa mientras se recibe en esa banda y TX mientras el transmisor está activado — dales el mismo valor para un filtro, y distintos para cualquier cosa que pertenezca solo a un lado de la conmutación (la línea de activación de un amplificador, el bypass de un preamplificador de recepción). Las bandas a 00 no activan nada. Se aplica con Aplicar / reconectar.
settings-radio-alex-band-code = CÓDIGO DE BANDA ALEX
settings-radio-replace-table-what-preset-would = Sustituye la tabla por lo que enviaría este preajuste en cada banda, como punto de partida para editar. Nada más de la configuración cambia.
settings-radio-clear = BORRAR
settings-radio-every-output-off-every-band = Todas las salidas apagadas en todas las bandas, que es lo que hace «Ninguna».
settings-radio-band = Banda
settings-radio-rx = RX
settings-radio-tx = TX
settings-radio-outputs-asserted = Salidas activadas
settings-radio-other = Otras
settings-radio-everywhere-outside-amateur-bands-short = Todo lo que queda fuera de las bandas de aficionado — escucha de onda corta, y cualquier lugar donde caiga el dial de un transversor que ninguna banda cubra.
settings-radio-dongle = Dongle
settings-radio-rescan = Volver a buscar
settings-radio-re-list-usb-bus-no = Vuelve a listar el bus USB. No se abre ningún dispositivo, así que es seguro pulsarlo mientras se recibe.
settings-radio-no-dongles-press-rescan = sin dongles — pulsa Volver a buscar
settings-radio-first-one-found = — el primero que se encuentre —
settings-radio-rtl2832u-s-resampler-reaches-225 = El remuestreador del RTL2832U alcanza 225–300 kHz y 900 kHz–3,2 MHz, nada entre medias. Surte efecto con Aplicar.
settings-radio-often-drops-samples = {"  "}(a menudo pierde muestras)
settings-radio-agc = AGC
settings-radio-manual-setting-measurement-weak-signal = Manual es el ajuste para medir y para los modos digitales de señal débil. El sintonizador y el demodulador tienen lazos automáticos independientes.
settings-radio-tuner-gain = Ganancia del sintonizador
settings-radio-applies-immediately-no-reconnect-tuner = Se aplica al instante — sin reconectar. El sintonizador tiene 29 pasos discretos, así que el valor se ajusta al más cercano que puede producir. Se ignora mientras el AGC del sintonizador está en marcha.
settings-radio-crystal-error-parts-per-million = Error del cristal en partes por millón. Ejecuta con RUST_LOG=sdroxide_rtlsdr=debug y el registro imprime el error de reloj medido tras unos 20 segundos — ese es el número que hay que introducir. Se aplica al instante.
settings-radio-hf-reception = Recepción de HF
settings-radio-tuner-itself-starts-24-mhz =
    El sintonizador en sí empieza en 24 MHz. Un RTL-SDR Blog V4 convierte hacia arriba por debajo de eso por hardware; otros dongles llegan a HF solo muestreando el ADC directamente, a través del puerto HF del V3. Automático entrega al ADC todo lo que está por debajo de 24 MHz y al sintonizador todo lo que está por encima.

    El muestreo directo cubre todas las bandas de HF, incluidas 17 m y 15 m — llegan en la segunda zona de Nyquist del ADC, del derecho. Sin embargo, nada filtra la entrada del ADC, así que lo que haya en 28,8 MHz menos el dial viene con ellas: 10,7 MHz bajo 17 m, 7,726 MHz bajo 15 m.

    Cambiar de modo interrumpe brevemente el flujo.
settings-radio-removes-dongle-s-own-dc =
    Quita el pico de continua propio del dongle del centro del ancho, y la imagen especular que cada señal deja reflejada respecto a él, midiendo el desequilibrio en las propias muestras — sin calibración, y se aplica al instante. El sintonizador no tiene modo de sintonía con desplazamiento, así que esta es la única forma de limpiar el centro.

    Una portadora de AM sintonizada justo en el dial también está en continua, así que se va con el pico: sintoniza un kilohercio aparte, o desactiva esto.
settings-radio-remove-centre-spike-mirror-image = Quitar el pico central y la imagen especular
settings-radio-bias-tee = Bias tee
settings-radio-feed-4-5-v-dc = Alimentar ~4,5 V CC por el coaxial
settings-radio-bias-tee-never-connect-transceiver = El bias tee está ACTIVADO. No conectes nunca un transceptor, una antena a masa ni un preamplificador alimentado desde otro sitio mientras esto esté activado — la corriente continua va directa por la línea de alimentación.
settings-radio-receive-only-dongle-sample-rate = Solo recepción. El dongle y la frecuencia de muestreo surten efecto con Aplicar; todo lo demás se aplica a medida que lo cambias.
settings-radio-server-address = Dirección del servidor
settings-radio-where-rtl-tcp-listening-address =
    Dónde escucha rtl_tcp: una dirección, o una dirección y un puerto. El puerto es por omisión 1234, que es el valor propio de rtl_tcp.

    En el otro extremo, ejecútalo como `rtl_tcp -a 0.0.0.0` — enlazado a 127.0.0.1, que es lo que hace sin -a, solo acepta conexiones de esa misma máquina.

    Surte efecto con Aplicar.
settings-radio-host-host-port-e-g = host o host:puerto, p. ej. raspberrypi.local:1234
settings-radio-requested-server-same-resampler-limits =
    Se solicita al servidor, con los mismos límites de remuestreador que un dongle local — es el mismo silicio en el otro extremo.

    La cifra junto a cada frecuencia es lo que cuesta en el enlace: las muestras se envían sin comprimir, y una frecuencia que la red no puede transportar hace que rtl_tcp corte la conexión en lugar de degradarse. Surte efecto con Aplicar.
settings-radio-wired-link = {"  "}(enlace cableado)
settings-radio-runs-server-s-dongle-manual = Funciona en el dongle del servidor. Manual es el ajuste para medir y para los modos digitales de señal débil.
settings-radio-applies-immediately-no-reconnect-sent = Se aplica al instante — sin reconectar. Se envía en décimas de dB y el servidor lo ajusta a un paso que tenga su sintonizador; el protocolo no tiene respuestas, así que lo que se decidió no puede leerse de vuelta y este deslizador sigue mostrando lo que se pidió. Se ignora mientras el AGC del sintonizador está en marcha.
settings-radio-crystal-error-server-s-dongle =
    Error del cristal del dongle del *servidor*, en partes por millón — una propiedad de ese hardware, así que se fija aquí y no en los dongles de esta máquina. Se aplica al instante.

    El error de reloj medido que imprime la interfaz USB no está disponible aquí: por una red, lo que esa medición ve es el almacenamiento en búfer y no el cristal, y se equivoca en miles de ppm. Calibra el dongle por USB una vez y lleva el número, o sintoniza una emisora de radiodifusión de frecuencia conocida y ajusta hasta que quede en el dial.
settings-radio-tuner-starts-24-mhz-below =
    El sintonizador empieza en 24 MHz; por debajo de eso el otro extremo necesita ayuda. Un Blog V4 convierte hacia arriba por sí mismo y Automático lo deja en paz — que es lo único que puede hacer, ya que el protocolo informa del chip del sintonizador y nada más, y un V4 no se distingue de un R828D normal por el cable.

    En un V3 o cualquier otro dongle, Automático cambia el servidor a muestreo directo por debajo del suelo de 24 MHz del propio sintonizador — lo que cubre todas las bandas de HF, con 17 m y 15 m llegando en la segunda zona de Nyquist del ADC con 28,8 MHz menos el dial plegado encima. Elige explícitamente Muestreo directo para un R828D normal que no oye nada en HF. Cambiar interrumpe brevemente el flujo.
settings-radio-removes-dongle-s-dc-spike =
    Quita aquí el pico de continua y la imagen especular del dongle, a partir de las muestras a medida que llegan — son artefactos del hardware, así que viajan por la red con todo lo demás. Se aplica al instante.

    Una portadora de AM sintonizada justo en el dial también está en continua, así que se va con el pico: sintoniza un kilohercio aparte, o desactiva esto.
settings-radio-powers-preamp-far-end-s = Alimenta un preamplificador desde el dongle del otro extremo. Los servidores más antiguos no implementan el comando y lo ignoran en silencio — el protocolo no tiene forma de decir que no — así que un bias tee que no se enciende no es necesariamente cosa de este extremo.
settings-radio-feed-4-5-v-dc-2 = Alimentar ~4,5 V CC por el coaxial remoto
settings-radio-bias-tee-hardware-somewhere-else = El bias tee está ACTIVADO, en hardware que está en otro lugar. Lo que haya en el otro extremo de esa línea de alimentación — un transceptor, una antena a masa, un preamplificador ya alimentado — no está delante de ti para comprobarlo.
settings-radio-sdrplay-server-rsp-tcp = Servidor SDRplay (rsp_tcp)
settings-radio-sdrplay-server-greets-exactly-like = Un servidor SDRplay saluda exactamente igual que un dongle, así que estos se muestran siempre y no solo cuando se detecta uno. Contra un servidor rtl_tcp normal simplemente se ignoran — el protocolo no tiene respuestas y descarta los comandos que no conoce. La pestaña Dispositivo nombra el servidor como rsp_tcp una vez que se ha identificado.
settings-radio-input = Entrada A
settings-radio-input-b = Entrada B
settings-radio-hi-z = Hi-Z
settings-radio-lna-state = Estado del LNA
settings-radio-step-index-not-db-figure = Un índice de paso, no una cifra en dB: lo que vale cada paso depende del modelo de RSP y de la banda. 0 es la mayor ganancia.
settings-radio-if-gain-reduction = Reducción de ganancia de FI
settings-radio-reduction-so-more-less-signal = Una reducción, así que más es menos señal. Solo se obedece con el AGC del RSP desactivado.
settings-radio-enable = Activar
settings-radio-dbfs = {" "}dBfs
settings-radio-level-rsp-s-agc-aims = El nivel que el AGC del RSP intenta mantener.
settings-radio-notches = Notches
settings-radio-am = AM
settings-radio-fm = FM
settings-radio-dab = DAB
settings-radio-rf = RF
settings-radio-reference-out = Salida de referencia
settings-radio-t-24-mhz-clock-out = Salida de reloj de 24 MHz
settings-radio-rsp2-rspduo-only = Solo RSP2 y RSPduo.
settings-radio-receive-only-address-sample-rate = Solo recepción. La dirección y la frecuencia de muestreo surten efecto con Aplicar; todo lo demás se aplica a medida que lo cambias. Una conexión perdida se reintenta sola, así que un servidor que se reinicia vuelve sin tocar nada aquí.
settings-radio-where-spyserver-listening-address-address =
    Dónde escucha spyserver: una dirección, o una dirección y un puerto. El puerto es por omisión 5555, que es el valor propio de spyserver.

    En el otro extremo, comprueba que su archivo de configuración se enlaza a una dirección que otras máquinas puedan alcanzar — enlazado a 127.0.0.1 solo acepta conexiones de esa misma máquina.

    Surte efecto con Aplicar.
settings-radio-host-host-port-e-g-2 = host o host:puerto, p. ej. raspberrypi.local:5555
settings-radio-i-q-bandwidth = Ancho de banda I/Q
settings-radio-how-wide-slice-band-arrives =
    Qué ancho de la banda llega como I/Q — y, por tanto, cuánto de este enlace usa. Es la ventana *demodulada*: sigue al dial, y todo lo más ancho que ella es la FFT del servidor en la franja sobre el panadaptador.

    Automático apunta a unos 96 kHz, que lleva todos los modos de aquí, incluida la FM de banda ancha, y aun así cabe por un enlace ascendente móvil. Surte efecto con Aplicar.
settings-radio-which-stage-server-s-own =
    Qué escalón de la escalera de frecuencias propia del servidor pedir. Cada receptor tiene una escalera distinta — es su frecuencia máxima dividida a la mitad escalón a escalón —, así que se guarda como el escalón y no como una cifra en hercios, y el mismo ajuste sigue significando algo razonable apuntado a otro servidor.

    Automático apunta a cerca de 1 Msps. Pulsa Probar conexión para ver a qué equivalen los escalones en este servidor. Surte efecto con Aplicar.
settings-radio-automatic = Automático
settings-radio-sample-format = Formato de muestra
settings-radio-bits-per-component-wire-so =
    Bits por componente en el cable, y por tanto lo que cuesta en el enlace: 16 bits es el doble que 8 bits, y el float de 32 bits es cuatro veces más, sin más información de la que tenía el ADC del receptor.

    8 bits es lo que hace que un receptor remoto funcione por un enlace ascendente doméstico y es lo correcto para casi todo. Un servidor puede anular esto — algunos están configurados para insistir en un formato — y lo dice en el registro cuando lo hace. Surte efecto con Aplicar.
settings-radio-gain = Ganancia
settings-radio-server-s-gain-stage-index =
    La etapa de ganancia del servidor, como un índice — no un número de decibelios. Lo que significa cada índice pertenece al receptor del otro extremo y cambia con la banda, y el protocolo nunca lo dice, así que nada aquí puede convertirlo en dB sin inventar una cifra.

    El rango real es el del servidor y solo se conoce una vez conectado; un índice que lo supere se recorta. Se aplica al instante — sin reconectar.
settings-radio-index = índice{" "}
settings-radio-digital-gain = Ganancia digital
settings-radio-how-far-server-scales-samples =
    Cuánto escala el servidor sus muestras antes de cuantizarlas para el cable. Automático observa lo que realmente llega y mantiene el pico a la mitad de la escala completa, que es lo único que puede ser correcto tanto en una banda muerta como en una concurrida — las dos están a treinta decibelios y ninguna cifra fija sirve para ambas.

    Importa solo a 8 bits, donde una señal muy por debajo de la escala completa pierde sus bits bajos en el cuantizador y una por encima se recorta plana. Los formatos más anchos tienen holgura de sobra y reciben la cifra que calculan todos los demás clientes, a partir del índice de ganancia y la etapa de diezmado. Se aplica al instante.
settings-radio-full-band-strip = Franja de banda completa
settings-radio-server-s-own-fft-whole =
    La FFT propia del servidor de toda la banda, dibujada en la franja sobre el panadaptador. En esta interfaz es la única vista de banda que hay — el propio panadaptador solo es tan ancho como el I/Q que se recibe.

    Desactivarla deja a un receptor sin forma de ver nada que no tenga ya sintonizado. Se aplica al instante, con un breve corte en el audio mientras el servidor cambia lo que envía.
settings-radio-ask-server-low-rate-fft =
    Pide al servidor una FFT de baja frecuencia de toda la banda además del I/Q, y la dibuja en la franja sobre el panadaptador.

    No cuesta casi nada — un par de kilobytes por trama, una docena de veces por segundo — y muestra todo el receptor en lugar de la porción que se demodula. Solo merece desactivarla en un enlace donde cada byte cuenta. Se aplica al instante, con un breve corte en el audio.
settings-radio-show = Mostrar
settings-radio-whole-band = toda la banda
settings-radio-how-much-receiver-strip-covers = Cuánto del receptor cubre la franja. La banda completa es la vista más ancha que hay; estrecharla pone el mismo número de bins sobre menos espectro, que es más detalle en un tramo menor.
settings-radio-strip-db-window = Ventana de dB de la franja
settings-radio-server-quantises-fft-into-window =
    El servidor cuantiza su FFT en esta ventana antes de enviarla, un byte por bin — así que esto decide con qué finura se mide la franja, no solo cómo se dibuja. El suelo y el techo con que se *muestra* la franja los fija el autonivelado del propio motor y son otra cosa.

    Los 150 dB predeterminados son todo el rango del protocolo y no requieren atención. Estrecharlo alrededor del suelo de ruido compra resolución en un receptor cuyas señales están todas en una parte pequeña de la escala. Se aplica al instante.
settings-radio-top = máx.{" "}
settings-radio-range-2 = rango{" "}
settings-radio-i-q-correction = Corrección I/Q
settings-radio-remove-dc-spike-mirror-image = Quita el pico de continua y la imagen especular en el DSP, de este lado. Si el receptor del otro extremo lo necesita depende de lo que sea — un Airspy HF+ no, un RTL-SDR sí — y el protocolo no dice con cuál está hablando. Se aplica al instante.
settings-radio-enabled = Activado
settings-radio-test-connection = Probar conexión
settings-radio-connect-read-what-server-says = Conecta, lee lo que dice el servidor sobre su receptor y vuelve a desconectar — sin iniciar un flujo, así que es seguro pulsarlo contra un servidor que usa otra persona.
settings-radio-receive-only-panadapter-narrow-i = Solo recepción. El panadaptador es la estrecha ventana I/Q, que sigue al dial; la vista de banda es la FFT del servidor en la franja sobre él. Pulsa «Aplicar / reconectar» para cambiar sin reiniciar.
settings-radio-receive-only-wideband-i-q = Solo recepción. I/Q de banda ancha, igual que cualquier SDR local. Pulsa «Aplicar / reconectar» para cambiar sin reiniciar.
settings-radio-receiver-address = Dirección del receptor
settings-radio-where-receiver-serves-web-page =
    Dónde sirve el receptor su página web: una dirección, o una dirección y un puerto. El puerto es por omisión 8073, que es el propio de un KiwiSDR.

    Ten en cuenta que un receptor alcanzado a través del proxy del proyecto (algo.proxy.kiwisdr.com — casi la mitad de los públicos) responde en cambio en el puerto 80, así que indícalo explícitamente. Elegir un receptor de «SDR públicos» lo rellena correctamente.

    Surte efecto con Aplicar.
settings-radio-host-host-port-e-g-3 = host o host:puerto, p. ej. kiwi.example.org:8073
settings-radio-password = Contraseña
settings-radio-receiver-s-user-password-where =
    La contraseña de *usuario* del receptor, donde su operador haya puesto una. En blanco en casi todos los receptores públicos. No la contraseña de administrador.

    Surte efecto con Aplicar.
settings-radio-announce = Anunciarse como
settings-radio-name-connection-shows-up-under =
    El nombre con el que aparece esta conexión, que pueden ver el propietario del receptor y todos los demás que lo escuchan.

    En blanco es el indicativo de tu estación, que es la convención de la propia red — algunos operadores sí bloquean a los clientes que no se identifican. Surte efecto con Aplicar.
settings-radio-blank-station-callsign = en blanco = indicativo de la estación
settings-radio-band-view = Vista de banda
settings-radio-ask-receiver-own-waterfall-well =
    Pide al receptor su propia cascada además de su I/Q, y la dibuja en la franja sobre el panadaptador.

    Merece la pena: el I/Q tiene solo unos 12 kHz de ancho, así que sin esto no hay nada *por* lo que sintonizar. Es una segunda conexión que lleva aproximadamente 20 kB/s a plena velocidad, frente a los 44 kB/s del I/Q.

    Surte efecto con Aplicar.
settings-radio-show-receiver-s-0-30 = mostrar la cascada de 0-30 MHz del receptor
settings-radio-band-view-speed = Velocidad de la vista de banda
settings-radio-how-often-receiver-sends-waterfall =
    Con qué frecuencia envía el receptor una fila de cascada, de 1 (la más lenta) a 4. El único ajuste de aquí que cambia lo que cuesta el enlace mientras funciona, así que bájalo en una conexión con tarifa limitada.

    Se aplica al instante.
settings-radio-band-view-span = Ancho de la vista de banda
settings-radio-how-much-band-receiver-s =
    Cuánta banda cubre la cascada del receptor. Sus 0-30 MHz completos a la izquierda del deslizador, reduciéndose a la mitad con cada paso a la derecha.

    La cascada siempre tiene 1024 bins de ancho, sea cual sea el ancho de la ventana, así que esto es realmente un control de resolución: la banda completa son 29 kHz por bin, que es un *mapa* de banda más que una imagen de nada, mientras que unos cientos de kilohercios muestran estaciones individuales en una banda de radiodifusión. Con zoom, la ventana sigue a tu dial.

    La banda completa es el valor predeterminado y es lo que hace de la franja algo por lo que sintonizar; el panadaptador de debajo son los ~12 kHz de I/Q. Se aplica al instante.
settings-radio-receiver-agc = AGC del receptor
settings-radio-receiver-s-agc-which-far =
    El AGC del *receptor*, que está en el lado lejano del enlace y delante del I/Q — así que actúa antes que cualquier cosa de sdroxide.

    Activado de forma predeterminada, a diferencia de todos los SDR locales de aquí, porque la alternativa medida era peor: en un receptor real la ganancia manual no era monótona a lo largo de su rango y su extremo superior recortaba el I/Q a fondo de escala, mientras que el AGC mantenía unos -24 dBFS con 7 dB de margen.

    Significa que la amplitud de las muestras no es un nivel de señal, por lo que el S-metro se lee en cambio de la cifra del propio receptor. Se aplica al instante.
settings-radio-let-receiver-ride-own-gain = dejar que el receptor lleve su propia ganancia
settings-radio-manual-gain = Ganancia manual
settings-radio-fixed-gain-when-receiver-s = Ganancia fija cuando el AGC del receptor está desactivado, en su propia escala 0-90. No son decibelios de nada declarado — el protocolo lo llama manGain y no dice más, y se midió que no es monótona, así que trátala como un dial en el que encontrar un buen punto más que como una calibración. Se aplica al instante.
settings-radio-read-receiver-s-status-page = Lee la página de estado del receptor: qué es, qué cubre, cuántos canales quedan libres y si su operador permite conexiones desde aplicaciones que no sean un navegador. No ocupa ninguno de sus canales.
settings-radio-host-port-e-g-127 = host:puerto, p. ej. 127.0.0.1:50001
settings-radio-iq-sample-rate = Frecuencia de muestreo IQ
settings-radio-receiver = Receptor
settings-radio-rig-two-receivers-sunsdr2dx-can = Un equipo con dos receptores (SunSDR2DX) puede servir dos pestañas de radio desde una conexión — ejecuta esta radio en RX1 y otra en RX2. El transmisor pertenece a la radio de RX1.
settings-radio-stream-delay = Retardo del flujo
settings-radio-how-long-rig-s-iq = Cuánto tarda en llegar el IQ del equipo en un centro nuevo después de que sdroxide lo mueva. Es la propia canalización de DSP del equipo, no la red — un servidor en esta máquina también la tiene — y mientras se arrastra el panadaptador con el zoom al mínimo, equivocarse en esto mueve las filas más recientes de la cascada hacia un lado respecto al historial en el error por la velocidad del arrastre. Demasiado alto las desplaza exactamente tanto como demasiado bajo, en el otro sentido. Mídelo con `cargo run --release -p sdroxide-tci --example retune_latency`; el valor predeterminado es un SunSDR2DX con ExpertSDR3 a 192 kHz.
settings-radio-wideband-iq-receive-audio-transmit = Recepción IQ de banda ancha, transmisión de audio. Pulsa «Aplicar / reconectar» para cambiar sin reiniciar.
settings-radio-radio-address = Dirección de la radio
settings-radio-address-shown-radio-under-set = La dirección que muestra la radio en SET > Network. Allí tiene que estar activado Network Control, y la radio necesita un nombre de usuario y una contraseña de red.
settings-radio-host-ip-e-g-192 = host o IP, p. ej. 192.168.1.50
settings-radio-control-port = Puerto de control
settings-radio-t-50001-unless-has-been-changed = 50001 salvo que se haya cambiado en la radio.
settings-radio-network-user = Usuario de red
settings-radio-stored-clear-radio-json-protocol = Guardada en claro en radio.json. El protocolo la ofusca de forma reversible en el cable, así que nada de esto la convertiría en un secreto.
settings-radio-receive = Recibir de
settings-radio-af-radio-demodulates-sdroxide-shows = AF: la radio demodula y sdroxide muestra la banda de audio. FI de 12 kHz: la radio envía en cambio su frecuencia intermedia DRM y sdroxide demodula, lo que aporta sus propios filtros, reducción de ruido y decodificadores sobre unos ±12 kHz. En cualquier caso la cascada ancha es el propio scope de la radio — ningún Icom saca I/Q. Una preferencia de FI usa temporalmente AF para modos más anchos como WFM, y luego restaura FI para los modos compatibles. Una preferencia de AF sigue siendo AF. HD Radio no está disponible en esta conexión.
settings-radio-audio-sample-rate = Frecuencia de muestreo del audio
settings-radio-t-12-khz-if-needs-48000 = Una FI de 12 kHz necesita el flujo de 48000 Hz — con esta frecuencia se usa en su lugar el audio demodulado de la radio.
settings-radio-if-spectrum = Espectro de FI
settings-radio-which-way-round-radio-s = En qué sentido corre la FI de 12 kHz de la radio. «Automático» usa lo que se sabe que hace el modelo — espejado en un IC-7760, normal en todos los demás Icom. Fíjalo a mano si la BLU sale en la banda lateral equivocada: la señal delatora es tener que seleccionar USB donde la banda es LSB mientras la pantalla de modo de la propia radio coincide con sdroxide en todo momento.
settings-radio-normal = Normal
settings-radio-mirrored = Espejado
settings-radio-displayed-bandwidth = Ancho de banda mostrado
settings-radio-width-audio-band-panadapter-cat = Ancho del panadaptador de banda de audio, como en un equipo CAT. Se usa en los modos digitales y donde el scope de la radio está apagado; en otro caso el panadaptador es el scope, y el Ancho del scope fija su anchura.
settings-radio-how-cw-panel-s-keyer-2 = Cómo transmite el manipulador del panel de CW. «Manipulador del equipo» pone la radio en CW y le entrega el texto para enviar con su propio manipulador (CI-V 0x17). «Tarjeta de sonido» envía en cambio el tono lateral manipulado por la LAN como audio (MCW), un tono en dial + tono — y como un equipo en CW ignoraría su entrada de modulador, seleccionar CW mantiene entonces la radio en USB normal, el mismo modo en que van los modos digitales.
settings-radio-how-much-audio-radio-holds = Cuánto audio retiene la radio antes de modular. Más aguanta una red peor, a costa de latencia de transmisión.
settings-radio-streams-radio-s-own-475 = Transmite el barrido propio de 475 bins de la radio. En AF es el panadaptador — el audio que envía la radio es lo que pasó por su filtro, no una imagen de la banda — y en la FI de 12 kHz es la cascada de banda completa sobre uno. En cualquier caso es la imagen de la radio, no el DSP de sdroxide: no hay I/Q con el que calcular una.
settings-radio-how-wide-sweep-only-wide = Qué ancho barrer. Es la única vista ancha que tiene un Icom, y la radio conserva el ancho que se eligió por última vez en su propia pantalla — a menudo unos pocos kHz, por lo que la franja puede salir no más ancha que el panadaptador de debajo. Fijar aquí un ancho también pone el scope en modo centrado, de modo que sigue al dial. También cambia la pantalla de la propia radio; «Como esté en la radio» lo deja en paz.
settings-radio-switch-modulation-input-lan = Cambiar la entrada de modulación a LAN
settings-radio-transmit-audio-only-heard-when = El audio de transmisión solo se oye cuando la entrada MOD de la radio está en LAN. sdroxide puede escribirlo en un modelo cuya numeración de menú conoce; en cualquier otro lo dice y deja el menú en paz. Es un préstamo: lo que tuviera la radio ahí se lee primero y se restituye al terminar la sesión, de modo que un equipo usado solo después sigue oyendo su propio micrófono.
settings-radio-copy-diagnostic-report = Copiar informe de diagnóstico
settings-radio-copies-radio-s-last-session = Copia al portapapeles la última sesión de esta radio — su negociación y su traza CI-V — para un informe de error. De esta radio, no de la estación: con dos Icom en la LAN cada pestaña responde de su propia dirección.
settings-radio-control-audio-radio-s-own = Control, audio y el scope propio de la radio por una sola conexión de red — sin cable serie y sin tarjeta de sonido. Pulsa «Aplicar / reconectar» para cambiar sin reiniciar.
settings-radio-testing = Probando…
settings-radio-was-only-check-press-apply = Eso fue solo una comprobación — pulsa Aplicar / reconectar abajo para empezar a usar esta radio.
settings-radio-radios = Radios
settings-radio-asks-network-iio-devices-also = Pregunta a la red por dispositivos IIO, y además prueba 192.168.2.1 directamente — un Pluto al extremo de un cable USB a menudo es inalcanzable por multicast aunque la dirección funcione.
settings-radio-no-radios-press-discover = sin radios — pulsa Descubrir
settings-radio-address = Dirección
settings-radio-overrides-selection-above-usb-cable = Anula la selección de arriba. El cable USB presenta el Pluto como un adaptador de red, no como un puerto serie, así que esto es una dirección IP incluso cuando la radio está en tu mesa.
settings-radio-pluto-revision-c-pluto-unlocked = Un Pluto+ o un Pluto de revisión C desbloqueado a 2R2T puede servir dos pestañas de radio desde una caja — esta radio en RX1 y otra en RX2, cada una con su propia antena. Las dos cadenas comparten el único oscilador, así que resintonizar cualquiera de las radios mueve ambas; lo que compra RX2 es una segunda antena sobre el mismo espectro (diversidad), no una segunda banda. El transmisor pertenece a la radio de RX1. Un Pluto 1R1T de serie rechaza RX2 al conectar.
settings-radio-width-spectrum-sdroxide-receives-ad9361 = Ancho del espectro que recibe sdroxide. El AD9361 llega a 61,44 Msps; el enlace de red USB no, que es a lo que se escala esta lista. Las frecuencias más bajas necesitan una configuración de filtro cargada en el AD9361, que sdroxide no hace — un Pluto de serie las ejecuta en cambio a unos 2,084 Msps y lo dice al conectar. Surte efecto con Aplicar.
settings-radio-more-than-usb-2-will = {"  "}(más de lo que llevará USB 2)
settings-radio-stock-pluto-runs-2-084 = {"  "}(un Pluto de serie funciona a 2,084)
settings-radio-analog-filter = Filtro analógico
settings-radio-ad9361-s-baseband-filter-leave = El filtro de banda base del AD9361. Déjalo en 0 para automático, que lo abre a nueve décimas de la frecuencia de muestreo — ancho a propósito, porque el receptor aparca su oscilador a un cuarto de ancho del dial para mantener las señales lejos del pico de continua, y un filtro estrecho cortaría justo la parte a la que las movió. Surte efecto con Aplicar.
settings-radio-ad9361-has-four-modes-not = El AD9361 tiene cuatro modos, no un interruptor de encendido/apagado. El ataque lento va bien para BLU y CW; el ataque rápido para señales a ráfagas; manual es el ajuste para medir y para los modos digitales de señal débil. Se aplica al instante.
settings-radio-rx-gain = Ganancia RX
settings-radio-applies-immediately-no-reconnect-ignored = Se aplica al instante — sin reconectar. Se ignora salvo que el AGC esté en manual, lo que es comportamiento propio del AD9361, no de sdroxide.
settings-radio-tx-gain = Ganancia TX
settings-radio-negative-because-ad9361-expresses-transmit = Negativa porque el AD9361 expresa el nivel de transmisión como atenuación: 0 dB es la salida completa. Se aplica al instante. El transmisor se pone en lo más silencioso antes de aplicar este valor al conectar, de modo que nada de lo que dejó el programa anterior puede estar activo.
settings-radio-reference-error-parts-per-million = Error de referencia en partes por millón. Ejecuta con RUST_LOG=sdroxide_pluto=debug y el registro imprime el error de reloj medido tras unos 20 segundos — ese es el número que hay que introducir. Lo aplica sdroxide, no se escribe en el ajuste persistente propio de la radio. Se aplica al instante.
settings-radio-full-duplex = Dúplex completo
settings-radio-keep-receiving-while-transmitting = Seguir recibiendo mientras se transmite
settings-radio-leave-off-pluto-reached-over =
    Déjalo desactivado en un Pluto alcanzado por su cable USB. Ese enlace no puede llevar un megamuestra por segundo en ambos sentidos a la vez, y un turno que deja sin datos el búfer de transmisión sale entrecortado — así que por omisión la recepción se detiene mientras dura un turno.

    Actívalo para una placa con Ethernet de verdad (una LibreSDR, un Pluto con un adaptador gigabit), donde hay sitio para ambos: entonces oyes el receptor a través de tu propia transmisión, que es como una estación de QO-100 escucha su propio enlace descendente.

    Un turno necesita el doble de la frecuencia de muestreo en enlace — 2,5 Msps son 10 MB/s en cada sentido — así que baja la frecuencia si el registro empieza a decir que el enlace no lo soporta. El panadaptador sigue mostrando la señal transmitida durante un turno; lo que sigue llegando es el audio.

    Surte efecto con Aplicar.
settings-radio-duplex = Dúplex
settings-radio-whether-ad9361-runs-both-directions =
    Si el AD9361 funciona en ambos sentidos a la vez (FDD, que es como arranca un Pluto y en lo que sdroxide siempre lo ha dejado) o de uno en uno (TDD).

    Déjalo en FDD salvo que quieras los pines PTT de abajo — TDD es de lo que se activan, y descarta el Dúplex completo de arriba. Surte efecto con Aplicar.
settings-radio-ptt-pins = Pines PTT
settings-radio-pluto-s-four-gpo-test =
    Los cuatro puntos de prueba GPO del Pluto pueden activar por sí solos un amplificador de potencia externo, un LNA o un conmutador transmisión-recepción. Elige una pareja y un pin está en alto todo el tiempo que la radio recibe, el otro todo el tiempo que transmite — sin software del host en el lazo y sin línea PTT serie que cablear.

    Esto pone la radio en TDD diga lo que diga la fila Dúplex, porque los pines siguen las líneas de habilitación del AD9361 y en FDD las dos están activadas durante toda la sesión. El Dúplex completo se desactiva con él, así que deja ambos en sus valores predeterminados si trabajas satélites y necesitas oír tu propio enlace descendente.

    La propia nota de Analog Devices pone un LNA externo en GPO0/GPO1, así que usa GPO2/GPO3 si tu placa está cableada así. Los pines están a unos 1,3 V a unos pocos miliamperios: maneja con ellos un transistor o un optoaislador, nunca la bobina de un relé.

    Surte efecto con Aplicar.
settings-radio-buffer-size = Tamaño del búfer
settings-radio-how-much-device-holds-before =
    Cuánto retiene el dispositivo antes de cada transferencia, en muestras complejas. El valor predeterminado de 32768 son unos 16 ms a 2 Msps: lo bastante largo para que el viaje de ida y vuelta de cada transferencia no sea el cuello de botella, lo bastante corto para que una resintonía no llegue visiblemente tarde.

    Redúcelo a la mitad si el registro informa de que se sustituye el socket de recepción. Un enlace marginal — un Pluto alcanzado por su cable USB a una frecuencia de muestreo alta es el caso habitual — se atasca a mitad de una transferencia, y una transferencia más pequeña tiene menos probabilidad de ser sorprendida en pleno vuelo y es más rápida de reparar después. Súbelo para cambiar latencia de resintonía por menos viajes de ida y vuelta.

    Surte efecto con Aplicar.
settings-radio-samples = {" "}muestras
settings-radio-rx-tx-port = Puerto RX / TX
settings-radio-ad9361-s-rf-port-select = El rf_port_select del AD9361. Un Pluto de serie cablea uno de cada, así que déjalos vacíos salvo que tengas una placa que no lo haga. Surte efecto con Aplicar.
settings-radio-opens-radio-reads-what-says = Abre la radio, lee lo que dice de sí misma e informa del rango de sintonía que tiene esta placa en concreto. No inicia un flujo.
settings-radio-copies-last-session-s-protocol = Copia al portapapeles la traza de protocolo de la última sesión, para un informe de error.
settings-radio-wideband-iq-receive-transmit-half = Recepción y transmisión IQ de banda ancha. Semidúplex — la recepción se detiene mientras dura un turno, porque el enlace de red USB no lleva ambos a la vez. No hacen falta ni SoapySDR ni libiio.
settings-radio-not-yet-verified-against-real = Aún no verificado con hardware real. Si se porta mal, envía el informe de diagnóstico — contiene todos los comandos intercambiados con la radio, y los primeros bytes del flujo de muestras.
settings-radio-flexradio-announces-itself-local-network = Una FlexRadio se anuncia sola en la red local aproximadamente una vez por segundo. Una radio alcanzada a través de un router o una VPN nunca emite hacia ti — introduce su dirección abajo en su lugar.
settings-radio-another-gui-client-has-radio = Otro cliente con interfaz tiene esta radio y multiFLEX está desactivado. Desconecta ese cliente, o activa multiFLEX en la radio.
settings-radio-overrides-selection-above-use-radio = Anula la selección de arriba. Úsalo para una radio en otra subred, tras una VPN o en un puerto no estándar.
settings-radio-width-spectrum-sdroxide-receives-192 = Ancho del espectro que recibe sdroxide. 192 kHz es el máximo de la radio para un flujo DAX IQ, y por tanto el ancho más amplio que puede mostrar esta interfaz.
settings-radio-t-192-khz-widest-dax-iq = 192 kHz es el flujo DAX IQ más ancho que enviará una FLEX, y la radio ata el ancho del panadaptador a él — así que ese es todo el ancho, y ningún ajuste de aquí lo amplía.
settings-radio-dax-iq-channel = Canal DAX IQ
settings-radio-radio-has-four-change-only = La radio tiene cuatro. Cámbialo solo si algo más de la red ya usa el canal 1 — la radio rechaza un canal dos veces.
settings-radio-station-name = Nombre de la estación
settings-radio-shown-against-session-radio-s = Se muestra junto a esta sesión en la lista de clientes de la radio. La radio también recuerda a un cliente por él, así que cambiarlo hace que la radio trate a sdroxide como uno nuevo.
settings-radio-gui-client-id = ID de cliente GUI
settings-radio-identity-radio-restores-session-s = La identidad a la que la radio restaura los slices y panadaptadores de esta sesión. Vacía se deriva del nombre de la estación — estable, pero igual en todos los sdroxide que conservaron el nombre predeterminado, y una radio resuelve un duplicado echando al cliente anterior. sdroxide lo detecta en el cable y recurre a una identidad de una sola sesión, lo que cuesta la restauración; pon aquí un UUID propio para conservarla.
settings-radio-optional-e-g-uuid-your = opcional, p. ej. un UUID propio
settings-radio-mirror-radio-s-i-q =
    Refleja el I/Q de la radio respecto al centro del panadaptador. Desactivado de forma predeterminada, que es como se verificó una FLEX-6600.

    Pruébalo si el audio de recepción es ininteligible tanto en USB *como* en LSB y nada decodifica, mientras la cascada parece del todo convincente — eso es lo que parece un flujo espejado, y es el único fallo sin otro síntoma (incidencia n.º 368). Si no es eso, activarlo lo hace evidente en lugar de sutil. Se aplica con Aplicar / reconectar.
settings-radio-network-mtu = MTU de red
settings-radio-largest-datagram-radio-may-send = El mayor datagrama que puede enviar la radio. 1450 es lo que pide el propio SmartSDR. Bájalo en un camino con una MTU menor — una VPN o un túnel — donde se descartan los fragmentos y no llega ningún espectro.
settings-radio-b = {" "}B
settings-radio-checks-radio-answers-without-registering = Comprueba que la radio responde, sin registrarse como cliente con interfaz — así que no molestará a una sesión de SmartSDR ya en marcha.
settings-radio-wideband-iq-receive-over-dax = Recepción IQ de banda ancha por DAX, transmisión de audio que modula la radio. Pulsa «Aplicar / reconectar» para cambiar sin reiniciar.
settings-radio-not-yet-verified-against-real-2 = Aún no verificado con hardware real. Si se porta mal, envía el informe de diagnóstico — contiene cada línea de protocolo intercambiada con la radio.
settings-radio-devices-soapysdr-can-see = Dispositivos que ve SoapySDR
settings-radio-waiting-machine-radio-attached-where = Esperando a la máquina a la que está conectada la radio, donde están instalados los módulos. El dispositivo que abre es `device_args` en el config.toml de esa máquina, o su --device.
settings-radio-ask-every-installed-soapysdr-module = Pide a cada módulo de SoapySDR instalado que busque. No se abre nada, así que es seguro mientras se recibe — pero puede tardar un momento.
settings-radio-not-enumerated-yet-press-rescan = Aún no enumerado — pulsa Volver a buscar.
settings-radio-no-soapysdr-devices-found-check = No se han encontrado dispositivos SoapySDR. Comprueba que el módulo de tu radio está instalado y que puedes acceder al dispositivo.
settings-radio-not-radio = no es una radio
settings-radio-sound-card-listed-above-if = Arriba aparece una tarjeta de sonido como si fuera un SDR. Acepta cualquier frecuencia y la ignora, así que abrirla muestra la entrada de la tarjeta de sonido en lugar de la banda. sdroxide no las elige automáticamente — pero una línea device_args que nombre una sigue obedeciéndose.
settings-radio-which-one-opens-set-device = Cuál se abre lo fija --device o device_args en config.toml; con ninguno, gana la primera radio listada.
settings-radio-no-device = sin dispositivo
settings-radio-rig-has-no-software-adjustable = Este equipo no tiene ganancias ajustables por software.
settings-radio-rx-gains = Ganancias RX
settings-radio-tx-gains = Ganancias TX
settings-radio-antennas = Antenas
settings-radio-remembered-next-start = Se recuerda para el próximo inicio.
settings-radio-these-controls-appear-once-soapysdr = Estos controles aparecen cuando hay un dispositivo SoapySDR abierto — son los propios del dispositivo, y hay que preguntárselos.
settings-radio-stream = Flujo
settings-radio-follow-app-setting = Seguir el ajuste de la aplicación
settings-radio-msps = o, en Msps
settings-radio-baseband-filter = Filtro de banda base
settings-radio-let-driver-choose = Dejar que elija el controlador
settings-radio-changing-either-reopens-radio = Cambiar cualquiera de los dos reabre la radio.
settings-radio-driver-publishes-no-settings-own = Este controlador no publica ajustes propios.
settings-radio-applied-once-remembered-next-start = Se aplica al instante y se recuerda para el próximo inicio.
settings-radio-no-rx-888-found-press = No se ha encontrado ningún RX-888 — pulsa Volver a buscar
settings-radio-adc-clock = Reloj del ADC
settings-radio-t-129-6-msps-needs-superspeed = 129,6 Msps necesita un enlace SuperSpeed y un host rápido; 64,8 es el valor seguro predeterminado. Cambiarlo reabre el receptor automáticamente, lo que tarda un momento pero no requiere reiniciar.
settings-radio-panadapter-width = Ancho del panadaptador
settings-radio-hf-only = {"  "}· solo HF
settings-radio-usable-both-front-ends = Utilizable en ambas etapas de entrada.
settings-radio-below-vhf-crossover-ordinary-width = Por debajo del cruce de VHF este es un ancho normal. Por encima, la FI de 8 MHz del sintonizador no puede centrarse en una ventana tan ancha, así que el espectro en vivo queda a un lado del panadaptador sin nada a su lado.
settings-radio-dial-above-crossover-now-so = El dial está ahora por encima del cruce, así que este ancho muestra los 8 MHz del sintonizador descentrados con espectro muerto a su lado. Estrecha el ancho para llenar el panadaptador.
settings-radio-vga-gain = Ganancia VGA
settings-radio-ad8370-variable-gain-amplifier-ahead = Amplificador de ganancia variable AD8370 delante del ADC.
settings-radio-attenuator = Atenuador
settings-radio-pe4304-step-attenuator-0-5 = Atenuador por pasos PE4304, en pasos de 0,5 dB.
settings-radio-adc-range = Rango del ADC
settings-radio-wide-2-25-vp-p = Ancho (2,25 Vp-p)
settings-radio-selects-adc-s-wider-input = Selecciona el rango de entrada más ancho del ADC: más margen para señales de radiodifusión fuertes, menos cuentas para las débiles. Desactivado selecciona el rango más sensible de 1,5 Vp-p.
settings-radio-dither = Dither
settings-radio-adds-small-dither-signal-ahead = Añade una pequeña señal de dither delante del ADC: cuesta un poco de suelo de ruido y compra rango dinámico libre de espurias.
settings-radio-randomiser = Aleatorizador
settings-radio-adc-scrambles-output-so-digital = El ADC embrolla su salida para que el bus digital deje de radiar sobre la etapa de entrada; el controlador la desembrolla. Déjalo activado salvo que estés depurando. Se aplica al reconectar.
settings-radio-dc-hf-antenna-port = CC en el puerto de antena de HF
settings-radio-powers-active-antenna-preamp-down = Alimenta una antena activa o un preamplificador por el coaxial.
settings-radio-vhf-tuner-gain = Ganancia del sintonizador de VHF
settings-radio-r828d-rf-gain-used-above = Ganancia de RF del R828D, usada por encima del cruce automático HF/VHF. 29 pasos discretos; se usa el más cercano.
settings-radio-auto = Auto
settings-radio-let-tuner-run-own-lna = Dejar que el sintonizador maneje sus propios lazos de LNA y mezclador.
settings-radio-bias-tee-vhf = Bias tee (VHF)
settings-radio-dc-vhf-antenna-port = CC en el puerto de antena de VHF
settings-radio-clock-trim = Ajuste del reloj
settings-radio-corrects-reference-oscillator-applied-when = Corrige el oscilador de referencia. Se aplica cuando sueltas el valor — reabrir en cada píxel de un arrastre reiniciaría el receptor cientos de veces.
settings-radio-receive-only-below-adc-s = Solo recepción. Por debajo del límite de Nyquist del ADC la antena se muestrea directamente y la resintonía es instantánea, porque no hay conversor descendente por hardware — todo el flujo del ADC se convierte a banda base en el host. Por encima, el receptor cambia a su sintonizador R828D y al SMA de VHF automáticamente, así que ambas antenas tienen que estar conectadas. VHF necesita un reloj del ADC de 32,4 Msps o más para que quepa la FI del sintonizador, y con relojes por debajo de 48 Msps hay un hueco entre los dos rangos al que nada puede llegar. Todos los ajustes de aquí se aplican al momento — no hay botón Aplicar que pulsar.
settings-radio-device = Dispositivo
settings-radio-no-devices-press-rescan = sin dispositivos — pulsa Volver a buscar
settings-radio-six-rates-are-six-different =
    Las seis frecuencias son seis imágenes de FPGA distintas, no un registro — por eso nada en el protocolo de ELAD elige entre ellas, y por eso lo que significa este ajuste depende del modelo.

    FDM-S1 / FDM-S2: un comando. Su FPGA la carga el ordenador en cada encendido, y sdroxide ejecuta el cargador elad-firmware de ELAD para poner la imagen de esta frecuencia. Sin ese cargador instalado un muestreador no envía ninguna muestra, y sdroxide lo dice en pantalla.

    FDM-DUO: no es un comando. La radio arranca su propia FPGA y llega a la frecuencia en que se encendió o en la que la dejó FDM-SW2 por última vez (192 kHz en una nueva), y esto solo dice cómo se LEE el flujo. Si te equivocas, el panadaptador simplemente tiene el ancho equivocado, con cada frecuencia dentro de él escalada en consecuencia.

    La frecuencia real del flujo se mide un par de segundos después de que empiece, y una discrepancia se informa en pantalla. Surte efecto con Aplicar.
settings-radio-fdm-s-fpga-image-loaded = FDM-S: la imagen de FPGA cargada — FDM-DUO: cómo se lee el flujo
settings-radio-receiver-s-input-pad-one = El atenuador de entrada del receptor — un paso, dentro o fuera. El mismo control que el deslizador de Ganancia de la ventana principal.
settings-radio-pre-selection-filters = Filtros de preselección
settings-radio-low-pass-bank-front-adc = El banco de filtros paso bajo delante del ADC. Puentearlo da la vista más ancha y el peor comportamiento cerca de señales fuertes fuera de banda — un transmisor de radiodifusión a unos kilómetros pondrá imágenes por toda la banda. Déjalo activado salvo que escuches deliberadamente fuera del rango filtrado.
settings-radio-filters-circuit = Filtros en el circuito
settings-radio-rig-control-fdm-duo-only = Control del equipo — solo FDM-DUO
settings-radio-transceiver-s-cat-usb-port = El puerto USB CAT del transceptor, que es un cable aparte del de recepción. Deja el puerto sin fijar en un FDM-S1 o FDM-S2, que no lo tienen — y en un FDM-DUO que prefieras manejar solo con su cable de recepción, donde aún se puede sintonizar y activar, pero no se puede leer ningún medidor.
settings-radio-port-set-radio-s-display = Con el puerto fijado, la pantalla de la radio y este dial son el mismo número en ambos sentidos: la ventana de recepción del FDM-DUO se construye alrededor de su VFO, así que sintonizar aquí mueve el panel frontal y girar el mando del panel frontal mueve la lectura de aquí. El panadaptador se recentra sobre el dial sobre la marcha. Sin puerto — o sin uno en el que la radio responda — no hay VFO que ordenar, y la sintonía se queda dentro de la ventana que la radio ya envía.
settings-radio-none-usb-control-only = — ninguno (solo control USB) —
settings-radio-must-match-menu-70-cat = Debe coincidir con el menú 70 «CAT BAUD» de la radio, que sale de fábrica en 38400. La radio tiene estas cuatro velocidades y ninguna otra, y un puerto abierto a cualquier otra es mudo en ambos sentidos — la radio ignora el dial y se niega a activarse, sin nada que diga por qué.
settings-radio-how-transmit-keyed-cat-sends = Cómo se activa la transmisión. «CAT» envía el comando TX propio de la radio y no necesita nada configurado en el equipo; RTS necesita el menú 54 «PTT» en PTT+RTS.
settings-radio-where-radio-takes-transmit-audio-2 =
    De dónde toma la radio su audio de transmisión — el comando TI del equipo, que es el menú 32 «TX IN» del panel frontal. Se establece al abrir el puerto.

    «Audio USB» es lo que hace funcionar la transmisión aquí: el FDM-DUO envía lo que sdroxide pone en su tarjeta de sonido USB. Una radio dejada en «Micrófono» envía en cambio la sala, y nada en pantalla lo dice.
settings-radio-which-two-sockets-back-receiver-2 =
    En cuál de los dos conectores de la parte trasera escucha el receptor — el comando AN del equipo, que es el menú 31 «ANTENNAS» del panel frontal y el indicador «ANT 1 2» de su pantalla.

    «RTX» es una antena que hace los dos trabajos, en el conector tipo M que también lleva la transmisión. «Solo RX» mueve la recepción al segundo conector y deja la transmisión en RTX — una antena de recepción, un lazo o una beverage, con el haz todavía en el transmisor.

    Mueve todo este receptor: el panadaptador, los demoduladores y el propio audio del equipo vienen todos del conector seleccionado aquí.

    Se aplica al instante y se lee de vuelta de la radio al abrir el puerto de control — es un ajuste del propio equipo, no una copia guardada aquí.
settings-radio-how-often-radio-asked-what-2 = Con qué frecuencia se pregunta a la radio qué está haciendo — su dial, su modo y sus medidores. Gira el mando VFO del propio equipo y la lectura sigue en un sondeo. También es todo el tráfico de control que genera este extremo, así que bájalo si el audio de la radio se entrecorta.
settings-radio-transmit-audio = Audio de transmisión
settings-radio-system-default = — predeterminado del sistema —
settings-radio-fdm-duo-s-own-usb = El puerto USB Audio propio del FDM-DUO. Dejado en el predeterminado del sistema, casi nunca es la radio — son los altavoces de la máquina.
settings-radio-press-rescan-list-sound-cards = pulsa Volver a buscar para listar las tarjetas de sonido
settings-radio-copies-last-session-s-trace = Copia al portapapeles la traza de la última sesión, para un informe de error: cada comando intercambiado con el dispositivo y los primeros bytes del flujo de muestras.
settings-radio-only-receiving-fdm-duo-has = Hasta ahora solo se ha ejecutado en hardware real la recepción en un FDM-DUO. Si algo se porta mal, adjunta el informe de diagnóstico a un informe de error.
settings-radio-cw-keyed-radio-s-own = El CW se activa con la propia llave o pala de la radio: el FDM-DUO no tiene ningún comando que acepte texto, así que el panel de CW no puede activarlo.
settings-radio-press-apply-reopen-radio = Pulsa Aplicar para reabrir la radio.
settings-radio-no-receivers-press-rescan = sin receptores — pulsa Volver a buscar
settings-radio-which-rates-receiver-has-depends = Qué frecuencias tiene un receptor depende del modelo y del firmware. Surte efecto con Aplicar.
settings-radio-every-rate-any-hf-model = todas las frecuencias que ofrece cualquier modelo HF+ — conecta uno para ver las suyas
settings-radio-receiver-s-own-gain-control = El control de ganancia propio del receptor. Déjalo activado para la escucha general; desactívalo para ajustar el atenuador a mano para medir.
settings-radio-agc-threshold = Umbral del AGC
settings-radio-high = Alto
settings-radio-high-trades-little-sensitivity-headroom = Alto cambia un poco de sensibilidad por margen frente a vecinos fuertes — el ajuste correcto en una banda concurrida por la noche.
settings-radio-front-end-attenuation-gain-0 = Atenuación de la etapa de entrada, como ganancia — 0 dB es ninguna. Solo se obedece con el AGC desactivado.
settings-radio-preamp = Preamplificador
settings-radio-hf-low-noise-amplifier-buys = El amplificador de bajo ruido de HF. Compra sensibilidad a costa de intermodulación, así que está desactivado de forma predeterminada — lo que suele ser correcto en una antena real.
settings-radio-lna = LNA activado
settings-radio-frequency-calibration = Calibración de frecuencia
settings-radio-parts-per-billion-receiver-s = Partes por mil millones — la unidad propia de este receptor, mil veces más fina que los ppm que usa un RTL-SDR. Nada de aquí se escribe nunca en la flash del receptor: esto anula el valor almacenado solo durante la sesión.
settings-radio-use-receiver-s-stored-value = Usar el valor almacenado del receptor
settings-radio-ppb = {" "}ppb
settings-radio-feed-dc-up-coax = Alimentar CC por el coaxial
settings-radio-not-every-hf-has-one = No todos los HF+ lo tienen; en un receptor sin él, esto no hace nada.
settings-radio-host-dsp = DSP en el host
settings-radio-image-balancer-zero-if-offset = El equilibrador de imagen, el desplazamiento de FI cero y el oscilador de sintonía fina. Desactívalo solo para ver la salida cruda del hardware — con él desactivado, la imagen especular aparece en las frecuencias de FI cero y el dial solo es exacto al kilohercio más cercano.
settings-radio-correct-image-fine-tune = Corregir la imagen y afinar
settings-radio-copies-last-session-s-trace-2 = Copia al portapapeles la traza de la última sesión, para un informe de error: cada comando intercambiado con el receptor y los primeros bytes del flujo de muestras.
settings-radio-receive-only-no-soapysdr-no = Solo recepción. No hacen falta ni SoapySDR ni libairspyhf. Por debajo del suelo del sintetizador el oscilador del host hace la sintonía, que es como este receptor llega a VLF. El receptor y la frecuencia de muestreo surten efecto con Aplicar; todo lo demás se aplica a medida que lo cambias.
settings-radio-not-yet-verified-against-real-3 = Aún no verificado con hardware real. Si se porta mal, envía el informe de diagnóstico — contiene cada comando intercambiado con el receptor y los primeros bytes del flujo de muestras decodificados como pares I/Q.
settings-radio-no-airspy-r2-mini-found = No se ha encontrado ningún Airspy R2 ni Mini — pulsa Volver a buscar
settings-radio-read-receiver = leído de este receptor
settings-radio-r2-mini-offer-different-rates = un R2 y un Mini ofrecen frecuencias distintas y no se pueden distinguir hasta que uno está abierto
settings-radio-rate-you-get-receiver-s = Esta es la frecuencia que obtienes. El ADC del receptor funciona al doble — digitaliza una señal real y sdroxide hace banda base compleja a partir de ella en el host.
settings-radio-gain-curve = Curva de ganancia
settings-radio-step-along-curve-above-not = Un paso a lo largo de la curva de arriba, no una cifra en dB — el LNA, el mezclador y el VGA del sintonizador se mueven juntos, y lo que vale cada paso depende de la curva y de la banda. 0 es el extremo silencioso.
settings-radio-tuner-agc = AGC del sintonizador
settings-radio-lna-2 = LNA
settings-radio-hands-lna-tuner-s-own = Entrega el LNA al lazo propio del sintonizador, anulando la curva.
settings-radio-mixer = Mezclador
settings-radio-same-mixer-stage = Lo mismo para la etapa del mezclador.
settings-radio-loop-running-gain-slider-no = Con un lazo en marcha, el deslizador de ganancia ya no fija la etapa que el lazo posee — el lazo la sobrescribe.
settings-radio-dc-antenna-port = CC en el puerto de antena
settings-radio-t-12-bit-packing = Empaquetado de 12 bits
settings-radio-third-less-usb-traffic-leave = Un tercio menos de tráfico USB. Déjalo activado: es un dispositivo USB 2.0 y la frecuencia máxima son 30 MB/s empaquetados frente a 40 sin empaquetar. Se aplica al reconectar.
settings-radio-dc-removal = Eliminación de continua
settings-radio-remove-adc-s-offset = Quitar el desplazamiento del ADC
settings-radio-turn-off-see-raw-hardware = Desactívalo para ver la salida cruda del hardware. Conviene saber adónde va la espuria: el desplazamiento cae en el borde del ancho y no en su centro, porque la señal se traslada un cuarto de la frecuencia de muestreo por el camino.
settings-radio-every-command-exchanged-receiver-sample = Cada comando intercambiado con el receptor, la aritmética de la frecuencia de muestreo y las primeras muestras decodificadas como I/Q.
settings-radio-receive-only-24-1800-mhz = Solo recepción, 24–1800 MHz. No hacen falta ni SoapySDR ni libairspy. El receptor, la frecuencia de muestreo y el empaquetado surten efecto con Aplicar; todo lo demás se aplica a medida que lo cambias.
settings-radio-not-yet-verified-against-real-4 = Aún no verificado con hardware real. Si se porta mal, envía el informe de diagnóstico — contiene cada comando intercambiado con el receptor, la aritmética de las frecuencias y las primeras muestras decodificadas como pares I/Q.
settings-radio-no-hydrasdr-rfone-found-press = No se ha encontrado ningún HydraSDR RFOne — pulsa Volver a buscar
settings-radio-these-are-rates-receiver-turned = estas son las frecuencias que resultó tener este receptor
settings-radio-receiver-only-reports-three-these = el receptor solo informa de tres de estas; el resto están en la tabla alternativa del firmware y una compilación más antigua puede no incluirlas
settings-radio-rf-input = Entrada de RF
settings-radio-antenna-sma-only-socket-bias = El SMA de antena — el único conector con el bias tee detrás.
settings-radio-cable-socket-no-bias-tee = Un conector de cable. Aquí no hay bias tee; el hardware lo pone solo en ANT.
settings-radio-powers-active-antenna-preamp-down-2 = Alimenta una antena activa o un preamplificador por el coaxial. Solo el conector ANT tiene uno — los dos puertos de cable son entradas normales.
settings-radio-third-less-usb-traffic-leave-2 = Un tercio menos de tráfico USB. Déjalo activado: es un dispositivo USB 2.0 y la frecuencia máxima son 36 MB/s empaquetados frente a 48 sin empaquetar. Se aplica al reconectar.
settings-radio-one-these-boards-1d50-60a1 = Una de estas placas está en 1d50:60a1, el ID USB que los prototipos de HydraSDR comparten con el Airspy R2 y el Mini. sdroxide comprueba el firmware tras abrir y lo dirá si se ha elegido la interfaz equivocada, en cualquiera de los dos sentidos.
settings-radio-receive-only-24-1800-mhz-2 = Solo recepción, 24–1800 MHz. No hacen falta ni SoapySDR ni libhydrasdr. El receptor, la frecuencia de muestreo y el empaquetado surten efecto con Aplicar; todo lo demás se aplica a medida que lo cambias.
settings-radio-re-list-fobos-sdrs-no = Vuelve a listar los Fobos SDR. No se abre ningún dispositivo, así que es seguro pulsarlo mientras se recibe.
settings-radio-no-fobos-sdr-found-press = No se ha encontrado ningún Fobos SDR — pulsa Volver a buscar
settings-radio-input-2 = Entrada
settings-radio-through-tuner-lna-vga-mixer = A través del sintonizador: LNA, VGA y el rango de frecuencias propio del mezclador.
settings-radio-direct-sampling-hf1-antenna-input = Muestreo directo en la entrada de antena HF1 — sin sintonizador, sin ganancia LNA/VGA, sintonizado enteramente por software.
settings-radio-direct-sampling-hf2-antenna-input = Muestreo directo en la entrada de antena HF2, igual que HF1.
settings-radio-both-real-adc-channels-once = Ambos canales reales del ADC a la vez, combinados por el filtro de diversidad de abajo — una fuente de ruido anulada, o dos trayectos con desvanecimiento combinados.
settings-radio-direct-sampling-tuner-so-lna = Muestreo directo — el sintonizador (y por tanto la ganancia LNA/VGA) no está en el camino. La frecuencia de abajo es un objetivo para el conversor descendente por software, no un ajuste de hardware, así que la frecuencia lograda puede diferir.
settings-radio-receiver-s-own-reported-rates = las frecuencias que informa este receptor
settings-radio-measured-one-real-unit-connect = medido en una unidad real — conecta este receptor y pulsa Volver a buscar para ver su propia lista
settings-radio-clock-source = Fuente de reloj
settings-radio-external-reference = Referencia externa
settings-radio-diversity-taps = Tomas de diversidad
settings-radio-taps = {" "}tomas
settings-radio-one-tap-gain-phase-null = Una toma es una ganancia y una fase — un nulo a una frecuencia que empeora a cada lado, que es todo lo que puede hacer un fasador analógico. Cada toma adicional compra un periodo de muestra de la diferencia de trayecto entre HF1 y HF2 que el filtro puede ecualizar, que es lo que convierte ese notch en una banda silenciosa de lado a lado.
settings-radio-receive-only-rf-port-tuner = Solo recepción. Puerto RF (sintonizador), HF1/HF2 (muestreo directo, decodificado por software) y HF1+HF2 (ambos a la vez, combinados por el filtro de diversidad) funcionan todos. El receptor, la entrada y la frecuencia de muestreo surten efecto con Aplicar; la ganancia, la fuente de reloj y las tomas se aplican a medida que las cambias — el modo, la velocidad de adaptación y la retención del filtro de diversidad están en el propio recuadro DIV de la ventana principal una vez seleccionada esta radio.
settings-radio-no-hackrf-found-press-rescan = No se ha encontrado ningún HackRF — pulsa Volver a buscar
settings-radio-front-end-amplifier-8-db = Amplificador de la etapa de entrada, en pasos de 8 dB. Es la etapa que cambia la sensibilidad — y la que se sobrecarga primero en una antena real.
settings-radio-baseband-amplifier-after-mixer-2 = Amplificador de banda base tras el mezclador, en pasos de 2 dB. Súbelo para una señal débil antes de recurrir al LNA.
settings-radio-rf-amp = Amp. RF
settings-radio-one-switch-shared-transmit-setting = Un interruptor, compartido con el ajuste de transmisión de abajo — la radio aplica el que pertenece a la dirección en que entra. Desactivado suele ser lo correcto en una antena real.
settings-radio-hackrf-pro-chooses-itself-three = Un HackRF Pro lo elige él mismo — tres cuartas partes de la frecuencia de muestreo, filtrado en la FPGA — e ignora lo que pida el host.
settings-radio-leave-automatic-choosing-one-too = Déjalo en Automático. Elegir uno demasiado estrecho no solo suaviza los bordes de la banda — retira el desplazamiento del LO que mantiene el pico de continua lejos de tu señal.
settings-radio-about-3-v-50-ma = Unos 3 V a 50 mA por el coaxial, para una antena activa o un preamplificador. Solo en HackRF One o Pro; el Jawbreaker y el rad1o no tienen tal circuito.
settings-radio-remove-dc-mirror-image = Quitar la continua y la imagen especular
settings-radio-zero-if-radio-so-own = Esta es una radio de FI cero, así que su propia fuga de LO queda a mitad del ancho y el error de cuadratura del mezclador pone una imagen especular a su través. Desactivarlo muestra la salida cruda del hardware, que es la forma rápida de distinguir un problema de controlador de uno de DSP.
settings-radio-corrects-reference-oscillator = Corrige el oscilador de referencia.
settings-radio-transmit = Transmisión
settings-radio-transmitter = Transmisor
settings-radio-off-default-while-off-backend = Desactivado de forma predeterminada. Mientras esté desactivado el motor no publica ningún canal de transmisión, así que nada puede activar la radio. Se aplica al reconectar.
settings-radio-tx-vga-gain = Ganancia VGA de TX
settings-radio-transmit-driver-amplifier-drive-applied = El amplificador excitador de transmisión. La excitación se aplica digitalmente antes de esta etapa, así que deja la excitación alta y fija aquí el nivel de salida — bajar la excitación hace funcionar el DAC a una fracción del fondo de escala y aumenta la intermodulación.
settings-radio-same-switch-receive-setting-above = El mismo interruptor que el ajuste de recepción de arriba, aplicado cuando la radio cambia de dirección.
settings-radio-transmit-armed-into-dummy-load = La transmisión está armada. Sobre una carga ficticia hasta que la hayas medido: los armónicos de un HackRF son lo bastante fuertes como para necesitar un filtro paso bajo externo para la banda en que estés, y es semidúplex, así que la recepción se detiene mientras dura cada turno.
settings-radio-every-command-exchanged-radio-session = Cada comando intercambiado con la radio durante esta sesión, en orden, incluida la secuencia alrededor de cada pulsación.
settings-radio-dc-7-25-ghz-half = CC – 7,25 GHz, semidúplex. No hacen falta ni SoapySDR, ni libusb, ni libhackrf. La radio y la frecuencia de muestreo surten efecto con Aplicar; todo lo demás se aplica a medida que lo cambias. Los límites de banda son los del firmware: un HackRF está especificado para 1 MHz – 6 GHz (100 kHz en un Pro) y se atenúa mucho fuera de eso, pero sí sintoniza ahí.
settings-radio-ask-sdrplay-api-service-device = Pide al servicio de la API de SDRplay su lista de dispositivos. No se abre nada, así que es seguro pulsarlo mientras se recibe.
settings-radio-no-rsps-press-rescan-needs = sin RSP — pulsa Volver a buscar (necesita el servicio de la API de SDRplay)
settings-radio-both-tuners-running-api-fixes = Con los dos sintonizadores en marcha, la API fija el ADC en 6 MHz y devuelve 2 Msps desde una FI baja — así que 2 Msps es el ancho más amplio, y los más estrechos son eso diezmado. Surte efecto con Aplicar.
settings-radio-rates-below-2-msps-run = Las frecuencias inferiores a 2 Msps hacen funcionar el ADC a 2 Msps y diezman en el servicio. Surte efecto con Aplicar.
settings-radio-reduced-adc-resolution = {"  "}(resolución del ADC reducida)
settings-radio-if-bandwidth = Ancho de banda de FI
settings-radio-tuner-s-analog-filter-auto = El filtro analógico del sintonizador. Auto elige el más ancho que cabe en la frecuencia de muestreo. Surte efecto con Aplicar.
settings-radio-rsp-s-own-if-gain = El lazo de ganancia de FI propio del RSP, manejado por el servicio de la API. Desactivado te devuelve el deslizador de ganancia de FI — el ajuste para medir y para los modos digitales de señal débil.
settings-radio-agc-set-point = Punto de consigna del AGC
settings-radio-signal-level-loop-holds-adc = Nivel de señal al que el lazo mantiene el ADC. Más bajo deja más margen para señales fuera de canal. Cuán bajo puede llegar depende de la frecuencia de muestreo — por encima de 8,064 Msps el conversor tiene menos margen que dar, y por encima de 9,216 Msps menos aún.
settings-radio-rsp-s-native-gain-unit = La unidad de ganancia nativa del RSP: 20 dB es la ganancia máxima (0 con el rango extendido de abajo), 59 dB la mínima. Se aplica al instante. Se ignora mientras el AGC está en marcha — entonces el lazo posee este valor, y el S-metro muestra el que ha alcanzado.
settings-radio-extended-if-range = Rango de FI extendido
settings-radio-lets-if-gain-reduction-go = Deja que la reducción de ganancia de FI baje de 20 dB, hasta 0 — los últimos 20 dB de ganancia que tiene el receptor. Desactivado es el valor predeterminado de la propia API y el correcto para la escucha normal, porque el extremo inferior del rango es donde un RSP se sobrecarga con más facilidad. Conviene activarlo para señales débiles con el LNA ya en 0. También permite que el punto de consigna del AGC suba hasta 0 dBFS. Se aplica al instante.
settings-radio-allow-if-gain-reduction-below = Permitir reducción de ganancia de FI por debajo de 20 dB
settings-radio-front-end-attenuation-steps-0 = Atenuación de la etapa de entrada en pasos: 0 es la ganancia máxima, cada paso hacia arriba mete más atenuación. Algunas bandas tienen menos pasos — el controlador recorta y conserva tu elección para cuando vuelvas a sintonizar, y el carril de aquí termina en lo que admita la banda en que esté la radio. Una elección más alta hecha en otra banda se conserva y se sigue mostrando a su lado. Se aplica al instante.
settings-radio-t-0-max-gain = 0 = ganancia máx.
settings-radio-reference-error-parts-per-million-2 = Error de referencia en partes por millón, aplicado por el propio dispositivo. Se aplica al instante.
settings-radio-radio-s-tuner = Sintonizador de esta radio
settings-radio-tuner = Sintonizador
settings-radio-which-rspduo-s-two-tuners = En cuál de los dos sintonizadores del RSPduo escucha esta radio. El otro pertenece a la radio configurada para él — dale a esa el mismo receptor y el otro sintonizador. Surte efecto con Aplicar.
settings-radio-which-rspduo-s-two-tuners-2 = Cuál de los dos sintonizadores del RSPduo lleva la antena que estás escuchando. El otro lleva la segunda antena. Surte efecto con Aplicar.
settings-radio-which-rspduo-s-two-tuners-3 = Cuál de los dos sintonizadores del RSPduo hacer funcionar (de uno en uno). Surte efecto con Aplicar.
settings-radio-applies-immediately = Se aplica al instante.
settings-radio-fm-broadcast-notch = Notch de radiodifusión FM
settings-radio-hardware-notch-over-88-108 = Notch por hardware sobre la banda de radiodifusión de 88–108 MHz, para cuando un transmisor local sobrecarga todo lo demás. Se aplica al instante.
settings-radio-dab-notch = Notch de DAB
settings-radio-hardware-notch-over-165-230 = Notch por hardware sobre la banda DAB de 165–230 MHz. Se aplica al instante.
settings-radio-hdr-mode = Modo HDR
settings-radio-enable-below-2-mhz = Activar por debajo de 2 MHz
settings-radio-rspdx-s-high-dynamic-range =
    El camino de alto rango dinámico del RSPdx para LF/MF. Se aplica al instante.

    Todavía no funciona: en el único RSPdx con el que se ha probado, activarlo silencia el receptor — en todos los centros, filtros, puertos, ganancias y frecuencias probados. Lo que necesite el modo no está en la superficie documentada de la API, y encontrarlo es la próxima tarea de alguien. Déjalo desactivado salvo que seas quien lo está buscando.

    Lo que se sabe: no es un modo que siga al dial. El camino tiene un filtro analógico fijo, construido solo para los centros listados bajo el filtro de abajo, así que sintonizar en cualquier otra parte no podría funcionar ni siquiera cuando funcione el resto.
settings-radio-hdr-filter = Filtro HDR
settings-radio-feed-4-7-v-dc = Alimentar ~4,7 V CC por el coaxial
settings-radio-second-tuner = Segundo sintonizador
settings-radio-rspduo-two-whole-tuners-one = Un RSPduo son dos sintonizadores completos en una placa, con reloj de una única referencia. Si ejecutas ambos oyen sus anchos en el mismo instante desde el mismo reloj — lo que hace coherentes dos antenas en ellos, con una fase relativa fijada por las líneas de alimentación y no por azar, y permite combinar el par. O déjalos separados: los sintonizadores sintonizan por separado, así que el otro puede ser una segunda radio en su propia banda.
settings-radio-run-both-tuners = Ejecutar ambos sintonizadores
settings-radio-puts-rspduo-api-s-dual = Pone el RSPduo en el modo de doble sintonizador de la API, donde el ADC está fijado a 6 MHz y el ancho más amplio es 2 Msps. Surte efecto con Aplicar, porque el modo se elige cuando se abre el dispositivo.
settings-radio-used = Usado para
settings-radio-what-other-tuner-takes-effect = Para qué sirve el otro sintonizador. Surte efecto con Aplicar — igual que ejecutar ambos, esto se elige cuando se abre la placa.
settings-radio-add-second-radio-settings-radio = Añade una segunda radio (Ajustes → Radio → +), dale este mismo receptor y el otro sintonizador del RSPduo, y ajústala para que también ejecute ambos sintonizadores — la radio que abra la placa la pone en modo de doble sintonizador, así que las dos tienen que esperarlo. Entonces las dos sintonizan de forma independiente: HF en una pestaña y VHF en la otra, desde una sola placa. Comparten un reloj de ADC, así que ambas funcionan a la frecuencia de muestreo que pidió la radio que abrió la placa, y ninguna puede transmitir.
settings-radio-what-do = Qué hacer con él
settings-radio-lna-state-2 = Su estado de LNA
settings-radio-second-tuner-s-own-front = La atenuación propia de la etapa de entrada del segundo sintonizador. Fíjala para que ambas antenas muestren aproximadamente el mismo suelo de ruido: este es el ajuste en el que descansa todo lo demás, porque la combinación pondera las dos ramas por su ruido, y una segunda etapa de entrada llevada a sobrecarga entrega al filtro una copia distorsionada de la interferencia — que no puede restarse de una sin distorsionar. Se aplica al instante.
settings-radio-if-gain-reduction-2 = Su reducción de ganancia de FI
settings-radio-second-tuner-s-if-gain = La ganancia de FI del segundo sintonizador, en la unidad nativa del RSP: 20 dB es la ganancia máxima. Se ignora mientras el AGC está en marcha — y una ganancia estable es lo que quiere el filtro, así que desactivar el AGC merece la pena para un nulo que piensas mantener.
settings-radio-filter-length = Longitud del filtro
settings-radio-one-tap-gain-phase-null-2 = Una toma es una ganancia y una fase — un nulo a una frecuencia que empeora a cada lado, que es todo lo que puede hacer un fasador analógico. Cada toma adicional compra un periodo de muestra de la diferencia de trayecto entre las dos antenas que el filtro puede ecualizar, que es lo que convierte ese notch en una banda silenciosa de lado a lado.
settings-radio-adaptation-2 = Adaptación
settings-radio-slow-steady-left-converging-inside = Lento y constante a la izquierda, convergiendo en una fracción de segundo y oscilando visiblemente a la derecha. Empieza rápido para encontrar el nulo y luego manténlo.
settings-radio-stop-filter-moving-reach-moment = Detiene el movimiento del filtro. Recurre a esto en cuanto aparezca un nulo: un filtro que se deja adaptando se volverá a apuntar a lo que se vuelva más fuerte, que en una banda tranquila es la estación que estás escuchando.
settings-radio-restart = Reiniciar
settings-radio-zero-filter-find-null-again = Pone el filtro a cero y vuelve a buscar el nulo.
settings-radio-nothing-here-can-tell-wanted = Nada de aquí puede distinguir una señal deseada de una no deseada — el filtro solo sabe lo que tienen en común las dos antenas. En Cancelar, la segunda antena quiere oír la fuente de ruido y la menor cantidad posible de la banda, o cancelará diligentemente también la estación. En Combinar, ambas quieren oír la misma estación. La profundidad que va alcanzando el nulo se escribe en el registro cada pocos segundos.
settings-radio-not-yet-verified-against-rspduo = Aún no verificado con un RSPduo: el funcionamiento con doble sintonizador de aquí está escrito a partir de la API de SDRplay y no medido en el hardware.
settings-radio-no-receiver-listed-check-against = No hay ningún receptor listado con el que comprobarlo — pulsa Volver a buscar, o lee el registro para ver qué se abrió realmente.
settings-radio-receive-only-1-khz-2 = Solo recepción, 1 kHz–2 GHz. Necesita el servicio de la API de SDRplay del fabricante (sdrplay.com/api) — los RSP posteriores al RSP1 original no tienen protocolo abierto. El dispositivo, la frecuencia de muestreo, el ancho de banda y el sintonizador del RSPduo surten efecto con Aplicar; todo lo demás se aplica a medida que lo cambias.
settings-radio-board = Placa
settings-radio-first-one-found-2 = La primera que se encuentre
settings-radio-receive-chain = Cadena de recepción
settings-radio-both-chains-tune-together-they = Las dos cadenas sintonizan juntas — comparten un sintetizador — pero son etapas de entrada separadas en conectores separados. Elige aquella en la que está tu antena, que es el ajuste al que recurrir cuando una cadena ha recibido la modificación de adaptación a HF y la otra es de serie.
settings-radio-receive-gain = Ganancia de recepción
settings-radio-one-combined-figure-which-limesuite = Una cifra combinada, que LimeSuite reparte entre el LNA, el TIA y el PGA por sí mismo. Admite decibelios enteros, así que cualquier cosa más fina se trunca.
settings-radio-receive-port = Puerto de recepción
settings-radio-automatic-follows-frequency-lnal-low = Automático sigue la frecuencia: LNAL baja, LNAH alta — salvo que haya un LimeRFE conectado abajo, que es un cable a un conector, y entonces es LNAW en todas las frecuencias. Nombra el conector al que está cableada la tuya si no es ese. A qué cadena pertenece el conector lo indica el selector de arriba.
settings-radio-t-0-follows-sample-rate-worth = 0 sigue la frecuencia de muestreo. Conviene dejarlo ahí: un filtro más estrecho que un cuarto del ancho retira en silencio el desplazamiento del LO de FI cero, lo que devuelve la fuga del LO justo encima de la señal que estás escuchando.
settings-radio-following-rate = siguiendo la frecuencia
settings-radio-corrections = Correcciones
settings-radio-host-iq-dc-correction = Corrección IQ / continua en el host
settings-radio-adaptive-image-dc-removal-side = Eliminación adaptativa de imagen y continua de este lado, además de la propia calibración del chip. Desactivarla es la forma de un clic de distinguir un problema de controlador de uno de DSP.
settings-radio-calibrate-automatically = Calibrar automáticamente
settings-radio-runs-chip-s-own-dc = Ejecuta la calibración propia del chip de desplazamiento de continua e imagen cuando se abre la radio, y de nuevo una vez que el dial se ha asentado en una banda nueva o en un conector distinto. Esos números se miden a una frecuencia y son erróneos en otras, que es lo que suele ser una portadora que queda en el centro del ancho. Cuesta aproximadamente un segundo cada vez.
settings-radio-calibrate-now = Calibrar ahora
settings-radio-stalls-receiver-better-part-second = Detiene el receptor durante la mayor parte de un segundo.
settings-radio-second-aerial = Segunda antena
settings-radio-socket = Su conector
settings-radio-same-first = Igual que la primera
settings-radio-gain-2 = Su ganancia
settings-radio-set-low-coupled-sample-your = Fíjala BAJA. La muestra acoplada de tu propio transmisor es una señal fuerte, y una cadena de realimentación llevada a compresión mide mal la curva del amplificador — enseña a la corrección su propia distorsión. Empieza por abajo y usa el atenuador del acoplador.
settings-radio-set-so-both-aerials-show = Fíjala para que ambas antenas muestren aproximadamente el mismo suelo de ruido. Este es el ajuste en el que descansa todo: la combinación pondera las dos ramas por su ruido, y una segunda cadena llevada a compresión entrega al filtro una copia distorsionada de la interferencia — que no puede restarse de una sin distorsionar.
settings-radio-table-steps-2 = Pasos de la tabla
settings-radio-how-finely-correction-follows-amplifier = Con qué finura sigue la corrección la curva del amplificador. Más pasos siguen un codo más agudo, pero cada uno tiene que aprenderse de las muestras que cayeron en él — y la parte alta del histograma de amplitud de la voz es escasa. Treinta y dos bastan para la curva suave que realmente tiene un amplificador de HF. Cambiarlo reinicia la corrección.
settings-radio-how-hard-each-block-feedback = Con qué fuerza mueve la corrección cada bloque de realimentación. La curva de un amplificador no cambia, así que no hay prisa — el punto medio promedia el ruido de varios turnos.
settings-radio-keep-correction-curve-learned-clean = Mantiene la corrección como está. Una curva aprendida en un turno limpio merece conservarse, y el amplificador no habrá cambiado para el siguiente.
settings-radio-forget-correction-learn-again = Olvida la corrección y la aprende de nuevo.
settings-radio-directional-coupler-amplifier-s-output = Un acoplador direccional en la salida del amplificador entra en esta cadena, y el transmisor compara lo que ha vuelto con lo que pretendía enviar — y luego envía la inversa de la diferencia, de modo que lo que sale del amplificador sale recto. Unos veinte decibelios menos de intermodulación en los QSO de los demás, y el amplificador conserva su potencia. La corrección se queda en la unidad hasta que la realimentación se alinea con la transmisión, así que un acoplador que no está conectado no cuesta nada; y nunca puede pedir al conversor más que el fondo de escala, así que un camino de realimentación que lea tonterías no puede sobreexcitar nada. Cómo va se escribe en el registro mientras transmites.
settings-radio-transmit-not-armed-so-there = La transmisión no está armada, así que aquí no hay nada que corregir.
settings-radio-off-interface-publishes-no-transmit = Con esto desactivado la interfaz no publica ningún canal de transmisión, así que nada puede activar la radio.
settings-radio-transmit-gain = Ganancia de transmisión
settings-radio-transmit-port = Puerto de transmisión
settings-radio-board-s-two-transmit-sockets = Los dos conectores de transmisión de la placa son dos redes de adaptación distintas, no dos jacks hacia la misma: BAND1 (TX_1) lleva de 30 MHz a 1,9 GHz y BAND2 (TX_2) es el puerto de 13 cm. Activar una banda de aficionado por debajo de 23 cm desde BAND2 pone el turno en una red de adaptación que no pasa casi nada — todos los ajustes parecen correctos, la excitación es la que fijaste y el medidor de potencia se queda en cero. Déjalo en Automático y sigue al dial.
settings-radio-limesdr-transmits-about-100-khz = Un LimeSDR transmite desde unos 100 kHz hasta 3,8 GHz sin ningún filtrado propio. Usa un filtro paso bajo, un canal adecuado del LimeRFE o una carga ficticia.
settings-radio-limerfe-front-end = Etapa de entrada LimeRFE
settings-radio-connected = Conectado por
settings-radio-limerfe-s-own-micro-usb = El puerto micro-USB propio del LimeRFE, no el de la radio. 9600 baudios, fijados por su firmware.
settings-radio-receive-connector = Conector de recepción
settings-radio-transmit-connector = Conector de transmisión
settings-radio-follow-dial = Seguir el dial
settings-radio-switch-filters-match-operating-frequency = Conmuta los filtros para que coincidan con la frecuencia de operación, antes de que aparezca RF. Sintonizar dentro de una banda no pone nada en el enlace de control.
settings-radio-relays = Relés
settings-radio-receive-attenuator = Atenuador de recepción
settings-radio-notch-filter = Filtro notch
settings-radio-fan = Ventilador
settings-radio-worth-having-any-sustained-transmitting = Conviene tenerlo activado para cualquier transmisión sostenida.
settings-radio-automatic-board-receives-switched-transmit = En Automático la placa recibe, y se conmuta a transmisión al pulsar y de vuelta al soltar — con cualquiera de los dos cableados. Sus canales de aficionado tienen un filtro con un conmutador de transmisión/recepción a cada lado, así que una placa a la que se pide ambos a la vez pone ese conmutador en el transmisor y deja de oír nada. El canal de 30 MHz solo es alcanzable a través de J5, que es un conector para ambas direcciones; por encima, transmitir desde J4 mantiene el camino de recepción alejado del conector que está manejando el amplificador.
settings-radio-limesdr-support-has-not-been = El soporte de LimeSDR no se ha verificado con hardware. Esta es la traza de la última sesión, para un informe de incidencia.
settings-radio-gains-filters-corrections-antenna-sockets = Las ganancias, los filtros, las correcciones, los conectores de antena y todos los controles del LimeRFE — sus conectores, banda, relés, atenuador, notch y ventilador — se aplican al instante. La placa, la cadena de recepción, la frecuencia de muestreo, armar la transmisión y la conexión del LimeRFE surten efecto con Aplicar.

## Settings window, Radio tab: messages with values

settings-radio-fill-from = RELLENAR DESDE { $name }
settings-radio-decimation-auto = Automático (la más cercana a { $khz } kHz)
settings-radio-decimation-stage-server = Escalón { $stage } — la frecuencia del servidor ÷ { $divisor }
settings-radio-decimation-stage = Escalón { $stage } — frecuencia ÷ { $divisor }
settings-radio-kiwi-receive-only = Solo recepción — esta es la antena de otra persona. El panadaptador son los ~12 kHz que el receptor envía como I/Q, que sigue al dial; la vista de banda sobre él es su propia cascada, y sintonizar a través de ella resintoniza el receptor. Unos { $rate } kB/s mientras está conectado. Pulsa «Aplicar / reconectar» para cambiar sin reiniciar.
settings-radio-connected-detail = Conectado: { $detail }
settings-radio-failed = Error: { $error }
settings-radio-pluto-buffer = ≈ { $ms } ms, { $kib } KiB por transferencia
settings-radio-driver-settings = Ajustes de { $driver }
settings-radio-rx888-coverage = Cobertura de 0–{ $mhz } MHz, { $mbytes } MB/s por USB
settings-radio-rx888-digitised = de los { $mhz } MHz digitalizados
settings-radio-elad-16-bit = { $khz } kHz (muestras de 16 bits)
settings-radio-elad-pad = Atenuador de { $db } dB conectado
settings-radio-hackrf-rx-preamp = Preamplificador de { $db } dB en recepción
settings-radio-hackrf-tx-preamp = Preamplificador de { $db } dB en transmisión
settings-radio-device-not-found = { $device } (no encontrado)
settings-radio-lime-chain = Cadena { $n } (RX{ $n }_*)
settings-radio-lime-chain-tx = Cadena { $n } (RX{ $n }_* / TX{ $n }_*)
settings-radio-lime-second-chain = La otra cadena de recepción de la placa, en los conectores { $sockets }. Comparte el sintetizador, así que oye el mismo ancho en el mismo instante que la primera — lo que le permite llevar una segunda antena, o una muestra de tu propio transmisor.
settings-radio-tx-blocked = La transmisión está bloqueada: { $reason }
settings-radio-rfe-rx-fallback = Al recibir por { $port }, estas recurren al camino de banda ancha sin filtrar: { $bands }.
settings-radio-rfe-tx-fallback = Al transmitir por { $port }, estas recurren al camino de banda ancha — sin amplificador de banda y sin filtrado: { $bands }.

## Top bar and control strip (src/app/top_bar.rs)

topbar-how-far-one-press-moves = Cuánto mueve el dial una pulsación. Toca para tomar el siguiente paso: 10 Hz, 100 Hz, 500 Hz, 1, 2,5, 5, 9, 10 y 25 kHz. Quita la fila en Ajustes › Interfaz.
topbar-volume-gain-agc-squelch-noise = Volumen, ganancia, AGC, silenciador, los controles de ruido y la lectura de RDS en FM de radiodifusión
topbar-receiver = RECEPTOR
topbar-vfo-b-split-rit-xit = VFO A/B, split y los desplazamientos RIT/XIT
topbar-tuning = SINTONÍA
topbar-diversity-filter-which-way-combines = El filtro de diversidad: de qué manera combina las dos antenas, con qué rapidez se adapta, y mantenerlo donde está
topbar-diversity = DIVERSIDAD
topbar-second-receiver-s-frequency-mode = La frecuencia, el modo, el filtro y el nivel del segundo receptor
topbar-sub-receiver = SUBRECEPTOR
topbar-transceiver-s-own-aerial-socket = El conector de antena propio del transceptor y su interruptor de encendido
topbar-radio = RADIO
topbar-tune-voice-keyer-drive-mic = Sintonizar, el manipulador de voz y los niveles de excitación y micrófono
topbar-transmit = TRANSMISIÓN
topbar-tune = {" "}TUNE{" "}
topbar-panadapter-two-layers-their-speeds = El panadaptador — sus dos capas, sus velocidades y detalle, retención de picos — además del contraste de la cascada, el tamaño de FFT y los skimmers
topbar-display = PANTALLA
topbar-logbook-spots-awards-memories-scanner = Diario, spots, diplomas, memorias, el escáner, ajustes y el manual
topbar-system = SISTEMA
topbar-linked = Enlazado
topbar-not-linked = Sin enlazar
topbar-bar-trace = barra / traza
topbar-needle-bar-trace = aguja / barra / traza
topbar-swap-vfos = Intercambiar VFO
topbar-copy-b = Copiar A a B
topbar-second-receiver-right-ear-tunes = Segundo receptor, en el oído derecho. Sintoniza independientemente de A/B — sus controles aparecen en el módulo SUB, y su banda pasante en la cascada.
topbar-band-plan = , del plan de bandas
topbar-repeater-shift-following-band-plan = Desplazamiento de repetidor: siguiendo el plan de bandas, que no tiene desplazamiento para esta frecuencia — símplex
topbar-repeater-shift-transmit-above-below = Desplazamiento de repetidor: transmitir por encima o por debajo del dial, para trabajar un repetidor. Ahora símplex
topbar-repeater-shift = DESPLAZAMIENTO DE REPETIDOR
topbar-auto = AUTO
topbar-take-shift-band-plan-dial = Toma el desplazamiento del plan de bandas a medida que se mueve el dial. Solo habla dentro de una subbanda de salida de repetidores — en cualquier otra parte deja la radio en símplex, de modo que los canales de llamada siguen en símplex.
topbar-offset = Desplazamiento
topbar-repeater-tone-ctcss-dcs-goes = Tono de repetidor: el CTCSS/DCS que sale bajo la voz, el tono de 1750 Hz y el silenciador por tono de recepción
topbar-transmit-tone = TONO DE TRANSMISIÓN
topbar-set-here-sent-nfm-sub = fijado aquí, enviado en NFM — la señalización subaudible es cosa de un canal FM
topbar-normal = NORMAL
topbar-invert = INVERTIDO
topbar-bit-order-dcs-encoded-here = el orden de bits con que se codifica aquí el DCS está transcrito de la norma y nunca se ha comprobado con un repetidor — si no se abre, prueba CTCSS
topbar-t-1750-hz-burst = TONO DE 1750 HZ
topbar-every-over = CADA TURNO
topbar-open-burst-every-time-transmitter = Abrir con el tono cada vez que se activa el transmisor
topbar-send = ENVIAR
topbar-send-burst-now-keying-transmitter = Enviar el tono ahora — activando el transmisor durante su duración si no está ya activado
topbar-nfm-only-burst-fm-repeater = Solo NFM: el tono es el abrepuertas de un repetidor FM
topbar-receive-tone-squelch = SILENCIADOR POR TONO DE RECEPCIÓN
topbar-require-receive-what-station-transmits = Exigir en recepción lo que esta estación transmite
topbar-hide-docked-band-selector-click = Ocultar el selector de banda acoplado — haz clic de nuevo para recuperarlo
topbar-show-docked-band-selector = Mostrar el selector de banda acoplado
topbar-dock = ACOPLAR
topbar-keep-band-mode-selector-open = Mantiene abierto el selector de banda y modo junto a la cascada en lugar de cerrar esta ventana emergente cada vez
topbar-band-mode = BANDA Y MODO
topbar-hide-band-chip-brings-back = Ocultar — el chip de banda lo recupera
topbar-undock = DESACOPLAR
topbar-return-selector-top-bar-popup = Devuelve el selector a la ventana emergente de la barra superior
topbar-dec-off = DEC desact.
topbar-vol = Vol
topbar-counted-steps-not-decibels-what =


    Contada en pasos, no en decibelios: lo que vale un paso es asunto del propio receptor, y en un SDRplay depende también de la banda. Derecha sigue siendo más ganancia.
topbar-gain = Gan.
topbar-plutosdr-s-own-agc-setting =


    El propio AGC del PlutoSDR está fijando esta ganancia, así que mover el deslizador no hace nada. Pon Ajustes → Radio → AGC en Manual para controlarla aquí.
topbar-agc-hang-time-click-cycle = Tiempo de retención del AGC — haz clic para alternar: Off / Lento / Med / Rápido
topbar-radio-s-own-agc-click = El AGC propio de la radio — haz clic para alternar: Lento / Med / Rápido. No se puede desactivar desde aquí.
topbar-man = Man
topbar-manual-audio-gain-used-while = Ganancia de audio manual, usada mientras el AGC está desactivado. Se inicia con el nivel que mantenía el AGC cuando se desactivó.
topbar-radio-s-own-squelch-sent = El silenciador propio de la radio, enviado por el enlace de control. Es la puerta por la que realmente pasa el audio — la de software solo cerraría más sobre lo que el equipo ya dejó pasar.
topbar-max-gain = ganancia máx.
topbar-manual-gain = ganancia manual
topbar-noise-reduction = reducción de ruido
topbar-impulse-noise-blanker = Supresor de ruido impulsivo
topbar-auto-notch-cancel-constant-tone = Auto-notch: cancela los elementos de tono constante (heterodinos)
topbar-noise-reduction-off-click-switch = Reducción de ruido, desactivada — haz clic para activarla, o elige un motor en las filas de abajo
topbar-binaural-audio-not-while-sub = Audio binaural — no mientras el subreceptor tiene el oído derecho
topbar-binaural-audio-passband-spread-across = Audio binaural: la banda pasante se reparte entre los dos oídos, así que las señales de distinto tono vienen de distintas direcciones y la que sintonizas flota de un lado a otro. Haz clic para volver a mono
topbar-binaural-cw-spread-passband-across = CW binaural: reparte la banda pasante entre los dos oídos, de modo que las señales de distinto tono vienen de distintas direcciones — un pile-up se convierte en varios lugares en lugar de una nota apiñada, y al sintonizar una estación esta flota de un lado a otro. Mejor con auriculares
topbar-binaural-audio-spread-passband-across = Audio binaural: reparte la banda pasante entre los dos oídos. En voz el ruido se reparte por toda la imagen mientras la estación se queda en medio, lo que es más fácil de escuchar durante una hora — a costa de que la propia voz se reparta. Mejor con auriculares
topbar-mute = SILENCIO
topbar-record-audio-raw-i-q = Grabar el audio, el I/Q en bruto, o ambos
topbar-wfm-stereo-forced-off-click = Estéreo WFM forzado a desactivado — haz clic para estéreo automático
topbar-wfm-stereo-pilot-locked-click = Estéreo WFM: piloto enganchado. Haz clic para forzar mono
topbar-wfm-stereo-automatic-no-pilot = Estéreo WFM: automático, sin piloto en esta emisora
topbar-rds-data-arriving-click-see = Llegan datos RDS — haz clic para verlos
topbar-rds-no-data-station-click = RDS — sin datos en esta emisora. Haz clic de todos modos para ver el diagnóstico del decodificador
topbar-drm-decoding-click-details = DRM está decodificando — haz clic para los detalles
topbar-drm-click-decoder-s-state = DRM — haz clic para ver el estado del decodificador
topbar-hd-radio-decoding-click-details = HD Radio está decodificando — haz clic para los detalles
topbar-hd-radio-click-decoder-s = HD Radio — haz clic para ver el estado del decodificador
topbar-presets = PREAJUSTES
topbar-preset-right-click = Clic derecho para cambiar este preajuste
topbar-preset-edit = EDITAR PREAJUSTE
topbar-preset-label = Etiqueta
topbar-preset-label-hint = Lo que muestra el botón. Hasta 8 caracteres.
topbar-preset-lo = Inf
topbar-preset-hi = Sup
topbar-preset-use-current = Usar actual
topbar-preset-use-current-hint = Tomar los bordes del filtro que tiene ahora el receptor
topbar-preset-save = Guardar
topbar-preset-reset = Restablecer
topbar-preset-reset-hint = Restaurar el preajuste integrado de este modo
topbar-preset-cancel = Cancelar
topbar-width-per-sideband = ANCHO POR BANDA LATERAL
topbar-width = ANCHO
topbar-passband-width-hertz-type-drag = Ancho de la banda pasante en hercios — escríbelo o arrastra. Haz clic en el campo para introducir una cifra exacta.
topbar-edges = BORDES
topbar-low-edge-hz-carrier = Borde inferior, en Hz desde la portadora
topbar-high-edge-hz-carrier = Borde superior, en Hz desde la portadora
topbar-noise-reduction-voice-off-click = Reducción de ruido (voz), desactivada — haz clic para elegir un motor
topbar-record-audio = GRABAR AUDIO
topbar-mp3 = ● MP3
topbar-what-you-hear-mp3-receiver = Lo que oyes, como MP3: el receptor en un canal y tu propia transmisión en el otro. Esta es la grabación de un QSO.
topbar-mono = MONO
topbar-mixed-one-channel-click-two = Mezclado en un canal — haz clic para dos
topbar-two-channels-rx-left-tx = Dos canales: RX a la izquierda, TX a la derecha — haz clic para un canal mezclado
topbar-quick-clip = CLIP RÁPIDO
topbar-stop-after = PARAR TRAS
topbar-no-stop = sin parada
topbar-auto-record = GRABACIÓN AUTOMÁTICA
topbar-set-squelch-auto-record-needs = Fija un silenciador — la grabación automática necesita uno para saber qué es silencio
topbar-auto-record-needs-sdr-front = La grabación automática necesita una etapa de entrada SDR con silenciador por software
topbar-stop-following-squelch = Dejar de seguir al silenciador
topbar-waiting-signal = esperando una señal
topbar-record-spectrum = GRABAR ESPECTRO
topbar-i-q-wav = ● WAV I/Q
topbar-i-q-wav-2 = WAV I/Q
topbar-raw-spectrum-receiver-delivering-stereo = El espectro en bruto que entrega el receptor, como un WAV estéreo de 32 bits en coma flotante (RF64 por encima de 4 GB) que abren SDR#, SDRuno, HDSDR y SDRangel — y que el propio sdroxide reproduce con --file. Grande: 8 bytes por muestra.
topbar-radio-hands-over-demodulated-audio = Esta radio entrega audio demodulado, así que no hay I/Q que grabar.
topbar-what-capture-costs-sample-rate = Lo que cuesta una captura con esta frecuencia de muestreo
topbar-engine = MOTOR
topbar-no-noise-reduction-decoders-never = Sin reducción de ruido — los decodificadores nunca la vieron de todos modos
topbar-strength = INTENSIDAD
topbar-tone = TONO
topbar-ctcss-dcs-no-sub-audible = CTCSS / DCS — sin tono subaudible en esta señal
topbar-tone-squelch = SILENCIADOR POR TONO
topbar-carrier-squelch-open-any-signal = Silenciador por portadora: abre con cualquier señal
topbar-require-tone-currently-being-received = Exigir el tono que se está recibiendo ahora
topbar-ctcss = CTCSS
topbar-any-dcs = CUALQUIER DCS
topbar-open-any-dcs-coded-signal = Abre con cualquier señal codificada con DCS. Cuál de los 104 códigos lleva no se puede leer de forma fiable aquí, así que no hay nada más fino que elegir
topbar-which-socket-back-radio-receiving =
    En qué conector de la parte trasera está recibiendo la radio — su propio comando ANT, el mismo ajuste que el botón ANT del panel frontal. Haz clic para pasar al siguiente.

    La elección se recuerda por banda, y se restituye la próxima vez que el dial entra en esa banda.
topbar-radio-s-separate-receiving-antenna =
    La antena de recepción independiente de la radio, conmutada dentro o fuera de la ruta de recepción — su propio ajuste RX ANT. La antena del conector principal sigue en transmisión en ambos casos.

    La radio lo recuerda por banda ella misma, así que sdroxide lo lee de vuelta tras cada cambio de banda en lugar de restituir lo último que vio: hacer clic aquí es lo único que lo mueve.
topbar-radio-s-own-power-switch =
    El interruptor de encendido propio de la radio, por el enlace de control, y el único encendido/apagado de verdad del programa — no el interruptor ENLACE de sdroxide, que cierra el extremo de sdroxide y deja la radio funcionando.

    Para que ENCENDER llegue a algo, el extremo de control de la radio tiene que seguir despierto mientras está apagada: Network Control por la LAN, o un puerto CI-V todavía alimentado desde la red eléctrica en un equipo apagado desde el frontal.
topbar-switch-radio = Encender la radio
topbar-switch-radio-off-audio-meters = Apagar la radio. El audio y los medidores se detienen con ella.
topbar-what-second-aerial-click-swap =
    Para qué sirve la segunda antena — haz clic para cambiar.

    CANCELAR la resta de la primera, con la ganancia, fase y retardo que hacen que una fuente de ruido local coincida en ambas: la forma DSP de un fasador de cancelación de ruido. COMBINAR suma las dos en la fase que refuerza, ponderadas por lo bien que oye cada una — recepción en diversidad, que rellena los desvanecimientos.
topbar-hold = MANTENER
topbar-stop-filter-moving-reach-moment = Detiene el movimiento del filtro. Recurre a esto en cuanto aparezca un nulo: un filtro que se deja adaptando se volverá a apuntar a lo que se vuelva más fuerte, que en una banda tranquila es la estación que estás escuchando.
topbar-restart = REINICIAR
topbar-zero-filter-find-null-again = Pone el filtro a cero y vuelve a buscar el nulo.
topbar-adapt = Adapt.
topbar-how-fast-filter-chases-slow = Con qué rapidez persigue el filtro: lento y constante a la izquierda, convergiendo en una fracción de segundo y oscilando visiblemente a la derecha. Empieza rápido para encontrar el nulo y luego MANTENLO.
topbar-where-sub-receiver-listens-shift = Dónde escucha el subreceptor. Haz Mayús+clic en la cascada, o arrastra dentro de la banda pasante del sub, para moverlo.
topbar-dial = ←DIAL
topbar-move-sub-receiver-main-dial = Mover el subreceptor al dial principal
topbar-dial-2 = DIAL←
topbar-move-main-dial-sub-receiver = Mover el dial principal al subreceptor
topbar-filter = Filtro
topbar-sub-receiver-passband-edges-hz = Bordes de la banda pasante del subreceptor, en Hz
topbar-sub-receiver-level-plays-right = Nivel del subreceptor (suena en el oído derecho)
topbar-ptt = {" "}PTT{" "}
topbar-voice-keyer-record-transmit-stored = Manipulador de voz: grabar y transmitir mensajes guardados
topbar-drive = Excit.
topbar-tune-2 = Tune
topbar-tx-audio-processing-microphone-click = Audio TX está procesando el micrófono — haz clic para abrirlo
topbar-click-open-tx-audio-gate = Haz clic para abrir Audio TX: puerta, ecualizador, compresor y limitador para el micrófono
topbar-mic = Mic
topbar-cessb = CESSB
topbar-microphone-gain = Ganancia del micrófono
topbar-deviation-how-far-mode-s =
    Desviación: cuánto oscila la ráfaga de este modo en una radio que la modula por sí misma. Un transmisor FM convierte el nivel de audio en excursión de frecuencia y no tiene ALC que la frene, así que el fondo de escala en una entrada de datos ajustada para voz sobredesvía — lo que suena completamente normal para un oyente y no decodifica para nadie.

    Se guarda por modo, así que una desviación fijada para packet a 1200 baudios nunca cae sobre FT8.
topbar-transmit-audio-how-hard-mode =
    Audio de transmisión: con qué fuerza este modo excita el modulador de una radio que modula lo que le enviamos — un equipo CAT en su tarjeta de sonido, una FLEX, un Icom por su puerto de red. Bájalo hasta que el ALC del equipo apenas se mueva y fija la potencia en la radio; un ALC trabajando sobre un modo digital de envolvente constante es lo que genera salpicaduras.

    La Excitación no es este control: en estas radios la Excitación llega al registro de potencia del equipo y nunca toca su audio.

    Se guarda por modo — FT8, RTTY, PSK y MCW guardan cada uno el suyo.
topbar-tx-audio = Audio TX
topbar-cw-psk-rtty-skimmers-decode = Skimmers de CW / PSK / RTTY — decodifican señales por toda la banda y las marcan en la cascada
topbar-skimmers = SKIMMERS
topbar-run-skimmer = Ejecutar este skimmer
topbar-minimum-snr-decoded-signal-needs = SNR mínima que necesita una señal decodificada para ser anotada como spot
topbar-needs-wideband-iq-source = necesita una fuente IQ de banda ancha
topbar-cw-decoder = DECODIFICADOR DE CW
topbar-how-many-signals-model-reads = Cuántas señales lee el modelo a la vez. El resto conserva su marcador pero no lleva texto.
topbar-spots = SPOTS
topbar-save-skimmer-s-spot-list = Guardar en un archivo la lista de spots del skimmer
topbar-show-hide-full-band-waterfall = Mostrar/ocultar la franja de cascada de banda completa sobre el panadaptador — todo lo que este receptor puede ver a la vez
topbar-spectrum-waterfall-either-layer-both = Espectro y cascada — cualquiera de las capas, ambas, o ninguna. Iluminado mientras se muestran ambas.
topbar-spectrum = ESPECTRO
topbar-show-spectrum = MOSTRAR ESPECTRO
topbar-draw-spectrum-line-across-top = Dibuja la línea de espectro en la parte superior del panadaptador. Desactivado, la cascada ocupa toda la altura.
topbar-peak-hold = RETENCIÓN DE PICOS
topbar-trace-highest-level-each-column = Traza sobre la línea en vivo el nivel más alto que ha alcanzado cada columna, decayendo poco a poco — lo que hizo la banda mientras mirabas a otro lado
topbar-draw-spectrum-receding-surface-instead = Dibuja el espectro como una superficie que se aleja en lugar de una línea plana: el espectro más reciente al frente, los anteriores alejándose de ti. Los últimos segundos de la banda como un paisaje — una portadora que va y viene es una cresta en lugar de una línea que tiembla. La rejilla y la retención de picos pertenecen a la línea plana y no se dibujan en ella.
topbar-how-quickly-spectrum-line-follows = Con qué rapidez sigue la línea de espectro a la banda. Más lento promedia más tramas entre sí: una línea más estable, y una portadora débil que se queda quieta el tiempo suficiente para leerla. La cascada no se ve afectada — esas filas reciben todas las tramas en cualquier caso.
topbar-waterfall = CASCADA
topbar-show-waterfall = MOSTRAR CASCADA
topbar-draw-scrolling-waterfall-below-spectrum = Dibuja la cascada desplazable bajo el espectro. Desactivado, la línea de espectro ocupa toda la altura.
topbar-how-fast-waterfall-scrolls-lines = Con qué rapidez se desplaza la cascada, en líneas por segundo: Lenta 5, Media 28, Rápida 56, Más rápida 112, La más rápida 224. El motor las temporiza él mismo, así que las dos más rápidas son detalle real y no la misma línea dibujada dos veces — hasta donde el receptor puede alimentarlas: una línea nunca puede mostrar más de una transformada, y una etapa de entrada estrecha produce solo unas pocas docenas por segundo. Cuestan historial, ya que la cascada guarda un número fijo de líneas — 73 segundos en Media, 9 en La más rápida.
topbar-decode-labels = ETIQUETAS DE DECODIFICACIÓN
topbar-mark-every-decoded-station-waterfall = Marca cada estación decodificada en la cascada con su indicativo, en la frecuencia en que se oyó — FT8, FT4 y los demás modos con ranuras. Una buena apertura pone treinta de ellas a lo ancho dos veces por minuto, sobre las trazas que estás leyendo; desactívalas y la cascada es solo la banda. La lista de decodificaciones de al lado sigue teniendo todos los indicativos.
topbar-switch-3d-set = Activa 3D para fijar esto
topbar-how-3d-spectrum-drawn-only = Cómo se dibuja el espectro 3D. Solo difiere la forma — ambas representaciones muestran los mismos espectros, y ambas ocultan lo que hay detrás, así que una señal fuerte al frente se alza sobre la banda que cubre.
topbar-lines = LÍNEAS
topbar-one-trace-per-remembered-spectrum = Una traza por espectro recordado, en el color propio de la línea plana, cada una ocultando las que tiene detrás. La forma sin los niveles, y la lectura que conviene cuando lo que buscas es la forma — cuesta un poco más que la sólida en lugar de menos, porque dibuja la misma superficie y luego traza cada cresta encima.
topbar-solid = SÓLIDO
topbar-filled-surface-coloured-waterfall-s = Una superficie rellena coloreada con la paleta de la cascada, de modo que el nivel está en el color además de en la altura y las dos mitades del panadaptador coinciden en cómo es una señal fuerte. Cambia la paleta en Ajustes › Pantalla.
topbar-how-fast-3d-spectrum-flows = Con qué rapidez se aleja de ti el espectro 3D, en filas por segundo: Lenta 6, Media 12, Rápida 24, Más rápida 48. La superficie tiene un número fijo de filas de profundidad, así que esto es también cuánto tiempo retiene. Más lento es una memoria más larga y una superficie que se arrastra; más rápido es una más corta que se mueve. La cascada conserva su propia velocidad de desplazamiento, más abajo.
topbar-how-many-columns-panadapter-waterfall = Con cuántas columnas se dibujan el panadaptador y su cascada. Auto lee el renderizador de esta máquina y el tamaño del panadaptador y elige lo máximo que puede soportar — una pantalla 4K quiere 4096. Cada columna es un byte en cada trama, así que un cliente conectado a una estación remota paga el detalle en su enlace: 4096 columnas a 60 fps son aproximadamente un cuarto de megabyte por segundo, el doble del ancho estándar.
topbar-read-machine-screen-take-most = Lee esta máquina y esta pantalla, toma lo máximo que pueden soportar y las sigue si cambian
topbar-width-force-right-now-whatever = El ancho en vigor ahora mismo, pida lo que pida la fila de arriba
topbar-keep-floor-ceiling-set-best = Mantiene el suelo/techo fijados para el mejor contraste de la cascada — reajustados suavemente tras un cambio de banda, tras un desplazamiento o zoom, y cuando los niveles derivan. Actívalo para ajustar de una vez; desactívalo para mantener los niveles donde los fijaste.
topbar-keep-tuned-frequency-middle-panadapter = Mantiene la frecuencia sintonizada en el centro del panadaptador: la ventana se desliza bajo el dial en lugar de que la imagen salte un ancho entero cuando sintonizas fuera del borde. Actívalo para centrar de una vez; desactívalo para desplazar y hacer zoom donde quieras.
topbar-spectrum-floor-ceiling-fft-size = Suelo / techo del espectro y tamaño de FFT
topbar-level-drawn-bottom-spectrum-darkest = El nivel dibujado en el fondo del espectro y como el color más oscuro de la cascada. Súbelo hasta que el suelo de ruido apenas se oscurezca. Fijarlo desactiva FIT.
topbar-ceil = techo{" "}
topbar-level-drawn-top-bring-down = El nivel dibujado arriba. Bájalo hasta que la señal más fuerte que te interesa llegue al color pleno. Fijarlo desactiva FIT.
topbar-fft-size = TAMAÑO FFT
topbar-flip = INVERTIR
topbar-scroll-waterfall-upwards-newest-row = Desplazar la cascada hacia arriba — fila más reciente abajo (V)
topbar-logbook-all-qsos-digital-manual = Diario — todos los QSO (digitales + manuales)
topbar-live-spots-dx-cluster-pota = Spots en vivo — clúster DX, POTA, SOTA, PSK Reporter
topbar-award-tracking-dxcc-was-waz = Seguimiento de diplomas — DXCC / WAS / WAZ / locátores
topbar-band-conditions-published-forecast-beside = Condiciones de banda — el pronóstico publicado junto a lo que realmente se ha oído en cada banda
topbar-satellite-qo-100-beacon-calibration = Satélite — la calibración con la baliza de QO-100 está en marcha
topbar-satellite-doppler-tracking-qo-100 = Satélite — seguimiento Doppler y calibración con la baliza de QO-100
topbar-ism-band-devices-decoding-now = Dispositivos de banda ISM — decodificando ahora, esté o no abierta esta ventana. Desactívalo con DECODIFICAR dentro de la ventana.
topbar-ism-band-devices-weather-sensors = Dispositivos de banda ISM — sensores meteorológicos, contadores y domótica que se oyen a tu alrededor
topbar-public-sdrs-internet-browse-kiwisdr = SDR públicos en internet — explora los directorios de KiwiSDR y SpyServer y abre uno como radio
topbar-winlink-radio-email = Correo por radio Winlink
topbar-memory-channels = Canales de memoria
topbar-scanner-stopped-signal = Escáner — parado en una señal
topbar-scanner-running = Escáner — en marcha
topbar-scan-memory-channels-frequency-range = Barrer canales de memoria o un rango de frecuencias
topbar-hfdl-ground-network-decode-log = Red terrestre HFDL — el registro de decodificaciones y el mapa de aeronaves, bajo la cascada. Activa el decodificador con ESCUCHAR dentro del panel.
topbar-hfdl-ground-network-decoding-now = Red terrestre HFDL — decodificando ahora. Abre el panel.
topbar-hfdl-ground-network-aircraft-shortwave = Red terrestre HFDL — el enlace de datos de onda corta de las aeronaves, un canal de escucha cada vez
topbar-grid-tracker-worked-maidenhead-squares = Seguimiento de locátores — cuadrados Maidenhead trabajados en un mapa, con lo que se oye
topbar-amplifier-set-up-hardrock-under = Amplificador — configura un Hardrock en Ajustes → Amplificador
topbar-settings-device-gains-antennas-audio = Ajustes — ganancias del dispositivo, antenas, dispositivos de audio
topbar-user-manual-f1 = Manual de usuario (F1)
topbar-range-was-typed-settings-radio = Ese rango se escribió en Ajustes ▸ Radio ▸ Rango RX, no lo informó la radio. Amplíalo, o vacía la casilla para usar lo que diga el dispositivo de sí mismo.
topbar-what-device-reports-about-itself = Eso es lo que informa el dispositivo de sí mismo. Si la radio sí cubre esta banda, indica su rango real en Ajustes ▸ Radio ▸ Rango RX.
topbar-band = BANDA
topbar-mode = MODO
topbar-digital = DIGITAL

## Top bar: the System box's window chips.

topbar-chip-log = DIARIO
topbar-chip-spots = SPOTS
topbar-chip-awards = DIPLOMAS
topbar-chip-bands = BANDAS
topbar-chip-sat = SAT
topbar-chip-ism = ISM
topbar-chip-public-sdr = SDR PÚBLICO
topbar-chip-mail = CORREO
topbar-chip-mem = MEM
topbar-chip-scan = BARRIDO
topbar-chip-hfdl = HFDL
topbar-chip-grid = LOCÁT.
topbar-chip-amp = AMP
topbar-chip-settings = ⚙ AJUSTES
topbar-chip-help = ? AYUDA

## Top bar: the mode-defaults chip.

topbar-changed-agc = AGC
topbar-changed-squelch = silenciador
topbar-changed-auto-notch = auto-notch
topbar-changed-stereo = estéreo
topbar-changed-binaural = binaural
topbar-mode-defaults =
    Volver a los valores predeterminados de { $mode }. Cambiado respecto a ellos: { $changed }.

    Haz clic para restituirlos. Vuelven los valores propios de { $mode }, y lo que fijaste aquí se olvida.

## Settings window, Radio tab: longer messages with values

settings-radio-no-such-baud = la radio no tiene el ajuste de { $baud } baudios — se usará { $default } en su lugar
settings-radio-icom-scope-hint =
    Transmite el barrido del scope propio de la radio por el enlace CI-V y lo dibuja como panadaptador — la única imagen de la banda que puede dar un equipo de audio demodulado, ya que el audio que envía es lo que ya pasó por su filtro.

    Necesita la radio preparada para ello: CI-V USB Baud Rate en 115200 y CI-V USB Port en «Unlink from {"["}REMOTE]» (ambos en SET > Connectors > CI-V), y los baudios de arriba ajustados en consecuencia — por debajo de { $min_baud } los barridos no caben por el enlace y el scope queda desactivado.

    Los barridos comparten el bus USB interno de la radio con su propia tarjeta de sonido. Si el audio recibido empieza a cortarse con el scope activado, esta casilla es lo primero que hay que probar a desactivar.
settings-radio-rx-tx-words = { $rx } / TX { $tx }
settings-radio-soapy-native = El hardware de arriba lo soporta directamente sdroxide: { $names }. Seleccionar esa interfaz arriba te da sus propios ajustes — etapas de ganancia, filtros y notches que SoapySDR no puede expresar — y no necesita ningún módulo SoapySDR.
settings-radio-rx888-width-hint = Cuánto del espectro digitalizado muestra el panadaptador a la vez. Toda la cadena de DSP funciona a este ancho, así que más ancho cuesta proporcionalmente más CPU — 1/2 es la banda entera en la cascada, y una cantidad seria de aritmética. Por encima del cruce de VHF ({ $crossover } MHz) el panadaptador no mira la antena sino la FI del sintonizador, que tiene 8 MHz de ancho y está aparcada en { $if_centre } MHz — así que nada más ancho que { $widest } puede centrarse en ella, y el ancho extra es espectro muerto junto a la señal en lugar de más de ella.
settings-radio-sdrplay-serial-missing = El número de serie { $serial } no está entre los receptores que informa el servicio de SDRplay. Elige uno de los receptores listados, o vuelve a enchufar este y pulsa Volver a buscar.
settings-radio-hdr-filter-hint =
    El filtro analógico delante del camino HDR — un control distinto del ancho de banda propio del receptor. Cada ajuste se construye para un conjunto fijo de centros y no puede hacer nada en otros; este se construye en { $centres }. Se aplica al instante.

    El camino en sí aún no funciona — mira el interruptor de arriba —, así que esto elige un filtro para un modo que ahora mismo está en silencio.
settings-radio-rspduo-only = Solo un RSPduo tiene un segundo sintonizador, y el receptor seleccionado aquí es un { $model }, así que esto no hará nada.
settings-radio-lime-tx-gain-low = La ganancia de transmisión es { $db } dB, en el extremo inferior de su rango — unos pocos microvatios fuera de la placa, que se leerán como nada en un medidor de potencia sea lo que sea que haya tras ella. Es el valor predeterminado para que un transmisor armado no pueda sorprenderte; súbela antes de transmitir.

## Settings window, General: SWR

settings-general-swr-tune-limit = (sintonización: { $limit }:1)

## Top bar and control strip: messages with values

topbar-smeter-hover =
    { $reading }

    Haz clic para cambiar de esfera del medidor: { $faces }
topbar-step-down = Abajo { $step }
topbar-step-up = Arriba { $step }
topbar-repeater-shift-hover = Desplazamiento de repetidor { $shift } — recibiendo en { $rx }, transmitiendo en { $tx } MHz{ $auto }
topbar-tone-and-burst = Transmitiendo { $tone } bajo la voz, y un tono de 1750 Hz de { $ms } ms al comienzo de cada turno
topbar-tone-under-voice = Transmitiendo { $tone } bajo la voz
topbar-burst-every-over = Un tono de 1750 Hz de { $ms } ms al comienzo de cada turno
topbar-match-tx = IGUALAR A TX ({ $tone })
topbar-decimation-on = Diezmado de la etapa de entrada: al receptor se le está dando el 1/{ $n } central de los { $msps } Msps que transmite esta radio — { $span } — y el resto se descarta antes de que nada de ello llegue al receptor.
topbar-decimation-off = Diezmado de la etapa de entrada, desactivado: el receptor ve los { $span } completos que transmite esta radio.
topbar-decimation-hint =
    { $what }

    Un ancho más estrecho significa resolución más fina de la cascada, un suelo de ruido más silencioso (3 dB por cada mitad) y menos CPU — a costa de la banda a cada lado. El dial sigue sintonizando en cualquier sitio; la radio mueve su LO para seguirlo.

    Haz clic para alternar: desactivado / 2 / 4 … { $max }.
topbar-rx-gain-hint = Ganancia RX de la etapa de entrada ({ $stage }). Demasiada recorta el ADC del receptor y reparte señales espurias por la banda; demasiado poca y se queda sordo.
topbar-rx-gain-stages =
    {""}

    Este equipo tiene { $n } etapas de ganancia RX — las demás están en Ajustes → Dispositivo.
topbar-passband-now =
    {""}

    La banda pasante está ahora en { $dbfs } dBFS.
topbar-squelch-hint =
    Silencia el audio por debajo de esta potencia en la banda pasante de recepción, en dBFS. A la izquierda está abierto.{ $level } Fíjalo por encima del ruido y por debajo de la señal.

    Un flujo que llega con el AGC propio de la radio ya aplicado — la FI de 12 kHz de un Icom, por ejemplo — queda mucho más alto en esta escala que la banda base cruda de un SDR, por lo que el carril llega al fondo de escala.
topbar-nr-cycle-hint =
    Reducción de ruido: { $strength } — { $engine }

    Haz clic para alternar la intensidad: Baja / Media / Alta / Off. El motor está en las filas de abajo.
topbar-recording-both = Grabando { $audio } y { $iq }
topbar-recording-audio = Grabando audio en { $audio }
topbar-recording-iq = Grabando I/Q en { $iq }
topbar-auto-record-armed = Grabación automática armada: un archivo por transmisión, cerrado tras { $secs } s de silencio
topbar-rds-hover = RDS: { $ps }. Haz clic para ver los datos de la emisora
topbar-drm-hover = DRM: { $service }. Haz clic para ver los detalles de la emisión
topbar-drm-no-codec = DRM: { $summary } — enganchado, pero su audio { $codec } no se puede decodificar aquí. Haz clic para ver qué falta
topbar-drm-state = DRM: { $summary } — haz clic para ver el estado del decodificador
topbar-hd-hover = HD Radio: { $station }. Haz clic para ver los detalles de la emisión
topbar-hd-state = HD Radio: { $summary } — haz clic para ver el estado del decodificador
topbar-binaural-preset = { $preset } en cada oído — { $lo } … { $hi } Hz, ambas bandas laterales
topbar-nr-hover =
    Reducción de ruido: { $strength } — { $engine }

    Haz clic para cambiar el motor o la intensidad
topbar-clipping = Grabando clip de { $span } — pulsa de nuevo para empezar de cero
topbar-record-clip = Grabar { $span } y parar — un clip para adjuntar a un informe de recepción
topbar-starting-clip = iniciando un clip de { $span }…
topbar-stop-recording-after = Detener la grabación MP3 tras { $minutes } minutos
topbar-stops-in = se detiene en { $time }
topbar-auto-record-hint = Graba cada transmisión en su propio archivo, cerrado tras { $secs } segundos de silencio
topbar-recording-silence = grabando · { $secs } s de silencio (se cierra a los { $hold })
topbar-tone-open = Recibiendo { $tone }, que es el silenciador por tono — audio abierto
topbar-tone-closed = Recibiendo { $heard }, pero el silenciador por tono quiere { $wanted } — el audio sigue cerrado
topbar-tone-heard = Recibiendo CTCSS/DCS { $tone }
topbar-tone-nothing = Silenciador por tono { $tone }: no se está recibiendo nada que coincida
topbar-tone-use = USAR { $tone }
topbar-sockets = Conectores: { $sockets }
topbar-voice-keyer-tx = Transmitiendo { $slot } — haz clic para abrir el manipulador de voz
topbar-drive-uncal-gen = Esta frecuencia, que no está en ninguna banda de aficionado, no está calibrada, así que el ajuste llega entero al transmisor.
topbar-drive-uncal-band = { $band } no está calibrada, así que el ajuste llega entero al transmisor.
topbar-drive-cal-gen = Esta frecuencia, que no está en ninguna banda de aficionado, está calibrada { $trim } dB, así que el mismo ajuste pone aquí { $diff } dB { $more ->
        [less] menos
       *[more] más
    } en el aire que en una banda sin calibrar.
topbar-drive-cal-band = { $band } está calibrada { $trim } dB, así que el mismo ajuste pone aquí { $diff } dB { $more ->
        [less] menos
       *[more] más
    } en el aire que en una banda sin calibrar.
topbar-drive-hover =
    Con qué fuerza se excita el transmisor. En una radio que sdroxide modula por sí mismo escala las muestras moduladas; en un equipo con control de potencia propio es la fracción de la potencia nominal que se le pide al equipo.

    { $trim }

    Ajustes → Radio → Excitación de transmisión por banda.
topbar-cessb-hover =
    BLU de envolvente controlada: más potencia media para el mismo pico, sin salpicaduras — tres o cuatro decibelios de sonoridad aparente en el otro extremo. Cuántos decibelios se excita la voz en el procesador; 0 es desactivado, 6 es una primera prueba razonable. No toca la Excitación.

    Ahora: { $now }
topbar-waterfall-rate = { $rows } filas por segundo — { $seconds } segundos de banda
topbar-detail-auto-columns = { $n } columnas, lo que habría elegido Auto aquí
topbar-n-columns = { $n } columnas
topbar-fft-bins-whole = { $n } bins en todo el flujo — uno por columna, o más fino
topbar-fft-bins-pooled = { $n } bins agrupados en las { $cols } columnas del panadaptador: señales más nítidas, no más señales
topbar-sat-locked = Satélite — enganchado a { $name }
topbar-amp-in-line = { $model } — en línea (PTT)
topbar-amp-bypassed = { $model } — puenteado (OFF)
topbar-amp-not-answering = { $model } — no responde
topbar-filter-edges = { $lo } … { $hi } Hz desde la portadora
topbar-filter-isb = Filtro de recepción: { $width } Hz en cada oído ({ $edges }). Las dos bandas laterales de ISB son transmisiones separadas, así que el ancho es el de una de ellas.
topbar-filter-width = Filtro de recepción: { $width } Hz de ancho ({ $edges }).
topbar-filter-hint =
    { $what }

    Haz clic para el ancho, los dos bordes y los preajustes de este modo, como números — los tiradores del panadaptador colocan una banda pasante a ojo, aquí es donde se escribe una cifra exacta.
topbar-band-outside-range =
    { $band } queda fuera de lo que recibe esta radio ({ $ranges } MHz).

    { $why }
topbar-band-forecast = { $band }: { $verdict } ({ $time ->
        [day] de día
       *[night] de noche
    }) — pronóstico de HAMQSL.com a partir de los índices solares, no una medición de tu propio trayecto.

## Top bar: short words and captions

topbar-squelch-open = abierto
topbar-off = apagado
topbar-recording = grabando
topbar-sql = sql
topbar-stations = estaciones
topbar-caption-reaction = reacción
topbar-caption-scroll = desplaz.
topbar-caption-surface = superficie
topbar-caption-flow = flujo
topbar-caption-detail = detalle
topbar-floor = suelo

## Digital-mode panel tabs (src/app/panels/mod.rs, pane_label).

pane-aircraft = AERONAVES
pane-chart = GRÁFICO
pane-chat = CHAT
pane-decodes = DECODIF.
pane-files = ARCHIVOS
pane-heard = OÍDAS
pane-image = IMAGEN
pane-map = MAPA
pane-messages = MENSAJES
pane-monitor = MONITOR
pane-panel = PANEL
pane-qso = QSO
pane-reading = LECTURA
pane-receive = RECIBIR
pane-saved = GUARDADOS
pane-send = ENVIAR
pane-spots = SPOTS
pane-stations = ESTACIONES
pane-status = ESTADO
pane-terminal = TERMINAL
pane-text = TEXTO
pane-traffic = TRÁFICO
pane-vessels = BUQUES
pane-wfall = CASCADA

## Digital-mode panels, shared (src/app/panels/mod.rs)

panels-save = {" "}GUARDAR{" "}
panels-nothing-decoded-save = Nada decodificado que guardar
panels-clear-rx = {" "}BORRAR RX{" "}
panels-empty-receive-window-nothing-air = Vacía la ventana de recepción. Nada de lo que está en el aire se detiene.
panels-nothing-received-clear = Nada recibido que borrar
panels-save-what-panel-has-decoded = Guarda en un archivo lo que este panel ha decodificado
panels-prop = PROP
panels-shade-map-where-signals-are = Sombrea el mapa según dónde llegan realmente las señales, en cada banda. Cada recepción se sitúa en el punto medio de su trayecto — el trozo de ionosfera que la desvió — y no en la estación lejana, así que es un mapa del cielo y no de dónde viven los radioaficionados. Se construye con trayectos en los que ha participado esta estación y, cuando la Reverse Beacon Network está activada en Ajustes → Spots, con lo que oyen los skimmers del mundo, lo que cubre las bandas en que no está esta radio.
panels-night = NOCHE
panels-shade-where-sun-down-twilight = Sombrea dónde se ha puesto el Sol, y el crepúsculo entre medias, para que la línea gris se vea en el mapa. Las bandas bajas se alargan y las altas se cierran en el lado nocturno de ella, y el propio terminador es donde está el DX.
panels-all-bands = TODAS LAS BANDAS
panels-every-band-once-one-hue = Todas las bandas a la vez, un tono cada una — condiciones generales.
panels-one-band = UNA BANDA
panels-one-band-cold-hot-blue = Una banda, de frío a calor: azul, verde, amarillo, rojo.
panels-frequency-you-are-actually-working = La frecuencia en la que realmente trabajas: el dial más el desplazamiento de tono de al lado. Esto es lo que va al diario — el dial solo queda tanto por debajo.
panels-send-return = ENVIAR CON INTRO
panels-freq = ⇵ FREC
panels-nothing-agreed-mode-save-your = Nada acordado para este modo — guarda las tuyas abajo.
panels-forget-saved-frequency = Olvidar esta frecuencia guardada
panels-dial-already-one-your-saved = Este dial ya es una de tus frecuencias guardadas.
panels-dial-already-agreed-list-mode = Este dial ya está en la lista de acordadas para este modo.
panels-decode-squelch-raise-stop-decoding = Silenciador de decodificación — súbelo para dejar de decodificar ruido
panels-message-repeats-long-transmit-held = El mensaje se repite mientras se mantenga la transmisión — un meteoro capta la parte que esté pasando.
panels-w1abc-w9xyz-fn42 = W1ABC W9XYZ FN42
panels-tx = {"  "}TX ACT.{"  "}
panels-tx-2 = {"   "}TX{"   "}
panels-call-cq = {" "}LLAMAR CQ{" "}
panels-nocall = NOCALL

## TX Audio window (src/app/tx_audio.rs)

txaudio-heard-too-little-speech-talk = Se oyó muy poca voz — habla con normalidad durante toda la cuenta e inténtalo de nuevo.
txaudio-tx-audio = Audio TX
txaudio-mode-not-voice-mode-so = Este modo no es un modo de voz, así que nada de lo que envía pasa por la cadena. Los ajustes hechos aquí surten efecto en BLU, AM y FM.
txaudio-bypass = PUENTEAR
txaudio-switch-every-stage-out-once = Saca todas las etapas a la vez, incluido el reductor de ruido, de modo que el micrófono sale sin procesar. Haz clic de nuevo para restituir exactamente las etapas que estaban activas — el A/B más rápido de lo que hace la cadena.
txaudio-denoise = REDUCIR RUIDO
txaudio-rnnoise-microphone-ahead-every-other = RNNoise en el micrófono, antes de cualquier otra etapa, para que el ruido de la sala se elimine antes de que la puerta o el compresor puedan subirlo. Añade unos 10 ms de retardo.
txaudio-digital-voice = VOZ DIGITAL
txaudio-also-run-freedv-rade-digital = Pasa también por la cadena la voz digital FreeDV / RADE. Desactivado de forma predeterminada: un vocoder se entrena con voz natural y puede decodificar peor una voz muy procesada, así que pruébalo en el aire antes de fiarte. El audio TCI y los modos digitales nunca pasan por la cadena.
txaudio-test = Prueba
txaudio-rec = {" "}⏺ GRABAR{" "}
txaudio-play = {" "}▶ REPRODUCIR{" "}
txaudio-play-test-phrase-through-chain = Reproduce la frase de prueba por la cadena tal como está ajustada ahora, por tus altavoces. Ajusta mientras suena y oye el cambio. El audio de recepción se sustituye mientras suena.
txaudio-recording = grabando…
txaudio-no-phrase-yet = aún no hay frase
txaudio-live = En vivo
txaudio-live-2 = {" "}🎧 EN VIVO{" "}
txaudio-hear-your-microphone-through-chain = Oye tu micrófono a través de la cadena mientras hablas, sin activar el transmisor. Usa auriculares: por altavoces se realimenta. Te oyes con unas decenas de milisegundos de retraso, lo que hace incómodo hablar — fija los niveles en los medidores y juzga el sonido con GRABAR / REPRODUCIR. El audio de recepción se sustituye mientras funciona; pulsar PTT lo detiene.
txaudio-meters = {" "}MEDIDORES{" "}
txaudio-run-your-microphone-through-chain = Pasa tu micrófono por la cadena solo para los medidores: nada en los altavoces, así que no hay retardo contra el que hablar ni nada que se realimente. La forma de ajustar la ganancia del micrófono y de entrada, el umbral de la puerta y con cuánta fuerza trabaja el compresor. El audio de recepción continúa; pulsar PTT lo detiene.
txaudio-monitoring-live-use-headphones-speakers = Monitorización en vivo — usa auriculares, o los altavoces se realimentarán en el micrófono.
txaudio-voice-modes-only-tci-clients = Solo modos de voz. Los clientes TCI y los modos digitales puentean la cadena; cuando esté en uso, deja planos el ecualizador de TX y el procesador de la propia radio.
txaudio-auto-set-stopped-microphone-was = Ajuste automático detenido: se tomó el micrófono.
txaudio-microphone-may-clipping-before-gets = {" "}El micrófono puede estar recortando antes de llegar aquí — baja su propia ganancia si suena áspero.
txaudio-level = Nivel
txaudio-mic-control-same-one-transmit = El control Mic — el mismo que el de la barra de transmisión, compartido por todos los perfiles. 0 dB en el centro, +6 dB arriba.
txaudio-mic = Mic
txaudio-gain-top-mic-microphone-stored = Ganancia sobre Mic para este micrófono, guardada con el perfil. Los umbrales de los preajustes suponen una voz con picos en torno a -10 dBFS en el medidor IN; el ajuste automático la deja ahí. Se mantiene en PUENTEAR, y se aplica también a la voz digital: es nivel, no procesado.
txaudio-input = Entrada
txaudio-auto-set = Ajuste automático
txaudio-auto-set-cancelled = Ajuste automático cancelado.
txaudio-talk-now-you-would-air = Habla ahora, como lo harías en el aire…
txaudio-db = -∞ dB
txaudio-chain = Cadena
txaudio-order-audio-meets-them = En el orden en que el audio las encuentra
txaudio-switch-stage = Activar esta etapa
txaudio-earlier-chain = Antes en la cadena
txaudio-later-chain = Después en la cadena
txaudio-output-limiter = El limitador de salida
txaudio-output = Salida
txaudio-trim-limiter-always-last = Recorte y limitador — siempre al final
txaudio-profiles = Perfiles…
txaudio-gate-expander = Puerta / expansor
txaudio-pushes-room-noise-fan-rumble = Empuja hacia abajo el ruido de la sala, el rumor del ventilador y el siseo en las pausas. Fija el umbral justo por encima de tu suelo de ruido y por debajo de tus sílabas más suaves.
txaudio-gate = PUERTA
txaudio-hard-gate-10-1-down = Puerta dura: 10:1, hasta -40 dB
txaudio-expander = EXPANSOR
txaudio-gentle-expander-2-1-never = Expansor suave: 2:1, nunca más de -15 dB — la sala nunca se queda muerta
txaudio-reset = RESTABLECER
txaudio-back-defaults = Volver a los valores predeterminados
txaudio-threshold = Umbral
txaudio-below-gate-starts-close = Por debajo de esto la puerta empieza a cerrarse.
txaudio-return = Retorno
txaudio-hysteresis-once-open-closes-only = Histéresis: una vez abierta, solo se cierra a esta distancia por debajo del umbral, para que una voz que ronde el umbral no haga aletear la puerta.
txaudio-hold = Retención
txaudio-how-long-stays-open-after = Cuánto tiempo sigue abierta después de que la voz decae, para que no se recorten los finales de palabra.
txaudio-release = Liberación
txaudio-how-fast-closes-once-hold = Con qué rapidez se cierra una vez terminada la retención. Alárgala si la puerta bombea.
txaudio-floor = Suelo
txaudio-most-ever-turns-pauses-down = Lo máximo que baja las pausas.
txaudio-ratio = Relación
txaudio-how-steeply-closes-below-threshold = Con qué pendiente se cierra por debajo del umbral.
txaudio-look-ahead = Anticipación
txaudio-opens-much-before-word-starts = Abre con esta antelación antes de que empiece una palabra, para que su primera consonante no se corte. Añade el mismo retardo a todo lo transmitido.
txaudio-high-pass-low-shelf-two = Paso alto, shelf grave, dos picos, shelf agudo y paso bajo. Arrastra un punto de la curva, o fíjalo abajo. Fija el paso bajo al ancho de tu filtro de transmisión.
txaudio-flat = PLANO
txaudio-every-band-back-0-db = Todas las bandas de vuelta a 0 dB, ambos bordes desactivados
txaudio-freq = Frec.
txaudio-gain = Ganancia
txaudio-q-slope = Q / Pendiente
txaudio-high-pass = Paso alto
txaudio-removes-handling-noise-breath-pops = Elimina el ruido de manipulación, los golpes de aliento y el zumbido por debajo de la esquina.
txaudio-low-shelf = Shelf grave
txaudio-body-mud-cut-little-here = Cuerpo, o barro: corta un poco aquí para una voz que suena encajonada.
txaudio-peak-1 = Pico 1
txaudio-bump-dip-voice-band-often = Un realce o corte en la banda de la voz — a menudo un corte en torno a 300–500 Hz.
txaudio-peak-2 = Pico 2
txaudio-presence-lift-around-2-3 = Presencia: un realce en torno a 2–3 kHz es lo que se abre paso en BLU.
txaudio-high-shelf = Shelf agudo
txaudio-brightness = Brillo.
txaudio-low-pass = Paso bajo
txaudio-stops-energy-above-transmit-filter = Impide que la energía por encima del filtro de transmisión se gaste en el modulador.
txaudio-t-24-db-oct = 24 dB/oct
txaudio-t-12-db-oct = 12 dB/oct
txaudio-click-steeper-gentler-slope = Haz clic para una pendiente más fuerte o más suave
txaudio-de-esser = De-esser
txaudio-turns-down-only-sibilant-band = Baja solo la banda sibilante cuando «s» y «sh» cruzan el umbral, de modo que el resto de la voz conserva su nivel. Encuentra la frecuencia barriéndola mientras dices «sister silver cellophane» y aparcándola donde el siseo es más fuerte.
txaudio-frequency = Frecuencia
txaudio-centre-sibilant-band = Centro de la banda sibilante.
txaudio-width-band-higher-narrower = Ancho de la banda — más alto es más estrecho.
txaudio-band-level-where-reduction-starts = Nivel de la banda donde empieza la reducción. Demasiado bajo y trabaja todo el tiempo, apagando la voz.
txaudio-amount = Cantidad
txaudio-most-band-ever-turned-down = Lo máximo que se baja la banda. Empieza en torno a -6 dB.
txaudio-attack = Ataque
txaudio-how-fast-reacts = Con qué rapidez reacciona.
txaudio-how-fast-lets-go = Con qué rapidez suelta.
txaudio-compressor = Compresor
txaudio-brings-quiet-syllables-up-holds = Sube las sílabas suaves y contiene las fuertes. Apunta a 3–6 dB de reducción en los picos; más de 10 dB es una compresión evidente, más propia de concursos.
txaudio-level-above-which-starts-compress = Nivel por encima del cual empieza a comprimir.
txaudio-how-hard-compresses-above-threshold = Con qué fuerza comprime por encima del umbral.
txaudio-how-fast-reacts-peak-10 = Con qué rapidez reacciona a un pico. 10–25 ms mantiene nítidas las consonantes.
txaudio-how-fast-recovers = Con qué rapidez se recupera.
txaudio-knee = Rodilla
txaudio-soft-knee-width-0-hard = Ancho de la rodilla suave; 0 es una rodilla dura.
txaudio-makeup = Compensación
txaudio-gain-after-compression-bring-level = Ganancia tras la compresión, para devolver el nivel.
txaudio-drive = Excitación
txaudio-gain-ahead-compressor-so-more = Ganancia antes del compresor, para que más voz cruce el umbral — sube la potencia media. Combínala con el limitador de salida.
txaudio-phase = Fase
txaudio-all-pass-phase-rotator-ahead = Rotador de fase paso todo antes del detector. Iguala los picos asimétricos que tiene una voz, de modo que el mismo nivel de pico lleva más potencia media. 4 es un ajuste habitual en radiodifusión; 0 es desactivado.
txaudio-last-stage-trim-look-ahead = La última etapa: un recorte y un limitador con anticipación que mantiene cada pico bajo el techo sin las salpicaduras que produce el recorte duro. Desactivado de forma predeterminada, de modo que activar la cadena no cambia un nivel de excitación que ya hayas fijado.
txaudio-limiter = Limitador
txaudio-on = Activado
txaudio-ceiling = Techo
txaudio-no-peak-leaves-chain-above = Ningún pico sale de la cadena por encima de esto.
txaudio-trim = Recorte
txaudio-output-gain-ahead-limiter = Ganancia de salida, antes del limitador.
txaudio-off = {"  "}desactivado
txaudio-meters-move-while-you-transmit = Los medidores se mueven mientras transmites un turno de voz, reproduces la frase de prueba o ejecutas EN VIVO o MEDIDORES.
txaudio-profiles-2 = Perfiles
txaudio-profile-whole-chain-every-stage = Un perfil es la cadena entera: los ajustes de cada etapa, cuáles están activas, su orden y la ganancia de entrada de tu micrófono. Cargar uno sustituye la cadena; el control Mic y la excitación se quedan como están.
txaudio-name = Nombre
txaudio-save = Guardar
txaudio-store-chain-now-under-name = Guarda la cadena tal como está ahora con este nombre, sustituyendo un perfil del mismo nombre
txaudio-import = Importar…
txaudio-read-profile-json-file = Leer un perfil de un archivo .json
txaudio-tx-audio-profile = Perfil de audio TX
txaudio-no-profiles-yet-save-stores = Aún no hay perfiles. Guardar almacena la cadena que has configurado ahora.
txaudio-load = Cargar
txaudio-export = Exportar…
txaudio-delete = Eliminar
txaudio-starting-points = Puntos de partida
txaudio-set-your-level-auto-set = Fija primero tu nivel con el ajuste automático — los umbrales de los preajustes lo suponen — y luego afina desde aquí en lugar de transmitir tal como se cargan: graba una frase y escucha tras cada cambio. Un preajuste conserva tu ganancia de entrada.
txaudio-profile-loaded = Perfil cargado.

## Solar-system 3D view overlay (src/solar3d/overlay.rs)

solar-prop = PROP
solar-where-signals-are-actually-getting =
    Dónde llegan realmente las señales, banda por banda. Cada recepción en la que participa esta estación — WSPR, FT8/FT4, JS8 y el diario — se sitúa en el punto medio de su trayecto, que es el trozo de ionosfera que la desvió, y no en la estación lejana. Así que es un mapa del cielo, no de dónde viven los radioaficionados.

    Solo puede mostrar trayectos de los que esta estación ha sido uno de los extremos. Los océanos se iluminan porque los puntos medios de los trayectos largos caen ahí; la Antártida se queda oscura porque nadie transmite desde ella. Desactivado de forma predeterminada — pinta todo el planeta.
solar-orbits = ÓRBITAS
solar-orbital-paths = Trayectorias orbitales
solar-clouds = NUBES
solar-cloud-cover-noaa-s-global = Cobertura de nubes del mosaico geoestacionario global de la NOAA, dibujada como un espesor de aire y no como una imagen pegada a la superficie: el infrarrojo dice lo fría que está la cima de cada nube, y así de alta está. Las tormentas centellean dentro de las torres que forman.
solar-planets = PLANETAS
solar-other-seven-planets-eighteen-their = Los otros siete planetas, dieciocho de sus lunas y los anillos de Saturno y Urano. Las posiciones de los planetas proceden del conjunto de elementos keplerianos del JPL; las lunas son órbitas circulares ajustadas a JPL Horizons.
solar-coronal-mass-ejection-trajectory-cones = Conos de trayectoria de las eyecciones de masa coronal
solar-sun-obs = OBS. SOL
solar-solar-observations-sun-s-disk = Observaciones solares sobre el disco del Sol: regiones activas de manchas solares y de dónde vinieron las erupciones
solar-labels = ETIQUETAS
solar-body-region-labels = Etiquetas de cuerpos y regiones
solar-small-bodies = CUERPOS MENORES
solar-names-asteroids-comets-they-are = Nombres en los asteroides y cometas. Se dibujan en cualquier caso — esto es solo si se dibujan con ellos treinta y cinco designaciones. Sea cual sea el ajuste, el cuerpo en el que está la cámara y todo lo que haya encontrado el cuadro de búsqueda conservan su nombre.
solar-decoded-ft8-ft4-stations-path = Estaciones FT8/FT4 decodificadas y el trayecto hasta la estación con la que se trabaja
solar-sats = SATS
solar-amateur-radio-satellites-their-orbits = Satélites de radioaficionado, sus órbitas y su elevación desde tu QTH
solar-aurora = AURORA
solar-auroral-oval-noaa-s-ovation = El óvalo auroral del modelo OVATION de la NOAA, dibujado como capas de emisión a sus altitudes reales, con el borde hacia el ecuador del contorno del 10 % marcado en la superficie
solar-awards = DIPLOMAS
solar-award-coverage-every-dxcc-entity = Cobertura de diplomas: cada entidad DXCC pintada según lo que aún le falta a tu diario. Naranja arde donde nunca has trabajado la entidad, ámbar donde sí pero sin confirmar, verde tenue donde ha vuelto una QSL. Sigue el filtro de banda de la ventana DIPLOMAS.
solar-view = VISTA
solar-which-body-camera-orbits-animated = Alrededor de qué cuerpo orbita la cámara, y la visita animada
solar-layers = CAPAS
solar-what-drawn-orbits-clouds-cmes = Qué se dibuja: órbitas, nubes, CME, etiquetas, QSO, aurora
solar-which-sdo-channel-wraps-sun = Qué canal de SDO envuelve el Sol, y qué antigüedad tiene
solar-weather = METEO
solar-aurora-propagation-numbers-which-bands = La aurora, los números de propagación y qué bandas están abiertas
solar-space-weather = METEOROLOGÍA ESPACIAL
solar-nothing-measured-yet = aún no se ha medido nada
solar-scale = ESCALA
solar-body-moon-orbit-exaggeration = Exageración de los cuerpos y de la órbita de la Luna
solar-time = TIEMPO
solar-scrub-whole-scene-forward-back = Recorre toda la escena hacia delante y hacia atrás
solar-activity = ACTIVIDAD
solar-replay-last-hour-decodes-globe = Reproduce en el globo la última hora de decodificaciones
solar-what-propagation-heat-map-showing = Lo que muestra el mapa de calor de propagación
solar-propagation = PROPAGACIÓN
solar-auto = ▶ AUTO
solar-fly-spline-through-set-framed = Vuela por una spline a través de un conjunto de puntos de vista encuadrados. Cualquier entrada del ratón la cancela.
solar-moons-ride-circular-orbits-fitted = Las lunas siguen órbitas circulares ajustadas a JPL Horizons: a uno o dos grados de donde están realmente, y hasta seis para Miranda, cuyo plano orbital oscila demasiado deprisa para que un círculo lo siga.
solar-fetch-everything-again-now = Obtener todo de nuevo ahora
solar-all-sats = TODOS LOS SATS
solar-show-every-satellite-element-set = Muestra todos los satélites del conjunto de elementos, no solo los populares. Los anillos de órbita se quedan en los pocos curados — noventa a la vez son ilegibles.
solar-starting = iniciando…
solar-body = cuerpo{" "}
solar-moon-orbit = órbita lunar{" "}
solar-stretch-earth-moon-distance-so = Estira la distancia Tierra→Luna para que el par pueda verse por separado
solar-sun = sol{" "}
solar-sun-radius-exaggeration-leave-1 = Exageración del radio del Sol. Déjalo en 1× para que la geometría de las CME siga siendo legible — un Sol hinchado se traga la base de cada cono.
solar-show = MOSTRAR
solar-paint-globe-what-getting-through = Pinta el globo según lo que llega
solar-paints-globe-what-getting-through = Pinta el globo según lo que llega, en cada banda. Cada recepción se sitúa en el punto medio de su trayecto — donde hizo su trabajo la ionosfera — y no en la estación lejana. Se construye con lo que oye esta estación, más los skimmers del mundo cuando la Reverse Beacon Network está activada.
solar-display = PANTALLA
solar-all-bands = TODAS LAS BANDAS
solar-every-band-once-one-hue = Todas las bandas a la vez, un tono cada una, mezclándose donde dos se solapan. La vista para «cómo están las condiciones» más que para «está abierta la de veinte».
solar-one-band = UNA BANDA
solar-one-band-cold-hot-blue = Una banda, de frío a calor: azul, verde, amarillo, rojo.
solar-band = BANDA
solar-nothing-heard-yet = aún no se ha oído nada
solar-bands = BANDAS
solar-sources = FUENTES
solar-contacts-logbook-path-was-open = Contactos en el diario. Un trayecto que estuvo abierto, sin un informe de señal que merezca ese nombre — un RST no es una SNR, así que estos cuentan para lo concurrida que está una celda y nunca para su margen.
solar-stations-reported-hearing-one-downloaded = Estaciones que informaron de haber oído esta, descargadas de WSPRnet. La única respuesta que recibe una baliza transmisora.
solar-reverse-beacon-network-skimmers-what = Skimmers de la Reverse Beacon Network: lo que oyen todos los demás, en todas las bandas, incluidas aquellas en las que no está esta radio. Más grueso que el resto — una línea de RBN no lleva locátores, así que ambos extremos se sitúan en el centro de su país, lo que es cercano para un país pequeño y muy desviado para uno grande. Se activa en Ajustes → Spots.
solar-decodes-station-made-each-measured = Decodificaciones que hizo esta estación. Cada una se mide contra el suelo de su propio modo, de modo que un informe de WSPR y uno de FT4 son comparables.
solar-memory = MEMORIA
solar-half-life = semivida{" "}
solar-how-long-reception-takes-count = Cuánto tarda una recepción en contar la mitad. La memoria propia de la ionosfera es corta — una apertura de hace dos horas no debería discutir con una de hace dos minutos.
solar-live = EN VIVO
solar-follow-band-happens = Seguir la banda a medida que ocurre
solar-replay = ⏸ REPETICIÓN
solar-replay-2 = ▶ REPETICIÓN
solar-replay-last-hour-decodes-over = Reproduce la última hora de decodificaciones, una y otra vez
solar-min-ago = {" "}min atrás
solar-where-last-hour-globe-showing = En qué punto de la última hora está el globo
solar-trail = estela{" "}
solar-how-long-decode-s-arc = Cuánto tiempo permanece en el globo el arco de una decodificación tras la cabeza
solar-speed = veloc.{" "}
solar-how-much-faster-than-real = Cuánto más rápido que el tiempo real corre la repetición
solar-qso-layer-off = Capa de QSO desactivada
solar-no-decodes-yet = aún no hay decodificaciones
solar-qth-not-set-enter-your = QTH sin fijar — introduce tu locátor en Ajustes
solar-mode = MODO
solar-path = TRAYECTO
solar-sent = ENV.
solar-rcvd = REC.
solar-grid = LOCÁT.
solar-dxcc = DXCC
solar-geostationary-always-above-your-horizon = Geoestacionario — siempre sobre tu horizonte.
solar-no-passes-next-48-hours = No hay pasos en las próximas 48 horas desde tu QTH.
solar-start = INICIO
solar-max-el = EL MÁX
solar-aos-los-are-azimuths-horizon = AOS/LOS son los acimuts en el horizonte. Las horas son UTC.
solar-locked-unlock = {" "}● ENGANCHADO — SOLTAR{" "}
solar-release-satellite-lock = Soltar el enganche del satélite
solar-lock = {" "}ENGANCHAR{" "}
solar-track-satellite-main-window-doppler = Sigue este satélite en la ventana principal: RX y TX corregidos por Doppler, enlace ascendente mapeado al transpondedor, orientación del rotor
solar-no-frequencies-file-one-add = No hay frecuencias guardadas para este — añádelas en Ajustes ▸ TLE.
solar-frequencies-yours = FRECUENCIAS · LAS TUYAS
solar-frequencies = FRECUENCIAS
solar-link = ENLACE
solar-downlink-mhz = DESCENDENTE (MHz)
solar-uplink-mhz = ASCENDENTE (MHz)
solar-doppler-shifts-these-few-khz = El Doppler desplaza estas unos pocos kHz a lo largo de un paso LEO.
solar-sim = -- SIM
solar-satellite-planet-comet = satélite, planeta o cometa
solar-planet-asteroid-comet = planeta, asteroide o cometa
solar-clear-search = Borrar la búsqueda
solar-no-match = sin coincidencias
solar-no-sounder = sin sondeador
solar-heard = OÍDAS ≥
solar-kp = Kp / A
solar-x-ray = Rayos X
solar-observed-above-sounder-better-than = observado sobre el sondeador — mejor que lo modelado
solar-power-n-s = potencia N/S
solar-edge-n-s = borde N/S
solar-kp-peak-24-h = Kp máx. 24 h
solar-observed-forecast = observado | pronóstico
solar-bands-open = BANDAS ABIERTAS
solar-dxcc-coverage = COBERTURA DXCC
solar-ir-vis = IR+VIS
solar-ir-only = solo IR
solar-no-deep-convection = sin convección profunda
solar-t-1-storm = 1 tormenta
solar-glancing = {" "}· de refilón
solar-direction-estimated = {" "}· dirección estimada

## Digital-mode setup dialogs (src/app/panels/setup.rs)

setup-aprs-setup = Configuración de APRS
setup-packet-setup = Configuración de Packet
setup-ft8-ft4-ft2-setup = Configuración de FT8 / FT4 / FT2
setup-my-callsign = Mi indicativo
setup-my-grid = Mi locátor
setup-station-call = Indicativo de la estación
setup-ssid-oe3jjs-10-packet-station = Con un SSID — OE3JJS-10. Una estación de packet suele tener un SSID distinto del del operador, y no se transmite nada hasta que esto esté fijado.
setup-speed = Velocidad
setup-t-9600-needs-radio-s-data = 9600 necesita el puerto de datos de la radio. Un camino de micrófono y altavoz lo destruye en ambos extremos — eso es la radio, no sdroxide.
setup-tx-delay = Retardo TX
setup-flags-sent-before-frame-so = Banderas enviadas antes de una trama, para que el otro extremo nos oiga y enganche su reloj. En un equipo CAT, solo sdroxide gasta 165–240 ms en salir al aire, y el propio tiempo de transmisión listo del equipo se suma a eso.
setup-packet-length = Longitud del paquete
setup-bytes = {" "}bytes
setup-most-single-frame-carries-shorter = Lo máximo que lleva una sola trama. Las tramas más cortas sobreviven a un trayecto marginal — solo se reenvía la trama que recibió el golpe — y cuestan más sobrecarga. 128 en HF, 256 donde el trayecto es sólido.
setup-frames-sent-before-waiting-acknowledgement = Tramas enviadas antes de esperar un acuse de recibo. Más llena un buen trayecto; en uno malo es más para reenviar.
setup-extended = Extendido
setup-ask-mod-128 = Pedir mod-128
setup-extended-sequence-numbers-window-bigger = Números de secuencia extendidos, para una ventana mayor de siete. Muchos nodos rechazan la petición con un DM, lo que parece exactamente una estación que no quisiera hablar contigo — así que déjalo desactivado salvo que sepas que el otro extremo lo quiere.
setup-answer-calls = Responder llamadas
setup-accept-connections = Aceptar conexiones
setup-off-winlink-client-which-dials = Desactivado para un cliente de Winlink, que llama hacia fuera y no tiene motivo para responder. Activado para ser alcanzable como par: una llamada llega al panel de terminal y hablas con quien la hizo. Una estación cuyo enlace ya está ocupado con una sesión de Winlink rechaza llamadas hasta que esta termine.
setup-connect-text = Texto de conexión
setup-what-caller-greeted = con qué se saluda a quien llama
setup-sent-station-connects-us-once = Se envía a una estación que conecta con nosotros, una vez establecido el enlace. Vacío no envía nada — lo que parece una avería a quien llamó.
setup-default-via = Vía predeterminada
setup-oe3xlr-1-oe3xms-1 = OE3XLR-1,OE3XMS-1
setup-digipeater-path-terminal-starts-path = La ruta de digipetidores con la que arranca el terminal. La ruta hasta tu nodo local es la misma cada vez, y volver a teclearla es como se deja fuera un salto.
setup-beacon = Baliza
setup-t-0-disables-timer = 0 desactiva el temporizador
setup-beacon-text = texto de la baliza
setup-kiss-server = Servidor KISS
setup-serve = Servir
setup-offer-modem-kiss-tnc-socket = Ofrece este módem como un TNC KISS en un socket, para que Pat, un cliente APRS o la pila AX.25 de Linux puedan usar la radio.
setup-aprs-call = Indicativo APRS
setup-callsign-station-beacons-under-ssid = El indicativo con el que esta estación emite su baliza, con su SSID — OE3JJS-9 para un coche, -10 para un I-gate, -5 para un teléfono. Déjalo vacío para usar el indicativo de arriba.
setup-no-callsign-station-will-not = Sin indicativo: esta estación no transmitirá nada — ni una baliza, ni un mensaje, ni un acuse de recibo.
setup-symbol = Símbolo
setup-two-characters-protocol-carries-them = Los dos caracteres tal como los lleva el protocolo: una tabla (`/` o `\`) y un código de símbolo. Un dígito o una letra en la primera posición es una superposición, dibujada encima del icono de la tabla alternativa.
setup-path = Ruta
setup-wide1-1-wide2-1 = WIDE1-1,WIDE2-1
setup-how-far-you-ask-repeated = Hasta dónde pides que te repitan. Este es el ajuste de más consecuencias del canal: cada salto multiplica las transmisiones que hace toda la red en una frecuencia compartida.
setup-position = Posición
setup-my-grid-2 = Desde mi locátor
setup-beacon-centre-locator-above-reported = Emite como baliza el centro del locátor de arriba, informado con la ambigüedad que realmente tiene un locátor — uno de seis caracteres tiene un par de kilómetros de lado, y decirlo es honesto. Desmarca para dar coordenadas exactas en su lugar.
setup-latitude = Latitud
setup-longitude = Longitud
setup-locator = desde el locátor
setup-not-coordinate = no es una coordenada
setup-comment = Comentario
setup-sent-every-beacon = enviado con cada baliza
setup-t-0-default-never-beacons-timer = 0 — el valor predeterminado — nunca emite baliza con un temporizador. Treinta minutos es la convención para una estación fija; una en movimiento emite más a menudo, pero cada baliza es tiempo de canal de otro.
setup-compressed = Comprimido
setup-compressed-position-format-third-air = El formato de posición comprimido: una tercera parte del tiempo en el aire y más preciso. Todo receptor desde los años 90 lo lee.
setup-first-goes-out-one-interval = La primera sale un intervalo después de ahora, no de inmediato.
setup-messages = Mensajes
setup-acknowledge = Acusar recibo
setup-answer-messages-addressed-you-acknowledgement = Responde a los mensajes dirigidos a ti. Un acuse de recibo es una transmisión que hace esta estación sin que se la pidan, así que una configuración solo de recepción debería desactivarlo — y entonces la baliza también deja de afirmar que es alcanzable.
setup-flags-sent-ahead-every-frame = Banderas enviadas antes de cada trama, para que el módem del otro extremo oiga la portadora y enganche su reloj antes de que empiecen los datos. Tiene que durar más que todo lo que hay entre pulsar transmitir y radiar: solo sdroxide gasta 165–240 ms de ello en un equipo CAT, y una radio que toma el audio por una red almacena más aún. Demasiado poco y el otro extremo nunca engancha — la transmisión está ahí y nada la decodifica.
setup-shared-packet-mode = compartido con el modo packet
setup-keep-stations = Conservar estaciones
setup-how-long-station-stays-map = Cuánto tiempo permanece una estación en el mapa tras oírse por última vez. También es contra lo que se mide el desvanecimiento del mapa, así que una ventana corta muestra solo lo que está en vivo.
setup-auto-reply = Respuesta automática
setup-answer-snr-grid-status = Responder SNR? / GRID? / STATUS?
setup-answer-direct-question-addressed-you = Responde a una pregunta directa dirigida a ti o a @ALLCALL, con la respuesta y no con un acuse de recibo. Nunca responde al tráfico de otra estación, y nunca se responde a sí misma.
setup-heartbeat = Heartbeat
setup-off = Desactivado
setup-turbo-does-not-beacon-local = Turbo no emite baliza — es la velocidad local y de VHF.
setup-sends-your-callsign-grid-so = Envía tu indicativo y locátor para que los demás sepan que eres recibible. El primero sale un intervalo después de ahora, no de inmediato.
setup-beacon-frequency = Frecuencia de la baliza
setup-t-500-1000-hz = 500–1000 Hz
setup-move-each-beacon-free-slot = Mueve cada baliza a una ranura libre de la subbanda de heartbeat, como hace JS8Call: es donde miran las estaciones que vigilan balizas, y mantiene un transmisor desatendido fuera del QSO de otro. La ranura se elige cuando la baliza sale realmente, lejos de todo lo que se está decodificando.
setup-working-freq = Frec. de trabajo
setup-beacon-where-you-are-working = Emite la baliza donde estás trabajando. Va contra la convención de la banda, pero mantiene todo lo que transmites en un solo lugar.
setup-heartbeat-reply = Respuesta al heartbeat
setup-answer-heartbeats-signal-report = Responder a los heartbeats con un informe de señal
setup-tell-station-beaconed-how-well = Dile a una estación que emitió baliza lo bien que la copiaste. Desactivado de forma predeterminada: una banda concurrida lleva un heartbeat en cada ranura, y responder a todos inundaría justo la banda que los heartbeats existen para mantener en calma. Limitado a una respuesta por estación cada 15 minutos, y nunca mientras sigue llegando un mensaje o mientras tienes algo en cola para enviar.
setup-status-message = Mensaje de estado
setup-tx-period = Periodo de TX
setup-even = Par
setup-odd = Impar
setup-auto-sequence = Secuencia automática
setup-decode-depth = Profundidad de decodificación
setup-fast = Rápida
setup-normal = Normal
setup-deep = Profunda
setup-how-hard-ft8-decoder-works =
    Cuánto se esfuerza el decodificador de FT8 por las señales débiles.

    Rápida — una pasada, sin sustracción de señal: la más veloz, con menos decodificaciones.
    Normal — sustracción multipasada plana: algo más rápida que Profunda, algo menos exhaustiva.
    Profunda — la multipasada con puntos de control: la que más decodifica, y ~1,2 s en una ranura concurrida.
setup-auto-tx-frequency = Frecuencia TX automática
setup-choose-transmit-frequency-automatically-quietest = Elige la frecuencia de transmisión automáticamente: el punto más tranquilo del periodo en que transmites, en lugar de la frecuencia de la estación a la que respondes. Desactivado NO la mantiene — responde en la frecuencia de la estación a la que se llama. Para mantenerla, usa Mantener frecuencia TX.
setup-hold-tx-frequency = Mantener frecuencia TX
setup-pin-transmit-tone-where-nothing = Fija el tono de transmisión donde está. Nada lo mueve: ni responder a una estación, ni la cola de llamadas, ni llamar CQ, ni un clic en una decodificación o en la cascada. Anula la Frecuencia TX automática. Cambiar de banda es la única excepción: el desplazamiento que fijaste por última vez en la banda nueva vuelve con ella. Para donde tu licencia es más estrecha que el plan de bandas — con un dial de 60 m del Reino Unido de 5357 kHz la asignación termina en 5358,0, así que el tono debe quedarse por debajo de 1000 Hz.
setup-tx-watchdog = Vigilante de TX
setup-stop-transmitting-after-long-no = Deja de transmitir tras este tiempo sin respuesta y sin ninguna acción tuya. 0 lo desactiva.
setup-give-up-after = Abandonar tras
setup-calls = {" "}llamadas
setup-unanswered-calls-one-station-before = Llamadas sin respuesta a una estación antes de pasar a otra. Llamar CQ está exento. 0 lo desactiva.
setup-contest = Concurso
setup-everyday-exchange-grid-report-rr73 = El intercambio de cada día: locátor, informe, RR73.
setup-european-vhf-contests-send-signal = Concursos europeos de VHF: envía un informe de señal, un número de serie y tu locátor de SEIS caracteres, en la disposición de mensaje que WSJT-X llama EU VHF Contest. El CQ sale como CQ TEST.
setup-serial = Serie
setup-serial-number-next-exchange-will = El número de serie que llevará el próximo intercambio. Avanza solo cada vez que se registra un contacto y se recuerda entre sesiones — fíjalo a mano solo para empezar un concurso, o para retomarlo donde llegó un registro en papel.
setup-needs-6-character-grid = ⚠ necesita un locátor de 6 caracteres
setup-eu-vhf-contest-messages-carry = Los mensajes de EU VHF Contest llevan un locátor de seis caracteres (JN88DD, no JN88). Mientras Mi locátor no tenga uno, se envía en su lugar el intercambio normal.
setup-dxpedition = DXpedición
setup-ordinary-ft8-operation = Operación FT8 normal.
setup-calling-dxpedition-running-fox-mode = Llamando a una DXpedición que usa el modo Fox: llama por encima de 1000 Hz, baja hasta el Fox cuando responda, y registra con su RR73 sin enviar 73.
setup-run-pile-up-several-signals = Dirige el pile-up: varias señales a la vez, una cola de llamantes, trabajados primero los más fuertes y raros. LLAMAR CQ lo inicia, DETENER QSO lo para.
setup-fox-signals = Señales Fox
setup-once = {" "}a la vez
setup-simultaneous-transmissions-spaced-60-hz = Transmisiones simultáneas, separadas 60 Hz. Comparten la potencia del transmisor, así que más señales significa que cada una es más débil.
setup-tx-audio = Audio TX
setup-how-loud-mode-s-burst =
    Con qué fuerza se entrega la ráfaga de este modo a una radio que la modula por sí misma — y en FM eso es la desviación, porque un transmisor FM convierte el nivel de audio en excursión de frecuencia y no tiene ALC que la frene. El packet a 1200 baudios quiere unos 3 kHz donde la voz quiere 5, así que el fondo de escala en una entrada de datos ajustada para voz sobredesvía — lo que suena completamente normal para un oyente y no decodifica para nadie. Bájalo hasta que otras estaciones te reciban, o fija el nivel en la radio.

    Este nivel pertenece solo a este modo. Un modo que nunca has ajustado parte de la cifra de FM, y los modos de banda lateral parten de la suya — así que una desviación fijada para 1200 baudios nunca cae sobre FT8.
setup-how-loud-mode-s-over =
    Con qué fuerza se entrega el turno de este modo a una radio que lo modula por sí misma — un equipo CAT en su tarjeta de sonido, una FLEX, un Icom por su puerto de red. En banda lateral esto es excitación del modulador: bájala hasta que el ALC del equipo apenas se mueva y fija la potencia en la radio, porque un ALC trabajando sobre un modo de envolvente constante es lo que genera salpicaduras. La Excitación llega aquí al registro de potencia del equipo, no a su audio, así que este es el nivel.

    Este nivel pertenece solo a este modo: FT8, RTTY, PSK y MCW guardan cada uno el suyo, porque no cargan un modulador de la misma manera. El mismo carril está en la franja de transmisión, donde puedes alcanzarlo mientras transmites.
setup-deviation-fm = desviación, en FM
setup-drive-ssb = excitación, en BLU
setup-grid = Locátor
setup-report = Informe
setup-r-report = R+Informe

## AMP window (src/app/amp.rs)

amp-t-10-w-scale-rescaled = , escala de 10 W, reescalada
amp-t-10-w-scale = , escala de 10 W
amp-t-2-w-scale-rescaled = , escala de 2 W, reescalada
amp-t-2-w-scale = , escala de 2 W
amp-t-100-w-scale-rescaled = , escala de 100 W, reescalada
amp-t-100-w-scale = , escala de 100 W
amp-t-20-w-scale-rescaled = , escala de 20 W, reescalada
amp-t-20-w-scale = , escala de 20 W
amp-rescaled = , reescalada
amp-and = {" "}Y{" "}
amp-not-while-transmitting-switching-relay = No mientras se transmite: conmutar un relé con RF encima puede dañarlo. Suelta primero el PTT.
amp-no-amplifier-set-up-choose = No hay ningún amplificador configurado. Elige su puerto en Ajustes → Amplificador.
amp-settings = AJUSTES
amp-dock = ACOPLAR
amp-put-amp-window-back-inside = Devuelve la ventana AMP al interior de la ventana principal
amp-pop-out = SEPARAR
amp-give-amp-window-window-own = Da a la ventana AMP una ventana propia, para llevarla a otra pantalla
amp-rig = , del equipo
amp-rig-flagged-reading = , el equipo marcó su lectura
amp-drv = , de DRV
amp-rig-s-swr = , de la ROE del equipo
amp-ptt-keys-amplifier-radio-off = PTT activa el amplificador con la radio; OFF lo deja en bypass. Haz clic para conmutar.
amp-switch-antenna-jack = Cambiar el conector de antena
amp-band-down = Banda abajo
amp-band-up = Banda arriba
amp-tuner = ACOPLADOR
amp-tuned = Sintonizado
amp-tune = TUNE
amp-start-tune-then-key-steady = Inicia una sintonización — y luego activa una portadora constante (CW, FM o RTTY). Pulsa de nuevo para detenerla.
amp-put-tuner-line-bypass = Poner el acoplador en línea, o puentearlo
amp-tuning = Sintonizando…
amp-bottom = ABAJO
amp-pick-any-reading-either-needle = Elige cualquier lectura para cada aguja. La aguja superior es blanca, la inferior azul.
amp-forward-power-measured-radio-amplifier = Potencia directa, medida por la radio: el amplificador no informa de ninguna en OFF
amp-forward-power-taken-drv-amplifier = Potencia directa, tomada de DRV: el amplificador no informa de ninguna en OFF, y en bypass la potencia de la radio pasa directa
amp-forward-power = Potencia directa
amp-reflected-power-measured-radio-amplifier = Potencia reflejada, medida por la radio: el amplificador no informa de ninguna en OFF
amp-reflected-power-worked-out-fwd = Potencia reflejada, calculada a partir de FWD y la ROE: el amplificador no informa de ninguna en OFF
amp-reflected-power = Potencia reflejada
amp-standing-wave-ratio-red-past = Relación de onda estacionaria: rojo en el límite de sintonía o más allá (Ajustes → General). Revisa la antena
amp-standing-wave-ratio-measured-radio = Relación de onda estacionaria, medida por la radio: el amplificador no informa de ninguna en OFF
amp-standing-wave-ratio = Relación de onda estacionaria
amp-drive-radio = Excitación desde la radio
amp-drain-supply = Alimentación de drenador
amp-drain-current = Corriente de drenador
amp-temp = TEMP
amp-heat-spreader-temperature-yellow-49 = Temperatura del disipador: amarillo desde 49 °C (120 °F), rojo por encima de 65 °C (149 °F), fuera de línea por encima de 70 °C (158 °F). Haz clic para °C/°F.
amp-c = °C
amp-f = °F
amp-performance = RENDIMIENTO
amp-numbers-hold-peaks-bars-are = los números retienen los picos · las barras son en vivo
amp-input-power = POTENCIA DE ENTRADA
amp-supply-volts-drain-amps = Voltios de alimentación × amperios de drenador
amp-efficiency = EFICIENCIA
amp-output-power-input-power = Potencia de salida ÷ potencia de entrada
amp-gain = GANANCIA
amp-output-power-over-drive-decibels = Potencia de salida sobre la excitación, en decibelios
amp-off-line-over-temperature = FUERA DE LÍNEA · exceso de temperatura
amp-connected-transmitting = CONECTADO · TRANSMITIENDO
amp-connected = CONECTADO
amp-connecting = Conectando…
amp-atu-bypassed = ATU PUENTEADO
amp-no-inductance-needed = no hace falta inductancia
amp-no-capacitance-needed = no hace falta capacidad
amp-c-out-antenna-side = C SALIDA · lado antena
amp-c-transmitter-side = C ENTRADA · lado transmisor

## SSTV panel (src/app/panels/sstv.rs)

sstv-chunks-received-lit-still-missing = Fragmentos recibidos (iluminados) y aún por recibir (oscuros)
sstv-sstv = SSTV
sstv-auto = Auto
sstv-signal = Señal
sstv-station-sending-mode-sdroxide-does = Una estación está enviando en un modo que sdroxide no tiene. La señal y la sintonía están bien — simplemente no hay decodificador para este. No hay nada que arreglar por tu parte.
sstv-listening = escuchando…
sstv-restart-rx = Reiniciar RX
sstv-abandon-picture-being-received-listen = Abandona la imagen que se está recibiendo y escucha la siguiente cabecera. Para una transmisión que empezó a decodificarse en el modo equivocado — el receptor queda comprometido hasta que ese modo se agota.
sstv-callsign-last-station-sent-fsk = El indicativo que envió la última estación como ID FSK tras su imagen.
sstv-tx-slant = Inclinación TX
sstv-transmit-clock-trim-ppm-remove = Ajuste del reloj de transmisión (ppm) para eliminar la inclinación en el decodificador del otro extremo
sstv-reset-0-ppm = Restablecer a 0 ppm
sstv-fsk-id = ID FSK
sstv-send-your-callsign-tones-after = Envía tu indicativo en tonos tras cada imagen — la identificación que leen los repetidores SSTV y otros programas. Añade unos 2,5 segundos, y no envía nada en absoluto hasta que hayas fijado un indicativo.
sstv-tx-lead = Antelación TX
sstv-silence-sent-after-keying-before = Silencio enviado tras activar y antes de la cabecera y el código VIS de la imagen. Un decodificador que pierda cualquier parte de esa cabecera no muestra imagen alguna, así que esto cubre el hueco entre pedir el PTT a un equipo y que esté realmente en el aire. 0 para un SDR que se activa al instante.
sstv-live = EN VIVO
sstv-waiting-picture = esperando una imagen…
sstv-waiting-signal = esperando una señal…
sstv-no-low-audio = sin audio / poco audio
sstv-received = RECIBIDAS
sstv-reading-radio-s-pictures = Leyendo las imágenes de la radio…
sstv-decoded-pictures-collect-here = Las imágenes decodificadas se acumulan aquí.
sstv-click-enlarge-right-click-delete = Clic para ampliar · clic derecho para eliminar
sstv-delete-picture = Eliminar esta imagen
sstv-loading = cargando…
sstv-transmit = TRANSMITIR
sstv-click-edit-slot-s-message = Clic para editar el mensaje de esta ranura · doble clic para cargar una imagen
sstv-change-image = Cambiar imagen…
sstv-load-image = Cargar imagen…
sstv-clear = Borrar
sstv-empty-slot-s-picture-message = Vacía la imagen de esta ranura (el mensaje se queda)
sstv-banner = Banner…
sstv-what-printed-across-top-every = Lo que se imprime en la parte superior de cada imagen que envía esta estación
sstv-fetching-slot-s-picture = Obteniendo la imagen de esta ranura…
sstv-load-image-into-slot = Cargar una imagen en esta ranura →
sstv-drawn-slot-s-image = Dibujado en la imagen de esta ranura
sstv-tx = {"   "}TX{"   "}
sstv-abort-tx = {" "}ABORTAR TX{" "}
sstv-delete-picture-store = Eliminar esta imagen del almacén
sstv-received-image = Imagen recibida
sstv-no-longer-store = ya no está en el almacén
sstv-loading-full-size = cargando a tamaño completo…
sstv-save-picture = Guardar imagen…
sstv-save-copy-computer = Guardar una copia en este ordenador
sstv-delete-sure = Eliminar — ¿seguro?
sstv-click-again-delete-good = Haz clic de nuevo para eliminarla definitivamente
sstv-delete = Eliminar…
sstv-image-banner = Banner de la imagen
sstv-drawn-into-every-picture-station = Se dibuja en cada imagen que transmite esta estación, sobre el mensaje propio de la ranura.
sstv-draw-banner = Dibujar el banner
sstv-substituted-when-picture-composed = Se sustituye al componer la imagen:
sstv-top-left = Arriba a la izquierda
sstv-top-right = Arriba a la derecha
sstv-colours = Colores
sstv-strip-top-edge-fades-black = La franja, en su borde superior — se desvanece a negro, o al color del degradado de debajo
sstv-both-texts = Ambos textos
sstv-gradient = Degradado
sstv-fade = desvanecer a
sstv-fade-strip-top-colour-second = Desvanece la franja de su color superior a un segundo color en lugar de a negro.
sstv-text-outline = Contorno del texto
sstv-draw-outline-around-banner-text = Dibuja un contorno alrededor del texto del banner, para que siga siendo legible sobre una franja cargada o clara.
sstv-text-gradient = Degradado del texto
sstv-fade-banner-text-colour-second = Desvanece el texto del banner de su color a un segundo color a lo ancho de la imagen.
sstv-rainbow-text = Texto arcoíris
sstv-all-text-overriding-colours-above = todo el texto, anulando los colores de arriba
sstv-draw-every-text-picture-banner = Dibuja todos los textos de la imagen — banner y mensaje — como un arcoíris horizontal, ignorando los colores de arriba.
sstv-message = Mensaje
sstv-slot-message-s-text-colour = El color del texto del mensaje de la ranura
sstv-outline = contorno en
sstv-height = Altura
sstv-how-tall-strip-pixels-transmitted = Cuánto mide de alto la franja, en píxeles de la imagen transmitida. El texto se dimensiona a partir de ella — un fotograma SSTV tiene solo 320 píxeles de ancho, así que un banner más alto es lo que lo hace legible en el otro extremo.
sstv-reset = Restablecer
sstv-back-callsign-left-version-right = Volver al indicativo a la izquierda y la versión a la derecha
sstv-waiting-station-s-digital-mode = Esperando a que se carguen los ajustes de modo digital de la estación…
sstv-rifp = RIFP
sstv-size = Tamaño
sstv-calling-frequency-draft-names = La frecuencia de llamada que nombra el borrador
sstv-encode = Codificar
sstv-how-picture-encoded-into-object = Cómo se codifica la imagen en el objeto que lleva RIFP. Auto prueba cada una y envía la más pequeña.
sstv-try-every-encoding-send-smallest = Prueba todas las codificaciones, envía la más pequeña (nunca con pérdidas)
sstv-gray = Gris
sstv-grayscale-depth-rifp-s-raster = Profundidad de escala de grises. El ráster de RIFP es de escala de grises por definición — el color no tiene sitio en su manifiesto.
sstv-dither = Dither
sstv-diffuse-quantisation-error-worth-below = Difunde el error de cuantización — merece la pena por debajo de 8 bits
sstv-caption = Leyenda
sstv-sent-manifest-content-hint-shown = Se envía en el manifiesto como indicación del contenido, y los receptores la muestran bajo la imagen. Viaja como texto — no se dibuja en la imagen.
sstv-what-picture = Qué es esta imagen
sstv-repeat-data = Repetir datos
sstv-send-every-data-frame-many = Envía cada trama de datos este número de veces. RIFP es unidireccional sin peticiones de reparación, así que esta es la única recuperación que tiene un receptor.
sstv-chunk = Fragmento
sstv-payload-octets-per-data-frame = Octetos de carga útil por trama de datos (el perfil recomienda 192)
sstv-valid-frames-frames-failed-crc = Tramas válidas, tramas que fallaron el CRC e imágenes completas verificadas
sstv-no-transfer-progress = ninguna transferencia en curso
sstv-forget-incomplete-transfer = Olvidar esta transferencia incompleta

## Memories and voice keyer windows (src/app/windows.rs)

windows-recall = Recuperar
windows-delete = Eliminar
windows-edit-name-frequency-mode-antenna = Edita el nombre, la frecuencia, el modo, la antena y la configuración de repetidor
windows-r = {" "}R
windows-repeater-shift-tone-1750-hz = El desplazamiento de repetidor, el tono y el tono de 1750 Hz guardados con este canal
windows-abandon-edit-escape = Abandonar la edición (Escape)
windows-save = GUARDAR
windows-keep-changes-enter = Conservar los cambios (Intro)
windows-recall-channel-without-moving-antenna = Recuperar este canal sin mover la antena
windows-antenna-switch-when-channel-recalled = La antena a la que cambiar cuando se recupera este canal. — deja lo que tenga ya la radio.
windows-shift = Desplaz.
windows-tone = Tono
windows-invert = INVERTIDO
windows-normal = NORMAL
windows-dcs-polarity = Polaridad DCS
windows-open-every-over-tone-burst = Abrir cada turno con el tono mientras este canal está recuperado
windows-memories = Memorias
windows-memory-name = nombre de la memoria
windows-store = Guardar
windows-folder-name = nombre de la carpeta
windows-new-folder = Nueva carpeta
windows-import = IMPORTAR
windows-read-channel-list-chirp-csv =
    Lee una lista de canales de un archivo CSV de CHIRP (.csv) — una exportación de un directorio de repetidores, una tabla de canales marinos o PMR, o cualquier otra cosa que CHIRP pueda escribir.

    Cada canal trae consigo su frecuencia, modo, desplazamiento de repetidor y tono CTCSS o DCS. Los canales que ya están en esta lista — misma frecuencia, mismo modo — se omiten, así que volver a importar un directorio actualizado añade lo nuevo en lugar de duplicar lo que no lo es.
windows-chirp-csv = CSV de CHIRP
windows-export = EXPORTAR
windows-write-list-out-chirp-csv = Escribe esta lista como un archivo CSV de CHIRP, para cargarlo en un portátil o conservarlo como copia de seguridad.
windows-no-memories-yet = aún no hay memorias
windows-drag-memory-onto-folder-file = arrastra una memoria a una carpeta para archivarla, debajo de ellas para desarchivarla
windows-drop-here-unfile = suelta aquí para desarchivar
windows-sort = Ordenar
windows-order-they-were-stored = En el orden en que se guardaron
windows-name-ignoring-case = Por nombre, sin distinguir mayúsculas
windows-frequency = Por frecuencia
windows-band-then-frequency-inside-each = Por banda, y luego por frecuencia dentro de cada una
windows-descending-click-ascending = Descendente — haz clic para ascendente
windows-ascending-click-descending = Ascendente — haz clic para descendente
windows-delete-folder-memories-move-top = Eliminar carpeta — sus memorias pasan al nivel superior
windows-rename-folder = Renombrar carpeta
windows-empty-drop-memories-here = vacía — suelta memorias aquí
windows-voice-keyer = Manipulador de voz
windows-rec-records-your-microphone-play = GRABAR graba desde tu micrófono, REPRODUCIR te deja escuchar lo que grabaste, TX lo pone en el aire — igual que una tecla del teclado numérico, un pad MIDI o el send_voice_mem de rigctld.
windows-stop-store = Parar y guardar
windows-stop = PARAR
windows-play = REPRODUCIR
windows-stop-listening = Dejar de escuchar
windows-nothing-recorded-slot = No hay nada grabado en esta ranura
windows-not-while-transmitting = No mientras se transmite
windows-listen-message-nothing-transmitted = Escucha este mensaje — no se transmite nada
windows-stop-transmitting = Dejar de transmitir
windows-tune-active-switch-off-first = TUNE está activo — desactívalo primero
windows-transmit-message = Transmitir este mensaje
windows-erase-recording = Borrar esta grabación
windows-rade-message-encoded-digital-voice = RADE: el mensaje lo codifica el códec de voz digital, exactamente como lo haría un turno en vivo.

## TX Audio window: messages with values

txaudio-rec-hint = Graba una frase de prueba del micrófono — hasta { $max } s, sin activar el transmisor. Haz clic de nuevo para parar. Se conserva sin procesar, de modo que REPRODUCIR puede pasar la misma frase por cada cambio que hagas.
txaudio-phrase-length = frase de { $secs } s
txaudio-auto-set-done = Ganancia de entrada fijada en { $gain } dB (los picos de voz fueron de { $loud } dBFS).{ $clip }
txaudio-listening = Escuchando… { $secs } s
txaudio-auto-set-hint = Habla con normalidad durante { $secs } s — como lo harías en el aire — y la ganancia de entrada se fija para que tu voz tenga picos en torno a { $target } dBFS. Haz clic de nuevo para cancelar.
txaudio-eq-point = { $hz } Hz, { $db } dB — arrastra para mover
txaudio-not-a-profile = No es un perfil de audio TX: { $error }
txaudio-imported = Importado «{ $name }».
txaudio-saved = Guardado «{ $name }».
txaudio-deleted = Eliminado «{ $name }».
txaudio-loaded-preset = Cargado el punto de partida «{ $name }».

## Solar-system 3D view: messages with values

solar-age-offline = { $age } · sin conexión
solar-radius-exaggeration = Exageración del radio de la Tierra/Luna (máx. { $max }× con esta escala de órbita lunar)
solar-brightest-cell = celda más brillante ≈ { $peak } trayectos · { $bands } { $bands ->
        [one] banda
       *[other] bandas
    }
solar-hits-in-hour = { $hits } en la hora
solar-passes-from = { $name } · PASOS DESDE { $grid }
solar-elements-age = elementos de hace { $age } · SGP4
solar-point-at = Apunta a { $az }° ({ $compass }), elevación { $el }°.
solar-published-as = publicado como { $name }
solar-fly-there = { $name } — ↵ para volar allí
solar-sats-tracked = { $n } de { $total } seguidos
solar-bodies-found = { $n } de { $total } cuerpos
solar-sats-and-bodies = { $n } de { $total } seguidos · { $bodies } de { $bodies_total } cuerpos
solar-confidence-on-band = { $confidence } en { $band }
solar-kp-now = { $kp } ahora
solar-kp-in = { $kp } dentro de { $hours } h
solar-valid-age = válido { $valid } · hace { $age }
solar-full-bar = barra completa = { $share } del mundo
solar-own-paths-memory = adónde fueron los trayectos propios de esta estación · memoria de { $minutes } min
solar-n-storms = { $n } tormentas
solar-clouds-legend = NUBES { $time } · hace { $age } · { $channels } · { $storms } · rayos simulados
solar-cme-eta = ETA { $eta } (+{ $hours } h)
solar-cme-arrived = la llegada fue { $eta } (hace { $hours } h)
solar-cme-earth-directed = CME DIRIGIDA A LA TIERRA { $time } · { $speed } km/s · { $when }{ $glancing }{ $estimated }

## Digital-mode setup dialogs: messages with values

setup-mode-setup = Configuración de { $mode }
setup-transmits-as = Transmite como { $call }
setup-decimal-degrees = Grados decimales, negativos para { $hemisphere }. Pega uno directamente de un mapa.

## AMP window: messages with values

amp-unreliable-at-swr = { $meters } NO FIABLES CON ESTA ROE
amp-lower-drive-first = Baja primero la excitación: { $drive } W sobreexcitarían el amplificador en línea (su límite es { $limit } W).
amp-live-value = en vivo { $value } { $unit }
amp-tripped = DISPARADO{ $why }

## Digital-mode panels, shared: messages with values

panels-top-paths = máx. ≈ { $n } trayectos
panels-no-agreed-frequency = No hay ninguna frecuencia acordada para { $mode } — abre esto para guardar en la que estás
panels-mode-frequencies-hint = Las { $n } frecuencias de { $mode } — elegir una sintoniza el dial, y puedes guardar las tuyas
panels-mode-frequencies = { $mode } · frecuencias
panels-global-convention = Una convención global en la que el plan de bandas de la Región { $region } de la IARU no pone datos de banda estrecha — consulta tu propio plan de bandas antes de transmitir aquí.
panels-outside-data-segment = Ámbar: fuera del segmento de datos de la Región { $region }.
panels-save-dial = ＋ Guardar { $mhz } MHz
panels-save-dial-hint = Recuerda este dial bajo { $mode } y lo ofrece aquí de ahora en adelante. Se guarda en la estación, así que lo tiene cada pantalla conectada a esta radio.
panels-tr-period = Periodo T/R de { $period } segundos
panels-this-turn = Este turno: { $into } s de { $slot }. Las decodificaciones llegan al final de él, y es entonces cuando puede empezar la siguiente transmisión.{ $mark }
panels-burst-mark = {" "}La marca es donde se detiene la ráfaga, a { $secs } s.
panels-tr-period-burst = Periodo T/R de { $slot } segundos, ráfaga de { $burst } s

## Memories and voice keyer windows: messages with values

windows-n-channels = { $n } { $n ->
        [one] canal
       *[other] canales
    }
windows-slot-n = Ranura { $n }
windows-record-from-mic = Grabar desde el micrófono (hasta { $max } s)

## SSTV panel: messages with values

sstv-picture-too-big = Esa imagen pesa { $size } MB — el límite es { $limit } MB.
sstv-auto-mode = Auto ({ $mode })
sstv-not-decoded = { $what } — no decodificado
sstv-last = última: { $mode }
sstv-saved-where = Las imágenes decodificadas se guardan { $place } y se acumulan aquí.
sstv-older-in-store = { $n } más antiguas en el almacén
sstv-older-load-more = { $n } más antiguas — cargar más
sstv-delete-picture-from = Eliminar esta imagen { $place }
sstv-rifp-profile = { $profile } · ~{ $khz } kHz ocupados · el dial es el centro del canal
sstv-rifp-too-wide = ⚠ { $profile } ocupa ~{ $khz } kHz — demasiado ancho para un segmento de banda estrecha
sstv-rifp-no-frequency = RIFP no asigna ninguna frecuencia, y sdroxide lo transmitirá donde sintonices. Un canal de { $khz } kHz solo cabe donde se permite la operación de banda ancha o FM — { $segments } — y no en un segmento de banda estrecha, y menos aún en HF. Incluso dentro de esos, tus propias condiciones de licencia pueden ser más estrechas. Tú eres el operador; consulta tus propias normas.
sstv-rifp-tx-frame = ● TX trama { $frame }/{ $frames } · quedan { $secs } s
sstv-rifp-sent-as = enviado como { $encoding } · { $octets } octetos
sstv-rifp-rx-stats = tramas { $frames } · erróneas { $bad } · imágenes { $pictures }
sstv-rifp-session-idle = sesión { $session } · inactiva { $secs } s
sstv-rifp-session-waiting = sesión { $session } · fragmentos retenidos, aún esperando el manifiesto · inactiva { $secs } s
sstv-store-on-radio = en la radio, en { $dir }
sstv-store-in = en { $dir }

## decodes (src/app/panels/decodes.rs)

decodes-decodes = DECODIFICACIONES
decodes-adif = ADIF
decodes-sort = Ordenar
decodes-single-list = Lista única
decodes-every-decode-one-list-newest = Todas las decodificaciones en una sola lista, el turno más reciente primero, en lugar de bloques de turnos pares/impares. Los chips de Ordenar ordenan entonces toda la lista en lugar de cada turno, y cada fila lleva la hora de su ranura — cian para una ranura par, dorado para una impar.
decodes-cq-only = Solo CQ
decodes-only-stations-calling-cq-only = Solo estaciones que llaman CQ — y solo las llamadas que puedes contestar: un CQ dirigido (DX, EU, JA, POTA, TEST …) aparece cuando te nombra a ti y se oculta cuando nombra a otro.
decodes-new-only = Solo nuevas
decodes-only-stations-would-new-entity = Solo estaciones que serían nuevas: entidad, ranura de banda, locátor o indicativo
decodes-auto-tx-frq = FRQ TX auto
decodes-overridden-hold-tx-lift-hold = Anulado por Mantener TX. Quita la retención para elegir hacia dónde se mueve la frecuencia de transmisión.
decodes-pick-our-transmit-frequency-automatically = Elige nuestra frecuencia de transmisión automáticamente: el punto más tranquilo del periodo en que transmitimos, en lugar de la frecuencia de aquel a quien respondemos — él transmite en el otro periodo, así que la suya no dice nada de quién hay cuando activamos nosotros. Desactivado NO mantiene la frecuencia: responde en la frecuencia de la estación a la que se llama. Para mantenerla, usa Mantener TX.
decodes-hold-tx = Mantener TX
decodes-pin-transmit-tone-where-nothing =
    Fija el tono de transmisión donde está. Nada lo mueve: ni responder a una estación, ni la cola de llamadas, ni llamar CQ, ni un clic en una decodificación o en la cascada. Desactívalo para moverlo, y vuelve a activarlo.

    Cambiar de banda es la única excepción, y es un acto tuyo: el desplazamiento que fijaste por última vez en la banda nueva vuelve con ella, de modo que una cifra de 60 m no se lleva a 20 m ni la de 20 m a 60 m.

    Para donde la licencia es más estrecha que el plan de bandas — con un dial de 60 m del Reino Unido de 5357 kHz la asignación termina en 5358,0, así que el tono tiene que quedarse por debajo de 1000 Hz, y cualquiera de los movedores automáticos se saldría de la banda entre un turno y el siguiente.
decodes-type-transmit-offset-hz-then = Escribe el desplazamiento de transmisión en Hz y pulsa Intro. Donde tu licencia es más estrecha que el plan de bandas, todo el rango por debajo del borde es tuyo para elegir, así que aquí no se sugiere nada.
decodes-clear-rx = {" "}BORRAR RX{" "}
decodes-empty-decode-list-nothing-air = Vacía la lista de decodificaciones. Nada de lo que está en el aire se detiene.
decodes-even = PAR
decodes-text = TEXTO
decodes-dxcc = DXCC
decodes-band = BANDA
decodes-grid = LOCÁT.
decodes-dupe = DUPL.
decodes-reply = RESP.
decodes-queued-click-remove = En cola — haz clic para quitar
decodes-work-station-after-current-one = Trabajar esta estación después de la actual
decodes-qsos-worked-since-sdroxide-was = QSO trabajados desde que se inició sdroxide
decodes-setup = ⚙ CONFIG.
decodes-qso-complete = ✓ QSO COMPLETO
decodes-contact-complete-log-held-here = El contacto está completo y en el diario. Se mantiene aquí unos minutos para poder reenviar el mensaje final si la otra estación repite el suyo — no se debe nada más.
decodes-tx = ● TX
decodes-dxpedition-mode-transmit-frequency-held = Modo DXpedición. La frecuencia de transmisión se mantiene fuera de la mitad del Fox de la banda pasante (por debajo de 1000 Hz) hasta que el Fox responda.
decodes-watchdog = VIGILANTE
decodes-transmitting-stopped-no-reply-no = La transmisión se detuvo: sin respuesta y sin ninguna acción durante el periodo del vigilante. Llama CQ o elige un mensaje para reanudar.
decodes-well-inside-tolerance = Bien dentro de la tolerancia.
decodes-no-active-qso-pick-decode = ningún QSO activo — elige una decodificación para responder, o llama CQ
decodes-ft8-engine-idle = Motor FT8 inactivo
decodes-being-worked = en curso
decodes-clear = BORRAR
decodes-no-messages = — sin mensajes —
decodes-call-cq = {"  "}LLAMAR CQ{"  "}
decodes-stop-qso = {" "}DETENER QSO{" "}
decodes-stop-tx = {" "}DETENER TX{" "}
decodes-free-text-13-chars = texto libre (13 caracteres)
decodes-send = ENVIAR

## sat (src/app/sat.rs)

sat-pass-profile = PERFIL DEL PASO
sat-satellite = SATÉLITE
sat-satellites = SATÉLITES
sat-passes = PASOS
sat-set-your-grid-passes-settings = Fija tu locátor para los pasos (Ajustes ▸ General).
sat-next = SIGUIENTES
sat-peak = MÁX ≥
sat-all-satellites = todos los satélites
sat-off-only-satellites-frequencies-file = Desactivado: solo satélites con frecuencias guardadas — los que puedes trabajar — y cualquiera que hayas pegado tú. Activado: todos los satélites de la lista, que pueden ser miles con una suscripción y tarda un rato en buscarse.
sat-no-passes-window-lower-peak = No hay pasos en esta ventana. Baja el máximo, o amplía la ventana.
sat-length = DURACIÓN
sat-peak-2 = MÁX
sat-from-to = DE → A
sat-up-now = ● visible ahora
sat-unlock = SOLTAR
sat-release-lock-dial-stays-where = Suelta el enganche; el dial se queda donde está
sat-azimuth = ACIMUT
sat-elevation = ELEVACIÓN
sat-range = DISTANCIA
sat-rate = VELOCIDAD
sat-downlink = DESCENDENTE
sat-doppler-rx = DOPPLER RX
sat-uplink = ASCENDENTE
sat-doppler-tx = DOPPLER TX
sat-no-pass-inside-next-48 = No hay ningún paso en las próximas 48 horas desde tu QTH.
sat-elements-stale-corrections-suspended-until = ⚠ Elementos obsoletos — correcciones suspendidas hasta que se actualice el TLE
sat-doppler-correction-needs-iq-front = La corrección Doppler necesita una etapa de entrada IQ — un equipo CAT solo hace seguimiento
sat-rotator-not-connected = Rotor: no conectado
sat-doppler = DOPPLER
sat-apply-computed-correction-rx-tx = Aplicar la corrección calculada a RX y TX
sat-rotator = ROTOR
sat-steer-antenna-through-rotctld-client = Orienta la antena a través del cliente rotctld (Ajustes ▸ Servidores)
sat-name-norad = nombre o NORAD
sat-set-your-grid-passes-settings-2 = fija tu locátor para los pasos (Ajustes ▸ General)
sat-norad = NORAD
sat-pass = PASO
sat-always-visible = siempre visible
sat-not-here = no desde aquí
sat-no-elements = sin elementos
sat-pick-satellite-see-links-lock = Elige un satélite para ver sus enlaces y engancharlo.
sat-no-frequencies-file-one-add = No hay frecuencias guardadas para este — añádelas en Ajustes ▸ TLE.
sat-link = ENLACE
sat-downlink-mhz = DESCENDENTE (MHz)
sat-uplink-mhz = ASCENDENTE (MHz)
sat-mode = MODO
sat-tune = SINTONIZAR
sat-set-dial-mode-link-no = Fija el dial y el modo en este enlace — sin enganche, sin corrección
sat-locked = {" "}ENGANCHADO{" "}
sat-lock = {" "}ENGANCHAR{" "}
sat-track-satellite-doppler-correction-rx = Sigue este satélite: corrección Doppler en RX y TX, enlace ascendente mapeado al transpondedor, y la vista 3D lo sigue

## wefax (src/app/panels/wefax.rs)

wefax-wefax = WEFAX
wefax-stop = {" "}■ PARAR{" "}
wefax-end-chart-now-save-what = Termina la carta ahora y guarda lo que ha llegado
wefax-start = {" "}● INICIAR{" "}
wefax-start-chart-now-without-waiting = Inicia una carta ahora, sin esperar un tono de inicio
wefax-phasing = sincronizando…
wefax-subcarrier-offset-1900-hz-tune = Desplazamiento de la subportadora respecto a 1900 Hz. Sintoniza para dejarlo en torno a cero: la portadora del fax va de 1500 Hz en negro a 2300 Hz en blanco, y un receptor desviado unos cientos de hercios recorta la imagen a negro o blanco sólidos.
wefax-auto-start = INICIO AUTO
wefax-begin-chart-when-300-hz = Empieza una carta cuando se oye el tono de inicio de 300 Hz (IOC 576) o de 675 Hz — y termina la que está en curso y sincroniza la nueva cuando llega la siguiente transmisión, haya llegado o no antes un tono de parada.
wefax-auto-stop = PARADA AUTO
wefax-end-450-hz-stop-tone = Termina con el tono de parada de 450 Hz. Desactívalo para seguir grabando a través de una estación que envía varias cartas seguidas.
wefax-phase = FASE
wefax-slant = INCLINACIÓN
wefax-sample-clock-trim-if-chart = Ajuste del reloj de muestreo. Si la carta se inclina a la izquierda, auméntalo; a la derecha, redúcelo. Una tarjeta de sonido desviada cien ppm desplaza de lado una carta de un cuarto de hora casi una línea entera.
wefax-view = VISTA
wefax-scale-chart-panel-width = Escala la carta al ancho del panel
wefax-whole = ENTERA
wefax-shrink-chart-until-all-view = Reduce la carta hasta que todo quepa en la vista
wefax-half-size = Mitad de tamaño
wefax-one-screen-pixel-per-fax = Un píxel de pantalla por píxel de fax — desplázate para ver detalle
wefax-twice-size-scroll-move-around = Doble tamaño; desplázate para moverte por la carta
wefax-height = ALTURA
wefax-stretch-picture-vertically-chart-came = Estira la imagen verticalmente. Una carta que salió aplastada o estirada suele estar decodificándose con la velocidad de línea equivocada — esto la hace legible, y los chips LPM lo arreglan bien. Haz doble clic para escribir un valor.
wefax-reset = RESTABLECER
wefax-back-picture-s-own-proportions = Volver a las proporciones propias de la imagen
wefax-follow = SEGUIR
wefax-keep-newest-lines-view-scrolling = Mantiene a la vista las líneas más recientes. Desplazarte hacia arriba lo desactiva para que puedas leer lo que ya ha llegado; volver a desplazarte hasta el final lo activa de nuevo.
wefax-tune-fax-schedule-usb-wait = Sintoniza un horario de fax en USB y espera un tono de inicio, o pulsa INICIAR para empezar a mitad de carta.
wefax-saved = GUARDADAS
wefax-path = RUTA
wefax-reading-radio-s-charts = Leyendo las cartas de la radio…
wefax-finished-charts-collect-here = Las cartas terminadas se acumulan aquí.
wefax-delete-chart = Eliminar esta carta
wefax-loading = cargando…
wefax-stations = ESTACIONES
wefax-broadcast-radiofax-schedules-picking-one = Horarios de radiofax — elegir uno sintoniza el dial
wefax-frequencies-are-published-carrier-dial = Las frecuencias son la portadora publicada; el dial va 1,9 kHz por debajo, lo que se hace por ti. Los horarios cambian y las estaciones cierran — tómalo como un lugar donde empezar a buscar, no como un horario.
wefax-delete-chart-store = Eliminar esta carta del almacén
wefax-newer = ◀ MÁS NUEVA
wefax-chart-received-after-one = La carta recibida después de esta
wefax-older = MÁS ANTIGUA ▶
wefax-chart-received-before-one = La carta recibida antes de esta
wefax-no-longer-store = ya no está en el almacén
wefax-loading-full-size = cargando a tamaño completo…
wefax-save = GUARDAR
wefax-save-copy-chart-computer = Guardar una copia de esta carta en este ordenador
wefax-sure = ¿SEGURO?
wefax-click-again-delete-chart-good = Haz clic de nuevo para eliminar esta carta definitivamente
wefax-delete = ELIMINAR
wefax-file-chart-was-saved = El archivo en que se guardó esta carta

## aprs (src/app/panels/aprs.rs)

aprs-starting-aprs-modem = iniciando el módem APRS…
aprs-aprs = APRS
aprs-no-callsign = sin indicativo
aprs-nothing-will-transmitted-no-beacon = No se transmitirá nada — ni una baliza, ni un mensaje, ni siquiera un acuse de recibo — hasta que esta estación tenga un indicativo. Fija uno en Ajustes → General, o uno específico de APRS con su SSID en CONFIG.
aprs-no-position = sin posición
aprs-beacon-needs-somewhere-report-fill = Una baliza necesita algo que informar. Rellena tu locátor en Ajustes → General, o da coordenadas en CONFIG.
aprs-setup = CONFIG.
aprs-callsign-symbol-digipeater-path-position = Indicativo, símbolo, ruta de digipetidores, posición y baliza
aprs-how-often-beacon-your-position = Cada cuánto emitir tu posición como baliza, sin supervisión. `off` — el valor predeterminado — nunca emite baliza: elegir un modo no debe poner una estación en el aire. Treinta minutos es la convención para una estación fija, más a menudo para una en movimiento, y cada baliza es tiempo de canal de otro.
aprs-beacon = {" "}BALIZA{" "}
aprs-station-has-no-callsign-see = Esta estación no tiene indicativo — mira el aviso de la izquierda.
aprs-station-has-no-position-report = Esta estación no tiene posición que informar — mira el aviso de la izquierda.
aprs-send-one-position-report-now = Envía un informe de posición ahora, sin esperar al intervalo. Espera a que el canal esté libre como todo lo demás.
aprs-stations = ESTACIONES
aprs-nothing-heard-yet = aún no se ha oído nada
aprs-heard-direct = oída directa
aprs-two-characters-station-sent-say = Los dos caracteres que envió la estación para decir qué es
aprs-killed = ELIMINADO
aprs-station-put-object-here-has = La estación que puso aquí este objeto lo ha cancelado
aprs-sender-deliberately-reported-square-rather = El remitente informó deliberadamente de un cuadrado en lugar de un punto. El mapa dibuja el cuadrado.
aprs-digipeaters-last-frame-came-through = Los digipetidores por los que pasó la última trama. Un `*` marca el que realmente la repitió.
aprs-message = {" "}MENSAJE{" "}
aprs-address-message-box-station = Dirige el cuadro de mensaje a esta estación
aprs-center = {" "}CENTRAR{" "}
aprs-put-station-middle-map-double = Pone esta estación en el centro del mapa. Haz doble clic en el mapa para devolver la vista al ajuste automático.
aprs-channel = CANAL
aprs-messages = MENSAJES
aprs-frames-arrived-failed-their-check = Tramas que llegaron y fallaron su secuencia de comprobación — una colisión, un desvanecimiento o una señal demasiado débil para leerse. Un contador que sube sin que nada decodifique significa que el módem *sí* oye el canal: revisa el medidor de nivel y abre el silenciador de la radio.
aprs-frames-read-cleanly-off-channel = Tramas leídas limpiamente del canal que no eran APRS — la sesión de packet de otra persona, o un formato que esta compilación no decodifica.
aprs-every-frame-channel-arrived-other = Cada trama del canal, tal como llegó — incluido el tráfico de otros y todo lo que el decodificador no pudo leer.
aprs-nothing-channel-yet = aún no hay nada en el canal
aprs-no-messages = Sin mensajes.
aprs-waiting-channel = esperando al canal
aprs-sent-retried-still-waiting-acknowledged = enviado y reintentado, aún esperando acuse de recibo
aprs-air-waiting-acknowledged = en el aire, esperando acuse de recibo
aprs-acknowledged-far-end = acusado por el otro extremo
aprs-refused-far-end-not-retried = rechazado por el otro extremo — no se reintenta
aprs-no-ack = sin ack
aprs-no-answer-after-every-retry = sin respuesta tras todos los reintentos
aprs-send = {" "}ENVIAR{" "}
aprs-station-has-no-callsign-so = Esta estación no tiene indicativo, así que no se puede transmitir nada. Fija uno en Ajustes → General, o uno específico de APRS con su SSID en CONFIG.
aprs-needs-station-address-something-say = Necesita una estación a la que dirigirse y algo que decir.
aprs-send-keep-retrying-until-far = Envíalo, y sigue reintentando hasta que el otro extremo acuse recibo. Los mensajes son lo único de este canal que se contesta.
aprs-busy = OCUPADO
aprs-stations-heard-still-inside-window = Estaciones oídas que siguen dentro de la ventana fijada en la configuración de APRS, y si hay otra estación en el canal ahora mismo. No se activará nada mientras esté ocupado.
aprs-tx = ● TX
aprs-over-progress-frames-waiting-channel = El turno en curso, las tramas que esperan a que el canal se libere, o el tiempo hasta la próxima baliza programada.
aprs-audio-reaching-modem-flat-means = Audio que llega al módem. Plano significa que no llega nada en absoluto — comprueba que la salida de datos de la radio es la que escucha sdroxide. Se ilumina en verde mientras el módem oye una portadora.

## vdl2 (src/app/panels/vdl2.rs)

vdl2-starting-vdl2-decoder = iniciando el decodificador VDL2…
vdl2-aircraft-datalink = enlace de datos de aeronaves
vdl2-tune-middle-vdl2-group-decoder = Sintoniza el centro del grupo VDL2. La propia ventana del decodificador se desliza desde ahí para abarcar tantos de los catorce canales como pueda alcanzar el receptor.
vdl2-setup = CONFIG.
vdl2-which-channels-listen-how-hard = En qué canales escuchar, lo fuerte que debe ser una ráfaga y cuánto registro conservar
vdl2-save-vdl2-message-log-file = Guardar en un archivo el registro de mensajes VDL2
vdl2-vdl2-channel = Canal VDL2
vdl2-messages = MENSAJES
vdl2-address-flight-registration-label = dirección, vuelo, matrícula o etiqueta
vdl2-held = RETENIDO
vdl2-hold = RETENER
vdl2-hold-message-log-where-so = Mantiene el registro de mensajes donde está, para poder leer uno sin que la siguiente transmisión lo empuje hacia arriba. El decodificador sigue funcionando y los contadores siguen moviéndose; al soltar se muestra todo lo que llegó entre tanto.
vdl2-nothing-decoded-yet = aún no se ha decodificado nada
vdl2-nothing-matches-filter = nada coincide con el filtro
vdl2-stations = ESTACIONES
vdl2-nobody-heard-yet = aún no se ha oído a nadie
vdl2-vdl2-setup = Configuración de VDL2
vdl2-channels = Canales
vdl2-every-25-khz-slot-136 = Cada ranura de 25 kHz de 136,650 a 136,975 MHz. Un conversor descendente cada una, todos dentro de la misma ventana del receptor. Desactivar uno ahorra un poco de tiempo de procesador; no hace a los demás más sensibles.
vdl2-burst-threshold = Umbral de ráfaga
vdl2-above-each-channel-s-own = sobre el suelo de ruido propio de cada canal
vdl2-lower-catches-weaker-transmissions-costs = Más bajo capta transmisiones más débiles y cuesta tiempo de procesador en ruido; más alto las pierde. El suelo se aprende por canal y un cambio aquí no lo descarta.
vdl2-keep-log = Conservar en el registro
vdl2-messages-2 = {" "}mensajes
vdl2-track-most = Seguir como máximo
vdl2-stations-2 = {" "}estaciones
vdl2-longest-silent-go-first = las que llevan más tiempo en silencio salen primero
vdl2-forget-station-after = Olvidar una estación tras
vdl2-nothing-heard = sin oír nada de ella
vdl2-show-unread-payloads = Mostrar cargas útiles sin leer
vdl2-hex = en hexadecimal
vdl2-frames-carrying-x-25-clnp = Tramas que llevan X.25, CLNP o las aplicaciones de enlace de datos por encima de ellos. SDRoxide las nombra y muestra los bytes en lugar de leerlas, y ocultarlas escondería cuánto del tráfico es eso.
vdl2-registration = Matrícula
vdl2-flight = Vuelo
vdl2-label = Etiqueta
vdl2-block = Bloque
vdl2-sequence = Secuencia
vdl2-mode = Modo
vdl2-more-blocks-follow = siguen más bloques
vdl2-check = Comprobación
vdl2-message-crc-good = CRC del mensaje correcto
vdl2-message-crc-not-checked = CRC del mensaje sin comprobar
vdl2-parity = Paridad
vdl2-link-control-no-payload = control de enlace, sin carga útil
vdl2-name = NOMBRE
vdl2-addr = DIR.
vdl2-type = TIPO
vdl2-msgs = MENS.

## cw (src/app/panels/cw.rs)

cw-tone-being-copied-tone-transmitted = El tono que se copia y el tono que se transmite — en CW son la misma frecuencia. Haz clic en la cascada para llevarlo a una señal.
cw-down-10-hz = Bajar 10 Hz
cw-up-10-hz = Subir 10 Hz
cw-qrg-main-readout-tuning-line = QRG: la lectura principal y la línea de sintonía están en la frecuencia con la que se trabaja. Haz clic para ver en su lugar el dial, con un tono lateral debajo — lo que muestran la mayoría de las radios.
cw-put-main-readout-tuning-line =
    Pone la lectura principal y la línea de sintonía en la señal en lugar de en el dial, de modo que la frecuencia mostrada es la que citarían ambos operadores y la línea de sintonía queda en el centro de la banda pasante. La sintonía no cambia; solo se mueven los números.

    Hacer clic en una señal la lleva al cursor solo con la precisión que permite el paso del clic — Controles → redondeo de la sintonía por clic, 10 Hz por omisión. Un paso grueso deja la señal desviada del tono hasta la mitad de él, y la lectura lo dirá.
cw-sending-speed-read-off-signal = Velocidad de envío leída de la señal
cw-signal-noise-500-hz-same = Relación señal/ruido en 500 Hz — la misma cifra que cita un informe
cw-how-far-off-cursor-signal = Cuánto se desvía del cursor la señal en realidad. El decodificador la sigue; la banda pasante no, así que mueve el cursor si crece.
cw-listening = — escuchando —
cw-tx = ● TX
cw-trainer = ENTRENADOR
cw-learn-morse-read-characters-words = Aprende Morse: lee caracteres y palabras, reprodúcelos a tu propia velocidad, y un ejercicio Koch que añade un carácter cada vez. Suena aquí, por los altavoces de este ordenador — nada activa la radio.
cw-nothing-copied-yet = — aún no se ha copiado nada —
cw-key-send-characters-you-send = Manipulador para enviar — los caracteres que envías aparecen aquí.
cw-type-line-return-sends = Escribe una línea, Intro la envía…
cw-type-here-send = Escribe aquí para enviar…
cw-key = {" "}LLAVE ●{" "}
cw-key-2 = {" "}LLAVE{" "}
cw-hold-key-bound-cw-straight = Mantén pulsada la tecla asignada a la llave recta de CW — Espacio por omisión, y cualquier tecla que quieras en Ajustes → Controles — como una llave recta: pulsada mientras la mantienes, suelta al soltar, en lugar de escribir texto. El cuadro de transmisión queda bloqueado mientras está activa, y todo el manipulador se entrega a la llave: el texto que estuviera en cola se descarta.
cw-radio-sends-own-keyer-text =
    Esta radio envía desde su propio manipulador: el texto va por el puerto de control y el equipo temporiza los elementos, así que no hay nada entre la llave recta y el aire que una mano pueda manejar.

    Para manipularla a mano, fija la Manipulación de CW en «Tarjeta de sonido (MCW)» en Ajustes → Radio. El equipo se mantiene entonces en una banda lateral y el tono lateral del propio manipulador se transmite como audio, que es la ruta que maneja la llave recta.
cw-watchdog = VIGILANTE
cw-straight-key-was-held-down = La llave recta se mantuvo pulsada demasiado tiempo — un levantamiento perdido más que una mano — así que se soltó la portadora y se desactivó la transmisión. Pulsa la llave de nuevo para continuar.
cw-tx-2 = {"  "}TX ACT.{"  "}
cw-tx-3 = {"   "}TX{"   "}
cw-send-what-box-now-without = Envía ahora lo que hay en el cuadro, sin esperar a Intro
cw-hold-key-down-between-characters = Mantiene la llave pulsada entre caracteres, para que nada de lo escrito espere
cw-call-cq = {" "}LLAMAR CQ{" "}
cw-nocall = NOCALL
cw-clear = {" "}BORRAR{" "}
cw-stop-sending-drop-whatever-has = Deja de enviar y descarta lo que no haya salido. Con la llave recta activada, la lectura de vuelta de lo que manipulaste se va con ello.
cw-sidetone = TONO LATERAL
cw-play-keyed-tone-through-computer = Reproduce el tono manipulado por los altavoces de este ordenador además de enviarlo, para que oigas lo que envías. En `Tarjeta de sonido (MCW)` el tono sale al equipo y a ninguna otra parte, así que sin esto envías en silencio. Desactívalo donde el monitor del propio equipo o una copia en el aire ya cumple esa función.
cw-hold-what-typed-until-return = Retiene lo escrito hasta Intro y luego envía la línea de una pieza en lugar de manipular cada carácter según se escribe. Conviene en un transceptor que se manipula solo a partir de texto, donde cada entrega a su manipulador es otro ciclo de transmisión-recepción.
cw-your-own-message-buttons-contest = Tus propios botones de mensaje — un intercambio de concurso, una respuesta con nombre y QTH, TNX 73 GL. Cada uno envía todo su texto de golpe, y F2–F10 pulsan los nueve primeros. Viajan con la configuración de la estación, así que un cliente remoto también los tiene.
cw-cw-messages = MENSAJES CW
cw-clear-rx = {" "}BORRAR RX{" "}
cw-empty-receive-window-straight-key = Vacía la ventana de recepción y la lectura de vuelta de la llave recta. Nada de lo que está en el aire se detiene.
cw-lock = FIJAR
cw-decode-speed-set-here-instead = Decodifica a la velocidad fijada aquí en lugar de leerla de la señal. Ayuda con una señal demasiado débil para que la búsqueda de velocidad se asiente.
cw-farnsworth-send-characters-full-speed = Farnsworth: envía los caracteres a plena velocidad y alarga solo los huecos entre ellos, de modo que se oyen con el ritmo correcto pero llegan lo bastante despacio para escribirlos.
cw-off-normal-spacing = Desactivado — espaciado normal
cw-keying-speed = Velocidad de manipulación
cw-idle-0 = REPOSO 0
cw-how-long-transmit-held-after = Cuánto tiempo se mantiene la transmisión tras el último carácter, o tras soltar la llave recta, antes de que caiga la portadora. Más largo salva las pausas de un mecanógrafo lento; más corto abandona antes la frecuencia; Desactivado la suelta en cuanto ha salido todo lo que estaba en cola.
cw-off-drop-once = Desactivado — soltar de inmediato

## textmodem (src/app/panels/text_modem.rs)

textmodem-tune-down-10-hz = Sintonizar 10 Hz abajo
textmodem-tune-up-10-hz = Sintonizar 10 Hz arriba
textmodem-tx = ● TX
textmodem-listening = — escuchando —
textmodem-type-line-return-sends = Escribe una línea, Intro la envía…
textmodem-type-here-transmit = Escribe aquí para transmitir…
textmodem-tx-2 = {"  "}TX ACT.{"  "}
textmodem-tx-3 = {"   "}TX{"   "}
textmodem-call-cq = {" "}LLAMAR CQ{" "}
textmodem-nocall = NOCALL
textmodem-clear = {" "}BORRAR{" "}
textmodem-hold-what-typed-until-return = Retiene lo escrito hasta Intro y luego envía la línea de una pieza en lugar de emitir cada carácter al aire según se escribe. Permite releer y corregir una línea antes de que salga nada de ella.
textmodem-your-own-message-buttons-working = Tus propios botones de mensaje — condiciones de trabajo, el tiempo, una respuesta estándar. Cada uno envía todo su texto de golpe, y F2–F10 pulsan los nueve primeros. Viajan con la configuración de la estación, así que un cliente remoto también los tiene.
textmodem-messages = MENSAJES
textmodem-hell = HELL
textmodem-contrast = Contraste
textmodem-harder-softer-dots-redraws-whole = Puntos más duros o más suaves — redibuja toda la franja
textmodem-width = Ancho
textmodem-t-2row = {" "}2FILAS{" "}
textmodem-draw-every-column-twice-stacked = Dibuja cada columna dos veces, apiladas. Hell no tiene sincronismo vertical, así que esto mantiene legible una copia completa del texto sea cual sea la fase.
textmodem-rev = {" "}INV{" "}
textmodem-reverse-video-light-dots-dark = Vídeo inverso — puntos claros sobre papel oscuro
textmodem-clear-rx = {" "}BORRAR RX{" "}
textmodem-hold-channel-idle-sends-blank = Mantiene el canal: en reposo envía papel en blanco, para que la franja siga desplazándose
textmodem-hold-what-typed-until-return-2 = Retiene lo escrito hasta Intro y luego envía la línea de una pieza en lugar de pintar cada carácter en la franja según se escribe. Una corrección escrita en directo ya está en el papel; una hecha antes de Intro nunca lo estuvo.
textmodem-shift = Desplaz.
textmodem-shift-5-hz = Desplaz. −5 Hz
textmodem-shift-5-hz-2 = Desplaz. +5 Hz
textmodem-baud = Baudios
textmodem-reverse-swap-mark-space-needed = Invertir: intercambia marca y espacio. Hace falta cuando la señal se recibe en la banda lateral opuesta a aquella en que se envió, lo que invierte los tonos.
textmodem-track-tuning-error-automatically-instead = Sigue el error de sintonía automáticamente en lugar de quedarse fijado al cursor.
textmodem-tones = Tonos
textmodem-mode = Modo
textmodem-off-keyed = manipulado por todo/nada

## ais (src/app/panels/ais.rs)

ais-starting-ais-decoder = iniciando el decodificador AIS…
ais-tune-between-two-ais-channels = Sintoniza entre los dos canales AIS — 161,975 (AIS 1, marino 87B) y 162,025 (AIS 2, 88B). El dial va en medio porque allí no transmite nada, así que el pico de continua de un receptor de FI cero no cae en ninguno.
ais-setup = CONFIG.
ais-channels-timeouts-trail-length-how = Canales, tiempos de espera, longitud de la estela y hasta dónde llegan los vectores
ais-vessels = BUQUES
ais-nothing-heard-yet = aún no se ha oído nada
ais-virtual-nothing-there = virtual — no hay nada ahí
ais-position-stale = posición obsoleta
ais-search-rescue-transmitter-man-overboard = un transmisor de búsqueda y rescate, una baliza de hombre al agua o una radiobaliza AIS
ais-call-sign = indicativo
ais-station-time = hora de la estación
ais-dgnss = {" "}(DGNSS)
ais-first-heard = oído por primera vez
ais-center = CENTRAR
ais-put-vessel-middle-chart = Pone este buque en el centro de la carta
ais-last-message-station-nmea-sentences = El último mensaje de esta estación, como las sentencias NMEA que pondría en el cable un receptor. Pégalo en cualquier otro decodificador AIS para comprobar qué hizo sdroxide con él.
ais-ais-setup = Configuración de AIS
ais-channels = Canales
ais-drop-chart-after = Quitar de la carta tras
ais-without-position-report = sin un informe de posición
ais-five-minutes-default-not-ads = Cinco minutos por omisión, no los diez segundos de ADS-B: un buque fondeado informa una vez cada tres minutos, y una ventana más corta dejaría en blanco la mayor parte de un puerto entre dos informes perfectamente buenos. Pasado ese tiempo el barco sale de la carta y su fila se vuelve gris — no se desvanece, porque un símbolo tenue en una posición obsoleta sigue siendo una afirmación sobre dónde está un barco, con la misma tinta que las verdaderas.
ais-drop-list-after = Quitar de la lista tras
ais-nothing-heard-all = sin oír nada en absoluto
ais-message-carrying-ship-s-name = El mensaje que lleva el nombre de un barco vuelve cada seis minutos, así que una ventana de lista corta no deja de descartar buques justo antes de que digan cómo se llaman.
ais-trail-length = Longitud de la estela
ais-history-behind-each-target = de historial tras cada objetivo
ais-minutes-rather-than-points-because = En minutos y no en puntos, porque las frecuencias de informe de AIS abarcan dos órdenes de magnitud: una cuenta fija serían ochenta segundos de un ferri y dos horas de un petrolero fondeado, dibujados igual. Cero desactiva las estelas.
ais-speed-vector = Vector de velocidad
ais-how-far-ahead-vector-reaches = hasta dónde llega el vector por delante
ais-slot-threshold = Umbral de ranura
ais-above-channel-s-learned-noise = sobre el suelo de ruido aprendido del canal
ais-track-most = Seguir como máximo
ais-vessels-2 = {" "}buques
ais-ais-receive-only-here-always = AIS aquí es solo de recepción y siempre lo será: es un servicio de seguridad de la vida humana, y poner en él tráfico falso de buques no es algo que cubra una licencia.
ais-fill-my-grid-digimode-setup = Rellena Mi locátor en la configuración de modos digitales para distancias y rumbos, y para que la carta se encuadre alrededor de donde estás y no alrededor de lo que esté más lejos.
ais-name = NOMBRE
ais-mmsi = MMSI
ais-type = TIPO

## qo100 (src/app/qo100.rs)

qo100-receiver-cannot-reach-10489-750 = Este receptor no puede llegar por sí solo a 10489,750 MHz — configura primero un desplazamiento de LNB/conversor (Ajustes ▸ Radio)
qo100-run-fast-spectral-tracker-every = Ejecuta el seguidor espectral rápido: cada segundo, busca en la ventana de aparcamiento los dos lóbulos simétricos de la baliza e informa de dónde está la portadora
qo100-telemetry = TELEMETRÍA
qo100-also-run-ao-40-uncoded = Ejecuta también el decodificador de tramas AO-40 sin codificar — palabra de sincronismo, CRC y el texto de telemetría — con una lectura paso a paso (portadora → sincronismo → CRC) de hasta dónde llega cada pasada
qo100-auto = AUTO
qo100-auto-correct-let-tracker-adjust = Autocorrección: deja que el seguidor ajuste por sí mismo el desplazamiento del conversor/LNB — un lazo cerrado lento que, con una estimación limpia y estable, devuelve la baliza a 10489,750 MHz y la mantiene ahí mientras el LNB deriva. Reabre la etapa de entrada cada vez que actúa.
qo100-narrower-search-smaller-slice = Más estrecho — busca en una porción menor
qo100-wider-when-beacon-isn-t = Más ancho — para cuando la baliza no se encuentra con el ancho actual
qo100-clear-mark = borrar marca
qo100-drop-hand-placed-beacon-mark = Descarta la marca de baliza puesta a mano y vuelve a la lectura propia del decodificador
qo100-lo = lo −
qo100-lo-2 = lo +
qo100-hi = hi −
qo100-hi-2 = hi +
qo100-radio-s-own-tuning-range = El rango de sintonía propio de esta radio no llega a 10489,750 MHz — necesita un desplazamiento de LNB/conversor (Ajustes ▸ Radio ▸ Conversor) antes de que esta ventana pueda hacer nada.
qo100-open-settings-radio = Abrir Ajustes ▸ Radio
qo100-tune-10489-750-mhz = Sintonizar 10489,750 MHz
qo100-tip-if-you-can-see = consejo: si ves la baliza en la franja pero no engancha, haz doble clic en ella — eso la marca como 10489,750 MHz y deja que APLICAR corrija hasta ella
qo100-tracker = SEGUIDOR
qo100-no-twin-lobe-shape-parking = no hay forma de dos lóbulos en la ventana de aparcamiento
qo100-shape = FORMA
qo100-drift = DERIVA
qo100-need-longer-run-estimates = — (hace falta una serie más larga de estimaciones)
qo100-cycles = CICLOS
qo100-receiver = RECEPTOR
qo100-target = OBJETIVO
qo100-measured = MEDIDO
qo100-clicked = {"  "}(con clic)
qo100-tracker-2 = {"  "}(seguidor)
qo100-not-locked-yet = aún sin enganchar
qo100-converter-offset = DESPLAZAMIENTO DEL CONVERSOR
qo100-decode = DECODIFICACIÓN
qo100-decode-filling-first-frame-buffer = DECODIFICACIÓN — llenando el primer búfer de trama…
qo100-decode-off-turn-telemetry-chip = DECODIFICACIÓN — desactivada. Activa el chip TELEMETRÍA (fila superior) para la lectura portadora → sincronismo → CRC.
qo100-apply-correction = {" "}APLICAR CORRECCIÓN{" "}
qo100-write-converter-lnb-offset-puts = Escribe el desplazamiento de conversor/LNB que lleva a 10489,750 MHz la baliza donde hiciste clic, y reabre el receptor — la misma breve interrupción que produce Ajustes ▸ Radio ▸ Aplicar
qo100-write-converter-lnb-offset-puts-2 = Escribe el desplazamiento de conversor/LNB que lleva a 10489,750 MHz la estimación del seguidor, y reabre el receptor — la misma breve interrupción que produce Ajustes ▸ Radio ▸ Aplicar
qo100-waiting-second-crc-valid-frame = Esperando una segunda trama con CRC válido antes de ofrecer escribir esto — un solo enganche podría ser una coincidencia
qo100-write-corrected-converter-lnb-offset = Escribe el desplazamiento de conversor/LNB corregido y reabre el receptor — una breve interrupción, la misma que produce Ajustes ▸ Radio ▸ Aplicar
qo100-decoder-runs-receiving-station-readout = el decodificador se ejecuta en la estación receptora — su lectura no se envía a los clientes remotos
qo100-starting = iniciando…
qo100-searching-first-window-fills-after = buscando — la primera ventana se llena tras unos 24 s, luego se repite

## js8 (src/app/panels/js8.rs)

js8-multi = MULTI
js8-decode-every-js8-speed-not = Decodifica todas las velocidades de JS8, no solo aquella con la que transmites. Las cuatro velocidades comparten la subbanda y son cuatro formas de onda distintas, así que sin esto una estación en otra velocidad es invisible — y un intercambio entre dos velocidades no puede ocurrir en absoluto. Cuesta unas cuatro veces la CPU de recepción.
js8-hb-auto = HB AUTO
js8-turbo-does-not-beacon-local = Turbo no emite baliza — es la velocidad local y de VHF
js8-beacon-your-callsign-grid-every = Emite tu indicativo y locátor como baliza cada 15 minutos
js8-until-next-heartbeat = Hasta el próximo heartbeat
js8-setup = ⚙ CONFIG.
js8-tx = ● TX
js8-heard = OÍDAS
js8-nothing-heard-yet = — aún no se ha oído nada —
js8-dxcc = DXCC
js8-band = BANDA
js8-grid = LOCÁT.
js8-dupe = DUPL.
js8-reply = RESP.
js8-no-messages = — sin mensajes —
js8-cq = {" "}CQ{" "}
js8-hb = {" "}HB{" "}
js8-clear = {" "}LIMPIAR A{" "}
js8-forget-selected-station-composer-goes = Olvidar la estación seleccionada — el compositor vuelve a @ALLCALL
js8-already-addressing-allcall = Ya se dirige a @ALLCALL
js8-stop = {" "}PARAR{" "}
js8-send = {" "}ENVIAR{" "}
js8-message = Mensaje…

## logbook (src/app/logbook.rs)

palette-hint = Ir a… un modo, una banda, una ventana, un ajuste, una memoria o una frecuencia (14.074)
palette-nothing = nada con ese nombre
palette-group-window = Ventana
palette-group-settings = Ajustes
palette-group-memory = Memoria
palette-group-tune = Sintonía
palette-tune-to = Sintonizar a { $mhz } MHz
palette-win-bands = Condiciones de banda
palette-win-awards = Diplomas
palette-win-windows = Lista de ventanas
logbook-logbook = DIARIO
logbook-new-entry = + NUEVA ENTRADA
logbook-adif = ADIF
logbook-import = IMPORTAR
logbook-import-qsos-adif-adi-adif = Importar QSO desde un archivo ADIF (.adi o .adif)
logbook-new-qso = NUEVO QSO
logbook-edit-qso = EDITAR QSO
logbook-call = Indicativo
logbook-lookup = BUSCAR
logbook-look-up-name-qth-grid = Buscar nombre / QTH / locátor
logbook-grid = Locátor
logbook-freq-mhz = Frec. MHz
logbook-mode = Modo
logbook-rst-sent = RST env.
logbook-rst-rcvd = RST rec.
logbook-name = Nombre
logbook-state = Provincia
logbook-country = País
logbook-date-utc = Fecha UTC
logbook-time = Hora
logbook-pwr-w = Pot. W
logbook-contest = Concurso
logbook-s-sent = N.º env.
logbook-s-rcvd = N.º rec.
logbook-comment = Comentario
logbook-save = {" "}GUARDAR{" "}
logbook-cancel = CANCELAR
logbook-lotw = LoTW ✓
logbook-eqsl = eQSL ✓
logbook-card = tarjeta ✓
logbook-lotw-2 = LoTW ↑
logbook-eqsl-2 = eQSL ↑
logbook-qrz = QRZ ↑
logbook-hamqth = HamQTH ↑
logbook-club-log = Club Log ↑
logbook-wavelog = Wavelog ↑
logbook-delete-entry = Eliminar esta entrada
logbook-edit = EDITAR
logbook-upload-qso-configured-logs = Subir este QSO a los registros configurados
logbook-no-qsos-yet-run-ft8 = aún no hay QSO — usa FT8/FT4 o añade una entrada manual

## scanner (src/app/scanner.rs)

scanner-scanner = Escáner
scanner-work-through-stored-memory-channels = Recorre los canales de memoria guardados
scanner-work-through-slice-band-channel = Recorre una porción de una banda sobre una rejilla de canales
scanner-stop = PARAR
scanner-start = INICIAR
scanner-from = Desde
scanner-step = Paso
scanner-mode = Modo
scanner-range-empty-high-edge-has = Ese rango está vacío — el borde superior tiene que estar al menos un canal por encima del inferior.
scanner-skip-while-holding-pass-over = SALTAR mientras se detiene para pasar por alto ese canal de ahora en adelante
scanner-skipping = Saltando
scanner-clear = BORRAR
scanner-stop-passing-over-any-them = Dejar de pasar por alto cualquiera de ellos
scanner-stop-passing-over-channel = Dejar de pasar por alto este canal
scanner-no-memory-channels-stored-yet = Aún no hay canales de memoria guardados — guarda algunos en la ventana MEM.
scanner-skip-channel-pass-over = SALTAR un canal para pasarlo por alto
scanner-skip = SALTAR
scanner-look-all-channels-fall-inside = Busca todos los canales que caen dentro de una ventana del receptor sobre la misma transformada con que se hace el panadaptador, y solo sintoniza aquellos en que hay algo. Una lista en una banda cuesta entonces una sintonía por vuelta sea cual sea su longitud, en lugar de un tiempo de asentamiento por canal. El barrido sigue escuchando en cada candidato antes de parar, así que lo que lo detiene no cambia — pero el barrido mide a través de la FFT y no del filtro del receptor, así que revisa el umbral si empieza a pararse en nada.
scanner-radio-hands-over-demodulated-audio = Esta radio entrega audio demodulado y no tiene espectro propio en el que buscar, así que su barrido de memorias visita todos los canales en cualquier caso.
scanner-fast = RÁPIDO
scanner-read-list-off-spectrum-instead = lee la lista del espectro en lugar de visitar cada canal
scanner-folders = Carpetas
scanner-scan-every-folder-every-folder = Barre todas las carpetas, y todas las que se creen de ahora en adelante. Elige carpetas en su lugar para barrer solo esas.
scanner-unfiled = Sin archivar
scanner-nothing-filed-under-folders-you = No hay nada archivado en las carpetas que elegiste.
scanner-stops = Se detiene en
scanner-use-receiver-s-own-squelch = Usa el silenciador propio del receptor, para que el barrido se pare exactamente donde se abriría el audio
scanner-squelch-off-scan-will-stop = el silenciador está desactivado — el barrido se parará en lo primero que mire
scanner-channel-power-signal-has-reach = Potencia de canal que debe alcanzar una señal para detener el barrido
scanner-listens = Escucha durante
scanner-how-long-stay-candidate-before = Cuánto tiempo quedarse en un candidato antes de juzgarlo. Por debajo de una décima de segundo el medidor de nivel no se ha asentado y se pierden señales débiles
scanner-resumes = Reanuda
scanner-carry-once-signal-drops = Continúa cuando la señal cae
scanner-carry-after-fixed-time-regardless = Continúa tras un tiempo fijo, pase lo que pase
scanner-stay-until-you-press-next = Se queda hasta que pulses SIGUIENTE
scanner-how-long-stay = Cuánto tiempo quedarse
scanner-how-long-wait-after-signal = Cuánto esperar tras caer la señal
scanner-move-don-t-stop-channel = Seguir adelante, y no volver a parar en este canal
scanner-next = SIGUIENTE
scanner-move-now = Seguir adelante ahora

## adsb (src/app/panels/adsb.rs)

adsb-starting-ads-b-decoder = iniciando el decodificador ADS-B…
adsb-ads-b = ADS-B
adsb-t-1090-mhz-mode-s = 1090 MHz Modo S
adsb-tune-receiver-1090-000-mhz = Sintoniza el receptor a 1090,000 MHz, el enlace descendente ADS-B mundial. Solo hay un canal.
adsb-setup = CONFIG.
adsb-timeouts-trail-length-how-far = Tiempos de espera, longitud de la estela y hasta dónde llegan los vectores de velocidad
adsb-aircraft = AERONAVES
adsb-nothing-heard-yet = aún no se ha oído nada
adsb-ground = en tierra
adsb-position-stale = posición obsoleta
adsb-gnss-altitude = altitud GNSS
adsb-ground-speed = velocidad sobre el suelo
adsb-vertical-rate = velocidad vertical
adsb-first-heard = oído por primera vez
adsb-center = CENTRAR
adsb-put-aircraft-middle-map = Pone esta aeronave en el centro del mapa
adsb-last-frame-accepted-aircraft = La última trama aceptada de esta aeronave
adsb-ads-b-setup = Configuración de ADS-B
adsb-drop-map-after = Quitar del mapa tras
adsb-without-position-report = sin un informe de posición
adsb-past-aircraft-comes-off-map = Pasado ese tiempo la aeronave sale del mapa y su fila se vuelve gris. No se desvanece: un cuadrado tenue en una posición obsoleta sigue siendo una afirmación sobre dónde está un avión, con la misma tinta que las verdaderas.
adsb-drop-list-after = Quitar de la lista tras
adsb-nothing-heard-all = sin oír nada en absoluto
adsb-trail-length = Longitud de la estela
adsb-points = {" "}puntos
adsb-history-dots-behind-each-target = puntos de historial tras cada objetivo
adsb-speed-vector = Vector de velocidad
adsb-how-far-ahead-leader-line = hasta dónde llega la línea guía por delante
adsb-one-minute-usual-radar-convention = Un minuto es la convención habitual del radar: la línea es exactamente tan larga como la distancia que recorre la aeronave en ese tiempo, así que dos guías iguales son dos velocidades iguales con cualquier zoom. Cero las desactiva.
adsb-track-most = Seguir como máximo
adsb-aircraft-2 = {" "}aeronaves
adsb-aircraft-ground-are-placed-against = Las aeronaves en tierra se sitúan respecto a la posición propia de la estación — un squitter de superficie no tiene una decodificación inequívoca propia —, así que rellena Mi locátor en la configuración de modos digitales si el aeropuerto cercano no muestra nada.
adsb-call = INDIC.
adsb-icao = OACI
adsb-v-s = V/S

## drm (src/app/drm.rs)

drm-waiting-receiver = esperando al receptor…
drm-not-drm-set-mode-drm = No está en DRM. Pon el modo en DRM en una emisión digital de onda corta — el dial va en el centro del canal, no en una banda lateral.
drm-no-drm-signal-locked-decoder = Ninguna señal DRM enganchada. El decodificador necesita unos segundos con una portadora limpia: DRM entrelaza durante 400 ms o 2 s antes de que se pueda leer nada.
drm-station-s-audio-codec-not = El códec de audio de esta emisora no se está decodificando aquí. xHE-AAC necesita libfdk-aac en el sistema — no puede incorporarse, porque su licencia y la de este programa son incompatibles. Instálala (Debian/Ubuntu: libfdk-aac2, Arch: libfdk-aac, macOS: brew install fdk-aac, Windows: libfdk-aac-2.dll junto a sdroxide.exe) y reinicia. Todo lo demás de esta pantalla se decodifica con normalidad.
drm-quality = CALIDAD
drm-last-minute = último minuto
drm-waiting-locked-signal = esperando una señal enganchada
drm-audio-frames-decoding-but-station = Tramas de audio decodificándose — pero el códec de esta emisora no se puede decodificar aquí
drm-audio-frames-decoding = Tramas de audio decodificándose
drm-samples-reaching-decoder = Muestras que llegan al decodificador
drm-time = TIEMPO
drm-symbol-timing-recovered = Temporización de símbolo recuperada
drm-frame = TRAMA
drm-transmission-frames-found = Tramas de transmisión encontradas
drm-fast-access-channel-what-transmission = Fast Access Channel — qué es la transmisión
drm-service-description-channel-what-services = Service Description Channel — cuáles son los servicios
drm-audio = AUDIO
drm-constellation = CONSTELACIÓN
drm-waiting-decoded-symbols = esperando símbolos decodificados
drm-sampled-across-frame = muestreado a lo largo de la trama
drm-whole-frame = trama completa
drm-mode = MODO
drm-robustness-mode-channel-width-ground = Modo de robustez y ancho de canal. A es para un trayecto de onda de superficie y lleva más; D es para uno de onda ionosférica muy dispersa y lleva menos.
drm-interleave = ENTRELAZADO
drm-how-far-transmission-spreads-each = Cuánto reparte la transmisión cada trama en el tiempo. Largo aguanta desvanecimientos más profundos y tarda correspondientemente más en adquirirse.
drm-protection = PROTECCIÓN
drm-offset = DESPLAZ.
drm-residual-sample-clock-error-against = Error residual del reloj de muestreo respecto al transmisor. Grande y constante significa que la referencia del receptor está desviada, no la emisión.
drm-doppler = DOPPLER
drm-delay = RETARDO
drm-doppler-delay-spread-path-how = Dispersión Doppler y de retardo del trayecto — con qué rapidez se mueve y cuán separados llegan sus ecos.
drm-service = SERVICIO
drm-decode-service-multiplex = Decodificar este servicio del múltiplex

## atchat (src/app/panels/atchat.rs)

atchat-starting-atchat-net-station = iniciando la estación de la red AtCHAT…
atchat-atchat = ATCHAT
atchat-net-multi-station-keyboard-file = RED — teclado y archivos entre varias estaciones
atchat-net = {" "}EN RED{" "}
atchat-net-click-leave-state-kept = En la red — haz clic para salir (el estado se conserva; haz clic de nuevo para volver a unirte)
atchat-rejoin = {" "}VOLVER A UNIRSE{" "}
atchat-off-net-click-rejoin = Fuera de la red — haz clic para volver a unirte
atchat-off-net = fuera de la red
atchat-carrier = ● PORTADORA
atchat-another-station-transmitting-channel-busy = Otra estación está transmitiendo — el canal está ocupado.
atchat-tx = ● TX
atchat-setup = ⚙ CONFIG.
atchat-station-callsign-virtual-channel-address = Indicativo de la estación y dirección del canal virtual
atchat-virtual-channel = CANAL VIRTUAL
atchat-work-net-over-tcp-endpoint = Trabaja la red sobre un extremo TCP en lugar de la radio — un bucle local compatible con channel_server para desarrollo sin radio. Desactivado pone la estación en el aire.
atchat-roster = LISTA
atchat-nobody-heard-yet = aún no se ha oído a nadie
atchat-chat = {" "}CHAT{" "}
atchat-common-channel-lines-here-go = El canal común — las líneas de aquí van a todos (ALL)
atchat-log = {" "}REGISTRO{" "}
atchat-station-s-own-air-activity = La actividad propia de la estación en el aire — elección de maestro, envejecimiento de la lista, reintentos ARQ
atchat-no-activity-logged-yet = Aún no hay actividad registrada.
atchat-no-messages-yet = Aún no hay mensajes.
atchat-message-everyone-all = mensaje para todos (ALL)
atchat-send = {" "}ENVIAR{" "}
atchat-transfers = TRANSFERENCIAS
atchat-all = → ALL
atchat-sent-file-follows-active-chat = Un archivo enviado sigue a la pestaña de chat activa
atchat-send-file = {" "}ENVIAR ARCHIVO{" "}
atchat-send-file-image-over-air = Envía un archivo o una imagen por el aire, con ARQ por CRC de bloque
atchat-nothing-flight = nada en curso
atchat-images = IMÁGENES
atchat-no-images-received = No se han recibido imágenes.


## rds (src/app/rds.rs)

rds-station = EMISORA
rds-diagnostics = DIAGNÓSTICO
rds-which-programme-type-table-read = Con qué tabla de tipos de programa leer esta emisora. RDS se usa en todas partes salvo en Norteamérica; RBDS añade el indicativo que deletrea el código de identidad de una emisora de EE. UU. Auto sigue el código de país extendido que envía la emisora, y hasta que llega uno no deletreará un indicativo — los rangos de identidad que leería los comparten los códigos RDS de nueve países, así que elige RBDS para nombrar una emisora que no envía código de país.
rds-waiting-receiver = esperando al receptor…
rds-no-rds-station-not-every = No hay RDS en esta emisora. No todos los transmisores lo llevan, y uno débil puede llevarlo sin que llegue.
rds-rds-only-carried-wfm-broadcast = RDS solo se emite en las emisoras de radiodifusión WFM.
rds-holding = RETENIENDO
rds-block-sync-has-dropped-last = Se ha perdido el sincronismo de bloque — esta es la última emisora leída.
rds-identity = IDENTIDAD
rds-programme = PROGRAMA
rds-described = DESCRITA COMO
rds-content = CONTENIDO
rds-music = Música
rds-speech = Palabra
rds-traffic = TRÁFICO
rds-announcement-air-now = anuncio en el aire ahora
rds-carried-station = emitido por esta emisora
rds-not-carried = no emitido
rds-radio-text = RADIOTEXTO
rds-station-clock = RELOJ DE LA EMISORA
rds-country-code = CÓDIGO DE PAÍS
rds-also = TAMBIÉN EN
rds-also-tagged = TAMBIÉN ETIQUETADA
rds-sync = SINC
rds-no-sync = SIN SINC
rds-blocks-failed-their-check-plus = Bloques que fallaron su comprobación, más aquellos que el corrector de errores tuvo que reparar. Las correcciones cuentan como errores a propósito: una cifra creciente es la primera señal de que una emisora está a punto de dejar de decodificarse.
rds-front-end-overloading = ETAPA DE ENTRADA SOBRECARGADA
rds-converter-clipping-which-puts-distortion = El conversor está recortando, lo que pone distorsión por todo el múltiplex — incluida la subportadora de datos de 57 kHz. Baja la ganancia de RF o conecta un atenuador: el RDS se va aproximadamente un decibelio antes de que nada sea audible, así que una emisora que suena perfecta puede seguir siendo demasiado fuerte para que sus datos sobrevivan.
rds-group-types = TIPOS DE GRUPO
rds-recent-groups-newest-last = GRUPOS RECIENTES — el más nuevo al final
rds-nothing-decoded-yet = aún no se ha decodificado nada

## winlink (src/app/winlink.rs)

winlink-mail = CORREO
winlink-abort = ABORTAR
winlink-stop-session-progress = Detener la sesión en curso
winlink-connect = CONECTAR
winlink-compose = REDACTAR
winlink-connecting = conectando…
winlink-not-connected-yet = aún sin conectar
winlink-loading = cargando…
winlink-date = FECHA
winlink-from = DE
winlink-subject = ASUNTO
winlink-no-subject = (sin asunto)
winlink-delete-message = Eliminar este mensaje
winlink-arch = ARCH
winlink-move-archive = Mover al archivo
winlink-select-message = selecciona un mensaje
winlink-reply = RESPONDER
winlink-delete = ELIMINAR
winlink-to = Para
winlink-cc = Cc
winlink-subject-2 = Asunto
winlink-callsign-smtp-someone-example-org = Un indicativo, o SMTP:alguien@example.org para correo de internet. Separa varios con comas.
winlink-file-outbox = ARCHIVAR EN SALIDA
winlink-discard = DESCARTAR
winlink-filed-messages-go-out-next = los mensajes archivados salen en la próxima sesión
winlink-mail-log = REGISTRO DE CORREO
winlink-session-running = sesión en curso…
winlink-no-session-yet = aún no hay sesión

## wspr (src/app/panels/wspr.rs)

wspr-receptions = RECEPCIONES
wspr-save-wspr-reception-list-csv = Guardar la lista de recepciones de WSPR como CSV
wspr-wspr = WSPR
wspr-weak-signal-propagation-beacon = baliza de propagación de señal débil
wspr-time-left-two-minute-slot = Tiempo que queda en esta ranura de dos minutos. Al final de ella la grabación va al decodificador y llega la siguiente decisión de transmisión.
wspr-tx = ● TX
wspr-decoding = decodificando…
wspr-band = Banda
wspr-last-slot = Última ranura
wspr-nothing-heard = no se oyó nada
wspr-receive-only = solo recepción
wspr-next-slot = Siguiente ranura
wspr-waiting-radio-take-change = esperando a que la radio acepte el cambio…
wspr-hop-next = Saltar a continuación
wspr-transmit = TRANSMITIR
wspr-receive-only-2 = Solo recepción.
wspr-power = POTENCIA
wspr-what-you-actually-radiate = lo que realmente radias
wspr-goes-out-message-everyone-who = Esto sale en el mensaje, y todos los que te oyen juzgan el trayecto por ello — así que una cifra optimista aquí hace que sus mediciones sean erróneas además de las tuyas.
wspr-roam = ERRAR
wspr-pick-different-offset-inside-200 = Elige un desplazamiento distinto dentro de la ventana de 200 Hz en cada transmisión. Esta es la convención: doscientos hercios compartidos por todos solo funcionan si nadie se aparca en el medio. Desactivado mantiene donde está el cursor.
wspr-band-hop = SALTO DE BANDA
wspr-move-dial-band-band-between = Mueve el dial de banda en banda entre ranuras, para que un receptor muestree todo el espectro en lugar de una porción. Girar tú el VFO lo pausa — pulsa esto dos veces para reanudar.
wspr-upload = SUBIR
wspr-send-what-station-decodes-wsprnet = Envía a wsprnet.org lo que decodifica esta estación. Informar de lo que oyes es lo que hace de un receptor WSPR parte de la red; no pone nada en el aire.
wspr-who-heard-me = QUIÉN ME OYÓ
wspr-ask-wsprnet-org-every-few = Pregunta a wsprnet.org cada pocos minutos qué estaciones decodificaron esta. WSPR no tiene ningún tipo de acuse de recibo, así que esta es la única forma en que una baliza se entera de su alcance.
wspr-bands = BANDAS
wspr-none-selected-hopping-has-nowhere = ninguna seleccionada — el salto no tiene adónde ir
wspr-set-your-callsign-grid-general = Fija tu indicativo y locátor en la pestaña General de Ajustes antes de transmitir.

## packet (src/app/panels/packet.rs)

packet-monitor = MONITOR
packet-busy = OCUPADO
packet-frames-arrived-but-failed-their = Tramas que llegaron pero fallaron su secuencia de comprobación — una colisión, un desvanecimiento o una señal demasiado débil para leerse.
packet-setup = ⚙ CONFIG.
packet-station-callsign-speed-tx-delay = Indicativo de la estación, velocidad, retardo TX, la ruta de digipetidores, la baliza y el servidor KISS
packet-starting-packet-modem = iniciando el módem de packet…
packet-nothing-heard-yet = aún no se ha oído nada
packet-terminal = TERMINAL
packet-connected = {" "}CONECTADO{" "}
packet-calling = {" "}LLAMANDO{" "}
packet-idle = {" "}INACTIVO{" "}
packet-extended-sequence-numbers-window-up = Números de secuencia extendidos — una ventana de hasta 127 tramas.
packet-frames-sent-not-yet-acknowledged = Tramas enviadas y aún no acusadas por el otro extremo.
packet-retries-against-link-s-limit = Reintentos frente al límite de este enlace. Un contador que sube mientras las tramas sin acusar se quedan quietas es lo que parece un trayecto con desvanecimiento visto desde este lado.
packet-mail-window-using-link = La ventana CORREO está usando el enlace.
packet-one-radio-one-channel-one = Una radio, un canal, un enlace. Una sesión de Winlink y este terminal son dos formas de usarlo, y lo obtiene quien lo pida primero.
packet-digipeaters-nearest-first-separated-commas = Digipetidores, el más cercano primero, separados por comas — OE3XLR-1,OE3XMS-1. Déjalo vacío para una estación que puedes oír directamente, o para usar la ruta predeterminada de los ajustes de packet.
packet-disconnect = {" "}DESCONECTAR{" "}
packet-hang-up-properly-wait-far = Cuelga como es debido, y espera a que el otro extremo esté de acuerdo. Cambiar de modo en su lugar termina la sesión sin avisar a nadie.
packet-connect = {" "}CONECTAR{" "}
packet-mail-window-has-link-finish = La ventana CORREO tiene el enlace. Termina o detén antes esa sesión.
packet-station-has-no-callsign-yet = Esta estación aún no tiene indicativo — fija uno en CONFIG., sobre el panel del monitor. No se transmite nada hasta que esté fijado.
packet-call-station-connected-mode-node = Llama a esta estación en modo conectado — un nodo, un BBS u otro operador.
packet-needs-callsign-call = Necesita un indicativo al que llamar.
packet-clear = {" "}BORRAR{" "}
packet-empty-transcript-link-untouched-connected = Vacía la transcripción. El enlace no se toca — una estación conectada sigue conectada.
packet-not-connected-type-callsign-above = Sin conexión. Escribe arriba un indicativo y pulsa CONECTAR para llamar a un nodo o a un BBS.
packet-type-here = escribe aquí
packet-not-connected = sin conexión
packet-send = {" "}ENVIAR{" "}
packet-send-line-node-bbs-reads = Envía la línea. Un nodo o un BBS lee una línea cada vez.

## fsq (src/app/panels/fsq.rs)

fsq-speed = Velocidad
fsq-call = Indicativo
fsq-callsign-directed-fsqcall-messages-defaults = Indicativo para los mensajes dirigidos (FSQCALL); por omisión es tu indicativo
fsq-tx = ● TX
fsq-contacts = CONTACTOS
fsq-heard = OÍDAS
fsq-none = — ninguno —
fsq-allcall = ALLCALL
fsq-image = IMAGEN
fsq-clear = BORRAR
fsq-forget-pictures-received-so-far = Olvidar las imágenes recibidas hasta ahora
fsq-send-image = Enviar imagen…
fsq-right-click-remove = Clic derecho para quitar
fsq-remove-picture = Quitar esta imagen
fsq-listening = — escuchando —
fsq-message = Mensaje…
fsq-send = {" "}ENVIAR{" "}
fsq-call-cq = {" "}LLAMAR CQ{" "}
fsq-heard-2 = {" "}? oídas{" "}
fsq-clear-2 = {" "}BORRAR{" "}
fsq-fsq-contacts = Contactos FSQ
fsq-add = Añadir

## publicsdr (src/app/publicsdr.rs)

publicsdr-tab = + PESTAÑA
publicsdr-open-receiver-new-radio-own = Abrir este receptor como una radio nueva, en su propia pestaña
publicsdr-point-radio-receiver-replacing-whatever = Apunta *esta* radio al receptor, sustituyendo la interfaz que tiene ahora — se vuelve a preguntar antes, porque es el trabajo de configurar una radio entera
publicsdr-not-stated = no indicado
publicsdr-replace = SUSTITUIR
publicsdr-point-radio-you-are-receiver = Apunta la radio en que estás a este receptor
publicsdr-tab-instead = + PESTAÑA EN SU LUGAR
publicsdr-open-receiver-another-radio-leave = Abre el receptor como otra radio y deja esta en paz
publicsdr-cancel = CANCELAR
publicsdr-public-sdrs = SDR PÚBLICOS
publicsdr-available = DISPONIBLES
publicsdr-hide-receivers-are-full-ones = Oculta los receptores que están llenos, y aquellos cuyo operador no ha abierto ningún canal a aplicaciones que no sean un navegador
publicsdr-band = EN BANDA
publicsdr-only-receivers-cover-current-dial = Solo receptores que cubren la frecuencia actual del dial
publicsdr-low-bw = BW BAJO
publicsdr-take-spyserver-low-bandwidth-shape = Toma un SpyServer en su forma de bajo ancho de banda: una ventana I/Q estrecha que sigue al dial más la vista de banda propia del servidor, en lugar de megabits de I/Q de banda ancha. Sin efecto en un KiwiSDR, que solo tiene una forma.
publicsdr-refresh = ⟳ ACTUALIZAR
publicsdr-fetch-both-directories-again = Obtener de nuevo ambos directorios
publicsdr-name-place-antenna-band = nombre, lugar, antena, banda
publicsdr-clear-search = Borrar la búsqueda
publicsdr-fetching-receiver-lists = Obteniendo las listas de receptores…
publicsdr-machine-radio-attached-does-not = La máquina a la que está conectada esta radio no responde a preguntas de dispositivos, así que tampoco puede obtener las listas.
publicsdr-no-match = sin coincidencias
publicsdr-nothing-matches-try-turning-filter = nada coincide — prueba a volver a activar un chip de filtro
publicsdr-radio = esta radio
publicsdr-radio-s-configuration-has-not = La configuración de esta radio aún no ha llegado de la máquina en que está, así que no hay nada que apuntar al receptor — tómalo con + PESTAÑA en su lugar.
publicsdr-position-not-published-so-nothing = {" "}Su posición no está publicada, así que nada de lo que oiga se informará.

## hd (src/app/hd.rs)

hd-hd-radio = HD Radio
hd-waiting-receiver = esperando al receptor…
hd-not-hd-radio-set-mode = No está en HD Radio. Pon el modo en HD Radio en una emisión FM que lleve los sidecars digitales — el dial va en el centro de la portadora analógica, no a su lado.
hd-no-hd-radio-lock-digital = Sin enganche de HD Radio. Las bandas laterales digitales se transmiten unos 20 dB por debajo de la portadora analógica, así que una emisora escuchable puede seguir siendo demasiado ruidosa para decodificarse: el decodificador necesita unos segundos con una limpia.
hd-sync = SINC
hd-ofdm-frame-timing-recovered = Temporización de trama OFDM recuperada
hd-audio = AUDIO
hd-audio-frames-decoding = Tramas de audio decodificándose
hd-quality = CALIDAD
hd-lower = INFERIOR
hd-upper = SUPERIOR
hd-last-minute = último minuto
hd-waiting-locked-signal = esperando una señal enganchada
hd-mer-l = MER INF
hd-modulation-error-ratio-lower-digital = Relación de error de modulación de la banda lateral digital inferior.
hd-mer-u = MER SUP
hd-modulation-error-ratio-upper-digital = Relación de error de modulación de la banda lateral digital superior.
hd-cber = CBER
hd-channel-bit-error-ratio-after = Tasa de error de bit del canal tras el código interno. Por debajo de aproximadamente 1e-3 el código externo puede corregirla; por encima, el audio empieza a entrecortarse.
hd-offset = DESPLAZ.
hd-residual-carrier-frequency-offset-large = Desplazamiento residual de frecuencia de portadora. Grande y constante significa que la referencia del receptor está desviada, no la emisión.
hd-psmi = PSMI
hd-primary-service-mode-indicator-which = Primary Service Mode Indicator: cuál de los modos de servicio híbrido FM está transmitiendo la emisora.
hd-programme = PROGRAMA
hd-restricted-access = {" "}— acceso restringido

## bands (src/app/bands.rs)

bands-bands = BANDAS
bands-daylight = ☀ DÍA
bands-night = ☾ NOCHE
bands-no-forecast-yet-fetching-hamqsl = · aún sin pronóstico — obteniéndolo de HAMQSL.com
bands-forecast-download-off = · descarga del pronóstico desactivada (Ajustes → Interfaz)
bands-band = BANDA
bands-condx = CONDX
bands-paths = TRAYECTOS
bands-reach = ALCANCE
bands-best = MEJOR
bands-condx-global-forecast-rest-what = CONDX es un pronóstico global; el resto es lo que realmente se ha oído. Una fila vacía significa que no se decodificó nada — lo que puede significar que la banda estaba cerrada, o solo que nadie estaba en ella.
bands-ibp-beacons = BALIZAS IBP
bands-meteor-showers = LLUVIAS DE METEOROS
bands-peak = MÁXIMO

## spots (src/app/spots.rs)

spots-pota = POTA
spots-sota = SOTA
spots-freedv = FREEDV
spots-spots = SPOTS
spots-longwave-shortwave-broadcast-stations-air = Emisoras de radiodifusión de onda larga y corta en el aire ahora
spots-view = EN VISTA
spots-only-spots-inside-panadapter-span = Solo spots dentro del ancho del panadaptador
spots-setup = ⚙ CONFIG.
spots-feeds-lookup-upload-settings = Ajustes de flujos, consulta y subida
spots-call-station-site-frequency = indicativo, estación, sitio, frecuencia
spots-clear-search = Borrar la búsqueda
spots-no-match = sin coincidencias
spots-no-spots-enable-feed-setup = sin spots — activa un flujo en ⚙ CONFIG.
spots-nothing-matches-search = nada coincide con la búsqueda

## ism (src/app/ism.rs)

ism-ism-devices = DISPOSITIVOS ISM
ism-decoding = DECODIFICANDO
ism-needs-wideband-iq-source-cat = Necesita una fuente IQ de banda ancha; un equipo CAT solo envía audio
ism-how-far-burst-must-stand = Cuánto debe destacar una ráfaga sobre el suelo de ruido propio del canal antes de decodificarse
ism-give-each-band-width-normally = Da a cada banda el ancho que normalmente necesita — un cuarto de megahercio para las bandas OOK de 315, 345 y 433 MHz, uno entero para 868 y 915.
ism-several-hundred-more-device-decoders = Varios cientos de decodificadores de dispositivos más, del proyecto rtl_433, integrados. Lee los dispositivos OOK de 433 MHz a los que los decodificadores propios de sdroxide no llegan en absoluto. Donde ambos pueden leer un dispositivo, rtl_433 conoce muchas más variantes y toma el relevo.
ism-t-1-your-own = 1 propio
ism-device-decoders-registered-how-many = Decodificadores de dispositivos registrados, cuántos proceden de tu rtl433_flex.conf y cuántas transmisiones se han decodificado desde que se inició.
ism-reload-decoders = RECARGAR DECODIFICADORES
ism-re-read-rtl433-flex-conf = Vuelve a leer rtl433_flex.conf tras editarlo. Los dispositivos ya oídos permanecen en la lista.
ism-rtl-433-knows-more-variants = rtl_433 conoce más variantes de estos que los decodificadores propios de sdroxide, así que los lee mientras escucha aquí. Selecciona otra banda y los de sdroxide retoman el relevo.
ism-decoder-your-rtl433-flex-conf = Se rechazó un decodificador de tu rtl433_flex.conf. Los demás se cargaron. Corrige la línea y pulsa RECARGAR DECODIFICADORES.
ism-waiting-decoder = esperando al decodificador…
ism-where-decoder-actually-listening-follows = Dónde está escuchando realmente el decodificador. Esto sigue el centro de hardware del receptor, no el VFO — en una etapa de entrada ancha los dos pueden estar muy separados, y es la ventana la que decide qué canales son alcanzables.
ism-transmissions-gate-opened-how-many = Transmisiones en que se abrió la puerta, y cuántas produjeron una trama válida. Muchas ráfagas y ninguna decodificación significa que la banda está ocupada por dispositivos que sdroxide aún no sabe leer. Ninguna en absoluto significa que nada llega al umbral — bájalo, o comprueba que el dial está en la banda.
ism-nothing-heard-yet = aún no se ha oído nada
ism-click-tune-receiver =


    Haz clic para sintonizar el receptor en él

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

navtex-messages = MENSAJES
navtex-sync = SINC
navtex-character-phase-locked-signal-being = La fase de carácter está enganchada — se está leyendo una señal.
navtex-no-character-phase-yet-no = Aún no hay fase de carácter: no hay señal, o no es esta.
navtex-characters-whose-first-copy-was = Caracteres cuya primera copia estaba corrupta y se tomaron de la repetición cinco ranuras después, y aquellos en que ambas eran malas. Un carácter NAVTEX se envía dos veces; esa es toda su corrección de errores.
navtex-swap-mark-space-tones-signal = Intercambia los tonos de marca y espacio, para una señal recibida en la otra banda lateral. Desactivado es banda lateral superior en la frecuencia del canal, que es lo que dice toda instrucción de sintonía publicada del servicio.
navtex-starting-navtex-receiver = iniciando el receptor NAVTEX…
navtex-nothing-yet-station-transmits-ten = aún nada — una estación transmite durante diez minutos cada cuatro horas
navtex-cut-short = cortado
navtex-message = MENSAJE
navtex-time-day-stated-message-utc = Hora del día indicada en el mensaje (UTC): solo para mostrar
navtex-receiving = RECIBIENDO
navtex-monitor = MONITOR
navtex-everything-decoded-message-not = todo lo decodificado, sea o no un mensaje

## rfpaint (src/app/panels/rf_paint.rs)

rfpaint-rf-paint = RF PAINT
rfpaint-scan-speed = Velocidad de barrido
rfpaint-how-fast-text-image-scanned = Con qué rapidez se barre el texto/imagen sobre la cascada. Más bajo es más lento y más legible; 100 % = velocidad base, 25 % (centro) es el valor predeterminado.
rfpaint-abort = Abortar
rfpaint-text-paint = PINTAR TEXTO
rfpaint-type-text-paint = Escribe el texto que pintar…
rfpaint-preview-waterfall = VISTA PREVIA DE LA CASCADA
rfpaint-type-text-preview = escribe texto para previsualizar
rfpaint-transmit = {"  "}TRANSMITIR{"  "}
rfpaint-image-paint = PINTAR IMAGEN
rfpaint-no-image-loaded = ninguna imagen cargada
rfpaint-load-image = Cargar imagen…
rfpaint-load-image-preview = carga una imagen para previsualizar

## hfdl (src/app/hfdl.rs)

hfdl-listen = ESCUCHAR
hfdl-switches-engine-s-hfdl-downconverter = Activa o desactiva el conversor descendente y el decodificador HFDL del motor. Decodificar cuesta un carril de 24 kHz y un hilo de trabajo, esté o no abierta la ventana.
hfdl-channel-assigned-frequency-listen-anything = El canal (la frecuencia asignada) en que escuchar. Sirve cualquier punto de la banda — las estaciones HFDL transmiten de 2,8 a 22 MHz — con un carril fijo de 24 kHz centrado aquí.
hfdl-save-hfdl-decode-log-file = Guardar en un archivo el registro de decodificaciones HFDL
hfdl-riverhead-northern-atlantic-north-america = Riverhead (Atlántico norte/Norteamérica) — el valor predeterminado, y el canal con el que se validó el decodificador.
hfdl-hfdl-assigned-frequency = Una frecuencia asignada de HFDL.
hfdl-one-your-saved-channels-click = Uno de tus canales guardados — haz clic para sintonizar, clic derecho para quitar.
hfdl-hfdl-selected-but-decoding-off = HFDL está seleccionado pero la decodificación está desactivada — pulsa ESCUCHAR arriba
hfdl-decodes = DECODIFICACIONES
hfdl-nothing-decoded-yet-ground-station = Aún no se ha decodificado nada — el squitter de una estación terrestre se repite cada ~32 s una vez que hay una en el canal.
hfdl-no-decode-matches-filter = Ninguna decodificación coincide con el filtro.
hfdl-decoding = DECODIFICANDO
hfdl-decoding-off = DECODIFICACIÓN DESACTIVADA
hfdl-dbfs = — dBFS

## rade (src/app/panels/rade.rs)

rade-rade = RADE
rade-freedv-v1-digital-voice = Voz digital FreeDV V1
rade-tx = ● TX
rade-mute-analog = SILENCIAR ANALÓGICO
rade-mute-demodulated-audio-so-only = Silencia el audio demodulado, de modo que solo se oye la voz decodificada. La señal en bruto se deja pasar en caso contrario siempre que el módem no tiene nada que reproducir — ese siseo es como encuentras un turno antes de que se sincronice, así que déjalo desactivado mientras sintonizas.
rade-sync = SINC
rade-how-far-received-signal-sits = A qué distancia está la señal recibida de donde la espera el módem. Los valores grandes aún decodifican — el lazo de adquisición los sigue — pero mover el dial para acercar esto a cero da el mejor margen.
rade-snr = SNR —
rade-callsign-station-put-end-over =
    El indicativo que la estación puso en su trama de fin de turno — el único punto de un turno RADE en que se envía uno. Es también lo que se informa a FreeDV Reporter, de modo que una estación que oyes aparece en qso.freedv.org como oída por ti.

    Se mantiene un minuto después de que se va la señal, y se descarta cuando empiezas un turno propio: nombra a la estación que está en la frecuencia, no a todas las que ha oído alguna vez el receptor.
rade-stop-talking = DEJAR DE HABLAR
rade-talk-ptt = HABLAR (PTT)
rade-open-close-rade-over-modem = Abre o cierra un turno RADE. El módem necesita ~120 ms de voz antes de que salga la primera trama, y envía una trama de fin de turno cuando paras, así que la transmisión se prolonga un poco más allá del botón.
rade-rade-v1-occupies-roughly-1060 = RADE V1 ocupa aproximadamente 1060–1880 Hz de la banda pasante — banda lateral superior, o inferior en 160, 80 y 40 m, donde RADE mantiene la práctica de fonía y sdroxide cambia por ti. Pon la señal dentro de la banda sombreada de la cascada; el módem la encuentra desde ahí.

## frame (src/app/frame.rs)

frame-acknowledge = Confirmar
frame-dismiss = Descartar
frame-get-sdroxide-com = Consíguelo en sdroxide.com
frame-reconnecting = Reconectando…
frame-not-connected = Sin conexión.
frame-reconnect-now = Reconectar ahora
frame-transmit-lockout-disabled = ⚠ BLOQUEO DE TRANSMISIÓN DESACTIVADO
frame-engine-was-started-oob-tx = Este motor se inició con --oob-tx. El bloqueo de banda de aficionado está desactivado: activará el transmisor en cualquier frecuencia que admita el hardware.
frame-transmitting-outside-your-licence-offence = Transmitir fuera de tu licencia es una infracción en todos los países que la emiten. Continúa solo si estás autorizado a usar las frecuencias en las que vas a transmitir — una licencia MARS/CAP o comercial, un permiso experimental, o una carga ficticia.
frame-i-understand = {"  "}LO ENTIENDO{"  "}
frame-restart-without-oob-tx-put = Reinicia sin --oob-tx para volver a poner el bloqueo.

## smeter (src/widgets/smeter.rs)

smeter-alc-how-hard-transmitter-being = ALC — con cuánta fuerza se está excitando el transmisor, como porcentaje de lo máximo que puede entregar. 100 % es el fondo de escala, no la potencia completa en vatios: lo que eso vale en la antena depende de la radio y de la Excitación.
smeter-radio-reports-no-forward-power =


    Esta radio no informa de potencia directa, así que la cifra junto a ALC es la lectura de ALC otra vez y no vatios.
smeter-no-swr-radio-has-no =


    Sin ROE: esta radio no tiene puente con que medirla, y sdroxide no se la inventará — solo puede mostrar lo que informa la propia radio. Un equipo con medidor de ROE (por CAT, TCI o un enlace LAN) y una placa HPSDR la rellenan ambos; un transmisor SDR simple no puede.
smeter-no-signal-report-yet-receiver = Aún no hay informe de señal — el receptor no ha entregado una lectura del medidor.
smeter-s9-73-dbm-each-s =


    S9 son −73 dBm y cada unidad S por debajo son 6 dB. La cifra en dBm es tan honesta como la calibración de la etapa de entrada — un receptor sin calibrar informa dBFS con una etiqueta de dBm.
smeter-ovld-converter-into-rails-so =


    OVLD: el conversor está en sus topes, así que el nivel mostrado subestima la señal. Reduce la ganancia de la etapa de entrada.
smeter-table-held = , tabla mantenida
smeter-ps = PS --

## multi (src/multi.rs)

multi-tx = ● TX
multi-link = ENLACE
multi-mute-radio-s-audio = Silenciar el audio de esta radio
multi-close-radio-s-split-view = Cerrar la vista dividida de esta radio
multi-open-radio-split-view-own = Abrir esta radio en una vista dividida propia
multi-already-open-another-split-view = Ya está abierta en otra vista dividida
multi-show-radio-here = Mostrar esta radio aquí
multi-add-radio-computer-settings-radio = Añadir una radio en este ordenador (Ajustes → Radio para añadir una en una estación)
multi-add-radio = Añadir una radio
multi-asked-station-another-radio = Se ha pedido otra radio a la estación…
multi-client-cannot-open-connection = Este cliente no puede abrir una conexión.

## net (src/app/net.rs)

net-fail = ERROR
net-will-retry = {" "}(se reintentará)
net-full-resync-downloading = Resincronización completa: descargando…
net-downloading = Descargando…
net-full-resync-log-already-matches = Resincronización completa: el registro ya coincide con Wavelog
net-full-resync-review-changes-below = Resincronización completa: revisa los cambios de abajo
net-full-resync-discarded-nothing-changed = Resincronización completa descartada; no cambió nada
net-adif-import-failed-file-could = Error al importar ADIF: no se pudo analizar el archivo
net-channel-import-failed-file-could = Error al importar canales: no se pudo leer el archivo
net-channel-import-nothing-read-chirp = Importación de canales: nada que leer — un archivo CSV de CHIRP empieza con una fila de cabecera que nombra sus columnas, una de las cuales debe ser Frequency

## widgets (src/app/panels/widgets.rs)

widgets-free-text = texto libre
widgets-hashed-callsign-not-yet-heard = indicativo con hash, aún no oído completo
widgets-entity-unknown = entidad desconocida
widgets-band = esta banda
widgets-new-entity-never-worked-any = Entidad nueva — nunca trabajada, en ninguna banda
widgets-new-grid-square = Cuadrado de locátor nuevo
widgets-not-worked-before = No trabajado antes
widgets-worked-before-but-not-band = Trabajado antes, pero no en esta banda
widgets-call-queue = En la cola de llamadas
widgets-image = Imagen

## pi4 (src/app/panels/pi4.rs)

pi4-receptions = RECEPCIONES
pi4-save-pi4-reception-list-csv = Guardar la lista de recepciones de PI4 como CSV
pi4-next-generation-beacon = Next Generation Beacon
pi4-time-left-one-minute-beacon = Tiempo que queda en este ciclo de baliza de un minuto. El mensaje PI4 se busca una vez que la ventana ha tenido tiempo de llenarse más allá de donde terminaría.
pi4-decoding = decodificando…
pi4-band = Banda
pi4-last-cycle = Último ciclo
pi4-nothing-heard = no se oyó nada
pi4-receive-only-tune-so-beacon = Solo recepción. Sintoniza de modo que la identificación en CW de la baliza quede en 800 Hz de audio — la convención propia de la red — y los cuatro tonos PI4 caigan donde este decodificador los busca por omisión.

## awards (src/app/awards.rs)

awards-awards = DIPLOMAS
awards-band = Banda
awards-all = Todas
awards-dxcc = DXCC
awards-grids = Locátores
awards-worked-all-states = Worked All States
awards-cq-zones-waz = Zonas CQ (WAZ)
awards-dxcc-entities = Entidades DXCC

## acars (src/app/panels/acars.rs)

acars-acars = ACARS
acars-audio-decoder-s-passband = Audio en la banda pasante del decodificador.
acars-blocks-framed-blocks-whose-check = Bloques que se enmarcaron, y bloques cuya secuencia de comprobación falló.
acars-starting-acars-receiver = iniciando el receptor ACARS…
acars-channel = CANAL
acars-tune-dial-here = Sintonizar el dial aquí.
acars-nothing-yet-acars-quiet-between = Aún nada. ACARS está en silencio entre aeronaves — déjalo en un canal y espera una ráfaga.
acars-check-failed = comprobación fallida
acars-block-check-sequence-did-not = La secuencia de comprobación del bloque no coincidió — el texto puede ser erróneo.
acars-no-text = (sin texto)

## login (src/login.rs)

login-sdr-oxide = SDR OXIDE
login-username = Usuario
login-password = Contraseña
login-remember-device = Recordar en este dispositivo
login-signs-without-asking-next-time = Inicia sesión sin preguntar la próxima vez, y deja entrar sin preguntar a la pestaña de la vista 3D. La contraseña se guarda en claro, así que déjalo desactivado en un equipo que usen otras personas.
login-keeps-password-device-clear = Guarda la contraseña en este dispositivo, en claro.
login-checking = {" "}COMPROBANDO…{" "}
login-sign = {" "}INICIAR SESIÓN{" "}
login-wrong-password-shuts-server-s = Una contraseña incorrecta cierra la puerta del servidor durante unos segundos, así que esto puede tardar un momento.

## help (src/help.rs)

help-find = BUSCAR
help-find-manual = buscar en el manual
help-previous-match-shift-enter-shift = Coincidencia anterior (Mayús+Intro, Mayús+F3)
help-next-match-enter-f3 = Siguiente coincidencia (Intro, F3)
help-no-matches = sin coincidencias
help-clear-search-esc = Borrar la búsqueda (Esc)
help-sdroxide-manual = MANUAL DE SDROXIDE
help-contents = CONTENIDO

## aismap (src/ais_map.rs)

aismap-t-1-station-heard-waiting-report = 1 estación oída — esperando a que informe de una posición
aismap-double-click-reframe = doble clic para reencuadrar
aismap-zoom-out-wheel-pinch-do = Alejar — la rueda o un gesto de pellizco también lo hacen
aismap-zoom-about-middle-chart = Acercar alrededor del centro de la carta
aismap-frame-everything-being-tracked-again = Encuadrar de nuevo todo lo que se sigue — un doble clic también lo hace

## specview (src/widgets/spectrum_view.rs)

specview-waiting-spectrum = esperando espectro…
specview-hounds = HOUNDS

## spectrum (src/app/spectrum.rs)

spectrum-window-has-no-gpu-renderer = Esta ventana no tiene renderizador por GPU, así que la cascada se queda con su ancho estándar.
spectrum-machine-drawing-without-gpu-every = Esta máquina dibuja sin GPU — cada columna de la cascada es trabajo del procesador, que ya lo comparte con la radio.
spectrum-window-renders-through-opengl-which = Esta ventana se renderiza a través de OpenGL, que es la vía de compatibilidad de sdroxide — una Raspberry Pi, un chip gráfico antiguo o un navegador sin WebGPU. Una cascada más ancha no compensa la tasa de fotogramas allí.
spectrum-wider-than-renderer-can-draw = Más ancho de lo que puede dibujar este renderizador.

## macros (src/app/panels/macros.rs)

macros-nocall = NOCALL
macros-each-button-sends-whole-text = Cada botón envía todo su texto de golpe. F2–F10 pulsan los nueve primeros, siempre que nada en pantalla tenga el teclado.
macros-remove-button = Quitar este botón

## gridtracker (src/app/grid_tracker.rs)

gridtracker-grid-tracker = SEGUIMIENTO DE LOCÁTORES
gridtracker-shade-squares-heard-live-decode = Sombrea los cuadrados oídos en la lista de decodificaciones en vivo, no solo los del diario — lo que está en el aire ahora, en cian bajo los cuadrados trabajados.
gridtracker-drag-pan-wheel-zoom-amber = Arrastra para desplazar, rueda para hacer zoom. Ámbar = trabajado, verde = confirmado, cian = oído.

## remote (src/remote.rs)

remote-server-busy-another-client-connected = servidor ocupado — hay otro cliente conectado
remote-transmitting-but-client-s-microphone = Transmitiendo, pero el micrófono de este cliente no produce ningún audio — no se está modulando nada. En un navegador, comprueba que se permitió el micrófono a la página y que ninguna otra aplicación lo retiene.
remote-connection-closed = conexión cerrada

## solarwin (src/solar3d/mod.rs)

solarwin-sdroxide-solar-system = sdroxide — sistema solar

## solarbtn (src/app/solar.rs)

solarbtn-solar-system-3d-view-sun = Vista 3D del sistema solar — Sol, Tierra, Luna, manchas solares y CME (ventana aparte)
solarbtn-solar-system-3d-view-sun-2 = Vista 3D del sistema solar — Sol, Tierra, Luna, manchas solares y CME (pestaña nueva del navegador)

## widespec (src/widgets/wide_spectrum.rs)


## speech (src/app/speech.rs)

speech-loading-voice = Cargando la voz…
speech-browser-client-cannot-speak-yet = el cliente de navegador aún no puede hablar

## freqdisplay (src/widgets/freq_display.rs)

freqdisplay-double-click-type-frequency = Haz doble clic para escribir una frecuencia
freqdisplay-tap-type-frequency = Toca para escribir una frecuencia

## solarstate (src/solar3d/state.rs)

solarstate-home = INICIO
solarstate-dwarf-planets = PLANETAS ENANOS

## adsbmap (src/adsb_map.rs)

adsbmap-double-click-reframe = doble clic para reencuadrar

## aprsmap (src/aprs_map.rs)

aprsmap-double-click-reframe = doble clic para reencuadrar

## solarnet (src/solar3d/net.rs)

solarnet-connection-closed = conexión cerrada

## alerts (src/app/alerts.rs)

alerts-could-not-start-alert-thread = no se pudo iniciar el hilo de alertas

## hfdlmap (src/hfdl_map.rs)

hfdlmap-no-aircraft-located-yet = aún no se ha localizado ninguna aeronave

## worldmap (src/widgets/worldmap.rs)

worldmap-double-click-reframe = DOBLE CLIC PARA REENCUADRAR

## chrome (src/chrome.rs)



## duopower (src/app/duo_power.rs)

duopower-sets-fdm-duo-s-own = Fija el ajuste de potencia propio del FDM-DUO (menú 34 TX POWER). La Excitación y Tune fijan entonces la potencia dentro de él, y SDRoxide vuelve a enviar el ajuste cada vez que conecta.

## Band conditions window: a peak date.

bands-peak-date = { $day } { $month ->
        [1] ene
        [2] feb
        [3] mar
        [4] abr
        [5] may
        [6] jun
        [7] jul
        [8] ago
        [9] sep
        [10] oct
        [11] nov
       *[12] dic
    }

## download (src/download.rs)

download-browser-could-not-read-file = el navegador no pudo leer ese archivo

## morse (src/app/morse.rs)

morse-translate = TRADUCIR
morse-practice = PRÁCTICA
morse-learn = APRENDER
morse-morse = MORSE
morse-text-morse = Texto → Morse
morse-morse-text = Morse → Texto
morse-per-character-space-between-characters = `.` y `-` por carácter, un espacio entre caracteres, ` / ` entre palabras. El alfabeto UIT, dígitos, puntuación y los prosignos habituales.
morse-reference-translator-available-desktop-build = El traductor de referencia está disponible en la versión de escritorio.
morse-play-what-you-type-your = Reproduce lo que escribes, por tus propios altavoces
morse-play = REPRODUCIR
morse-stop = PARAR
morse-every-character-learned-well-done = Todos los caracteres aprendidos — bien hecho.
morse-drill-needs-sound-which-desktop = El ejercicio necesita sonido, que tiene la versión de escritorio.
morse-reveal = REVELAR
morse-reset = REINICIAR
morse-replay = REPETIR
morse-what-did-you-hear = ¿Qué oíste?
morse-check = COMPROBAR
morse-press-new-hear-character = Pulsa NUEVO para oír un carácter.
morse-tone = Tono
morse-speed = Velocidad
morse-spacing = Espaciado
morse-characters-go-out-speed-above = Los caracteres salen a la velocidad de arriba; esta es la velocidad a la que se envían los *huecos*, para que un principiante oiga la letra entera antes de poder enviarla tan deprisa.
morse-playback-available-desktop-build = La reproducción está disponible en la versión de escritorio.

## ADS-B panel: aircraft detail rows.

adsb-row-altitude = altitud
adsb-row-track = rumbo
adsb-row-turning = girando
adsb-row-squawk = squawk
adsb-row-range = distancia
adsb-row-position = posición
adsb-row-signal = señal
adsb-row-frames = tramas
rade-transmitting = transmitiendo
decodes-odd = IMPAR
decodes-new = NUEVA
js8-new = NUEVA

## AIS panel: vessel detail rows.

ais-row-status = estado
ais-row-speed = velocidad
ais-row-course = rumbo
ais-row-heading = proa
ais-row-turning = girando
ais-row-altitude = altitud
ais-row-size = tamaño
ais-row-draught = calado
ais-row-destination = destino
ais-row-range = distancia
ais-row-signal = señal
ais-row-type = tipo
ais-row-position = posición
ais-row-messages = mensajes
ism-off = OFF
textmodem-frequency-shifted = desplazado en frecuencia
winlink-to-heading = PARA

## adsbmap: messages with values (src/adsb_map.rs)

adsbmap-ft-min =
    {""}
    { $vr } ft/min

## aismap: messages with values (src/ais_map.rs)

aismap-heard-no-position-newer-than = { $heard } oídas — ninguna posición más reciente que { $drop_map_s } s
aismap-stations-heard-waiting-position-report = { $heard } estaciones oídas — esperando un informe de posición
aismap-km-across = { $lon_span }° · ~{ $km } km de ancho

## alerts: messages with values (src/app/alerts.rs)

alerts-alarming = alarma sonando en { $device }
alerts-alarm-device-unavailable = dispositivo de alarma no disponible: { $error }

## awards: messages with values (src/app/awards.rs)

awards-worked = { $w } trabajadas
awards-confirmed = { $value } confirmadas

## bands: messages with values (src/app/bands.rs)

bands-forecast-hamqsl-com-old = · pronóstico de HAMQSL.com, de hace { $value }
bands-ncdxf-iaru-next-beacon-s = NCDXF/IARU · próxima baliza en { $now } s
bands-hearing-means-path-open-each = { $callsign } en { $location } — oírla significa que el trayecto de { $band } está abierto. Cada baliza sube una banda cada 10 s, así que esta fila cambia en cada ranura.
bands-radiants-placed = radiantes situados para { $lat }°, { $lon }°
bands-zhr = ZHR { $zhr }
bands-radiant = radiante { $alt_deg }° { $az_deg }
bands-radiant-down = radiante bajo el horizonte ({ $alt_deg }°)
bands-km-s-parent-peak-rate =
    { $value } ({ $code }) — { $velocity_kms } km/s, origen { $parent }. Tasa máxima ZHR { $zhr } en torno a { $peak }.

    Un radiante sobre el horizonte significa que las estelas pueden llegarte; una lluvia rápida deja estelas ionizadas más duraderas para la dispersión por meteoros en 6 m y 2 m, y las breves aperturas de 10 m.

## drm: messages with values (src/app/drm.rs)

drm-not-decoding = { $channel } sin decodificar
drm-symbols = { $value } símbolos · { $frame }
drm-not-decodable = { $value } — no decodificable
drm-broadcaster-s-clock-utc = reloj de la emisora { $year }-{ $month }-{ $day } { $hour }:{ $minute } UTC

## duopower: messages with values (src/app/duo_power.rs)

duopower-max-fdm-duo-s-full =
    MAX: la salida completa del FDM-DUO, unos 8 W. El ajuste a usar con el HR500 en línea.

    { $base }
duopower-t-5-w-hr500-line-will =
    5 W. Con el HR500 en línea no llegará a la salida completa en la mayoría de las bandas con este ajuste: usa MAX, y la Excitación para bajarla.

    { $base }
duopower-w-running-barefoot-qrp =
    { $code } W, para trabajar sin amplificador o en QRP.

    { $base }

## frame: messages with values (src/app/frame.rs)

frame-re-enables-transmit-after-1 = Reactiva la transmisión tras { $swr }:1. Revisa primero la antena: si el fallo sigue ahí, la próxima transmisión también se detendrá.
frame-sdroxide-has-been-released = Se ha publicado SDRoxide { $version } — esta es { $sdroxide_version }.
frame-reconnecting-s = Reconectando en { $value } s…
## gridtracker: messages with values (src/app/grid_tracker.rs)

gridtracker-worked = { $worked } trabajados
gridtracker-confirmed = { $confirmed } confirmados
gridtracker-heard = { $heard } oídos

## hd: messages with values (src/app/hd.rs)

hd-decode = Decodificar { $value } ({ $codec_label })

## hfdl: messages with values (src/app/hfdl.rs)

hfdl-khz-already-has-chip = { $cur_khz } kHz ya tiene un chip
hfdl-save-current-channel-khz-one = Guardar el canal actual ({ $cur_khz } kHz) como uno de tus chips
hfdl-aircraft = { $aircraft } aeronaves
hfdl-bursts = { $bursts } ráfagas
hfdl-decodes-2 = { $decodes } decodificaciones
hfdl-fixed = { $fec } corregidas

## ism: messages with values (src/app/ism.rs)

ism-not-decoded-yet = { $hint } — aún no decodificado
ism-listen-tunes-radio-there-one = Escuchar en { $value } — sintoniza allí la radio. Una banda cada vez: están demasiado separadas para que ningún receptor cubra dos a la vez.
ism-watch-hz-around-band-centre = Vigila { $value } Hz alrededor del centro de la banda, pida lo que pida la banda. Más estrecho se ajusta a un receptor que no puede entregar el ancho propio de la banda; más ancho alcanza dispositivos más alejados del centro. Tu receptor tiene que entregar aproximadamente un tercio más que esto.
ism-rtl-433 = rtl_433 — { $why }
ism-rtl-433-khz = rtl_433 { $value } · { $khz } kHz
ism-your-own = { $flex } propios
ism-decoders-decoded = { $decoders } decodificadores ({ $flex }), { $decodes } decodificadas
ism-handled-rtl-433 = gestionado por rtl_433: { $names }
ism-flex = flex: { $error }
ism-window-mhz-mhz-wide = ventana { $mhz } MHz, { $e62 } MHz de ancho
ism-bursts-decoded = { $bursts } ráfagas, { $decodes } decodificadas
ism-devices = { $ism_reports } dispositivos
ism-id-mhz-db-heard-first =
    { $protocol } id { $device }
    { $e6 } MHz { $snr_db } dB oído { $value }×
    oído por primera vez hace { $first_at }

## logbook: messages with values (src/app/logbook.rs)

logbook-confirmations-downloaded-newly-confirmed = Confirmaciones: { $recs } descargadas, { $matched } recién confirmadas
logbook-qso = { $qso_log } QSO
logbook-worked-before = ⚠ TRABAJADO ANTES ({ $dupe_band })
logbook-utc-qso = { $oldest }–{ $newest } UTC · { $value } QSO

## app: messages with values (src/app/mod.rs)

app-radio = radio { $id }

## morse: messages with values (src/app/morse.rs)

morse-could-not-start-morse-audio = no se pudo iniciar el hilo de audio del Morse: { $error }
morse-no-audio-output = sin salida de audio: { $error }
morse-about-s = unos { $farnsworth_wpm } s
morse-characters-play = Caracteres en juego: { $set }
morse-score-run = Puntuación: { $correct } / { $total }. Racha: { $streak } de { $value }.
morse-correct = correcto — { $value }
morse-was = era { $value }

## net: messages with values (src/app/net.rs)

net-synced-z-new-already-log = Sincronizado { $when }Z: { $added } nuevos, { $matched } ya en el registro
net-wavelog-2 = Wavelog: { $line }
net-full-resync-stopped-nothing-changed = Resincronización completa detenida, no cambió nada: { $error }
net-full-resync-applied-added-updated = Resincronización completa aplicada: { $add } añadidos, { $update } actualizados, { $remove } quitados, { $mark } de los tuyos marcados como presentes en Wavelog
net-wavelog-3 = Wavelog: { $line }
net-adif-import-failed = Error al importar ADIF: { $error }
net-not-unicode-read = {" "}(no es Unicode; leído como { $enc })
net-received-reports-swl-left-out = , { $n } informes recibidos (SWL) omitidos
net-adif-import-added-duplicates-skipped = Importación ADIF: { $added } añadidos, { $skipped } duplicados omitidos{ $reports }{ $assumed }
net-channel-import-failed = Error al importar canales: { $error }
net-line-s-skipped = , { $n } línea(s) omitida(s)
net-not-unicode-read-2 = {" "}(no es Unicode; leído como { $enc })
net-channel-import-channel-s-read = Importación de canales: { $n } canal(es) leído(s){ $skipped }{ $assumed }

## acars: messages with values (src/app/panels/acars.rs)

acars-frames-bad = { $frames } tramas · { $bad } erróneas
acars-mode = modo { $mode }

## adsb: messages with values (src/app/panels/adsb.rs)

adsb-aircraft-3 = { $aircraft } aeronaves
adsb-frames = { $frames } tramas
adsb-preambles = { $preambles } preámbulos
adsb-bad-crc = { $bad_crc } con CRC erróneo
adsb-tune = SINTONIZAR { $mhz }
adsb-ft-barometric = { $ft } ft barométrica

## ais: messages with values (src/app/panels/ais.rs)

ais-vessels-3 = { $vessels } buques
ais-messages = { $messages } mensajes
ais-slots = { $bursts } ranuras
ais-bad-fcs = { $bad_fcs } con FCS erróneo
ais-ais-mhz = AIS { $value } ({ $mhz } MHz) — { $value2 }
ais-ais-mhz-slots-messages-floor = AIS { $value } ({ $mhz } MHz) — { $bursts } ranuras, { $messages } mensajes, suelo { $floor_dbfs } dBFS
ais-ais = AIS { $value }
ais-tune = SINTONIZAR { $mhz }
ais-ships-heard-are-khz-off = los barcos oídos están { $khz } kHz fuera de frecuencia ({ $mhz } ppm a 162 MHz) — fija la corrección de frecuencia de la etapa de entrada, o las transmisiones de más de unos 5 kHz dejarán de decodificarse
ais-over-ground = { $value }° sobre el fondo
ais-dbfs-db-snr = { $rssi_dbfs } dBFS, { $snr_db } dB SNR
ais-last-type-ais = { $messages } (último tipo { $last_type }, AIS { $channel })
ais-ais-2 = AIS { $ch }
ais-mhz-marine-channel-ship-alternates = { $mhz } MHz — canal marino { $ch }. Un barco alterna entre los dos, así que desactivar uno reduce a la mitad la frecuencia con que se oye cada buque.

## aprs: messages with values (src/app/panels/aprs.rs)

aprs-object = objeto de { $reported_by }
aprs-via = vía { $via }
aprs-object-reported = objeto informado por { $reported_by }
aprs-digit-s-blanked = ± { $ambiguity } dígito(s) en blanco
aprs-wind-m-s = viento { $value }° { $value2 } m/s
aprs-gust = ráfaga { $value }
aprs-rain-mm-h = lluvia { $value } mm/h
aprs-frame-s-last-ago = { $packets } trama(s), última hace { $last_heard }
aprs-via-2 = vía { $via }
aprs-bad = { $bad_frames } erróneas
aprs-other = { $non_aprs } otras
aprs-sent = enviado ×{ $tries }
aprs-stn = { $stations } est.
aprs-queued = { $tx_queue } en cola

## atchat: messages with values (src/app/panels/atchat.rs)

atchat-master = maestro { $value }
atchat-last-heard-s-ago-click = { $status } — oído por última vez hace { $age_s } s · haz clic para abrir una pestaña de mensaje directo
atchat-close-tab = cerrar la pestaña de { $peer }
atchat-direct-message = mensaje directo a { $value }
atchat-cannot-display = no se puede mostrar { $path }

## cw: messages with values (src/app/panels/cw.rs)

cw-click-decoder-neural-one-copies =
    { $hint }

    Haz clic para el decodificador { $neural }. El neuronal copia más abajo y lee CW enviado a mano que un ajuste de temporización no acepta; el de temporización es el único que copia las letras acentuadas — Ä, Ö, Å, Ü, É — porque el modelo no tiene clase de salida para ellas.
cw-idle-s = REPOSO { $idle } s

## decodes: messages with values (src/app/panels/decodes.rs)

decodes-save-decodes-list-adif-received = Guarda en ADIF las { $n } decodificaciones de la lista — informes recibidos, no contactos. Se omiten las decodificaciones que no nombran remitente.
decodes-save-decodes-list-csv-one = Guarda en CSV las { $n } decodificaciones de la lista, una fila cada una
decodes-session-qso = Sesión: { $session } QSO
decodes-save-whole-logbook-qso-adif = Guardar todo el diario ({ $logged } QSO) como ADIF
decodes-save-whole-logbook-qso-text = Guardar todo el diario ({ $logged } QSO) como texto
decodes-hz-slots = { $audio_hz } Hz · { $odd } ranuras
decodes-your-slot-timing-against-stations =
    La temporización de tus ranuras frente a las estaciones que estás oyendo.
    { $after }

    Cubre todo el camino de recepción, así que una cadena lenta de audio o de red cuenta igual que un reloj equivocado. Por debajo de 0,5 s es cómodo.
decodes-pile-up = PILE-UP { $fox_queue }
decodes-queue = COLA { $call_queue }
decodes-db-hz-click-remove =
    { $waiting } · { $snr_db } dB · { $audio_hz } Hz{ $unwrap_or_default }
    Haz clic para quitar

## js8: messages with values (src/app/panels/js8.rs)

js8-every-speed-being-decoded-marks = Se decodifican todas las velocidades; ▸ marca { $speed }, aquella con la que transmites
js8-every-speed-being-decoded-click = Se decodifican todas las velocidades. Haz clic para transmitir en su lugar a { $speed }
js8-transmit-decode = Transmitir y decodificar a { $speed }
js8-beaconing-every-min-click-stop = Emitiendo baliza cada { $hb_min } min — haz clic para parar
js8-last-beacon-went-out-hz = La última baliza salió a { $hz } Hz — una ranura libre de la subbanda de heartbeat de { $value }–{ $value2 } Hz, elegida para que caiga lejos de las señales que se están decodificando.
js8-frames-s = { $tx_frames_pending }/{ $tx_frames_total } tramas · { $left } s
js8-decoded = Decodificado a { $speed }
js8-frames = … ({ $frames } tramas)
js8-reply-2 = Responder a { $value }: «{ $value2 }»
js8-address-composer = Dirigir el compositor a { $value }

## macros: messages with values (src/app/panels/macros.rs)

macros-f-sends = F{ $i }: envía «{ $text }»
macros-sends = Envía «{ $text }»
macros-most = { $value } es el máximo

## navtex: messages with values (src/app/panels/navtex.rs)

navtex-repaired-lost = { $repaired } reparados · { $lost } perdidos
navtex-characters-lost = { $lost } caracteres perdidos

## packet: messages with values (src/app/panels/packet.rs)

packet-bad = { $bad_frames } erróneas
packet-link-layer = La capa de enlace está en { $link }
packet-via = {" "}vía { $via }
packet-unacked = { $unacked } sin acusar
packet-retry = reintento { $retries }

## pi4: messages with values (src/app/panels/pi4.rs)

pi4-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] baliza
       *[other] balizas
    }
pi4-utc-ago = { $t }:{ $t2 } UTC — hace { $slot_utc }

## rade: messages with values (src/app/panels/rade.rs)

rade-snr-db = SNR { $snr_db } dB
rade-offset-hz = desplazamiento { $freq_offset_hz } Hz
rade-heard = oída { $call }
rade-samples-dropped-between-receiver-decoder = ⚠ { $dropped } muestras perdidas entre el receptor y el decodificador — esta máquina no sigue el ritmo de la decodificación neuronal.

## textmodem: messages with values (src/app/panels/text_modem.rs)

textmodem-char-s = { $chars_per_sec } car/s
textmodem-char-s-hz-wide = { $value } — { $chars_per_sec } car/s, { $bandwidth_hz } Hz de ancho, { $keyed }

## vdl2: messages with values (src/app/panels/vdl2.rs)

vdl2-frames = { $frames } tramas
vdl2-bursts = { $bursts } ráfagas
vdl2-sync = { $syncs } sinc
vdl2-hdlc-bad = { $hdlc_bad } HDLC erróneas
vdl2-bad-fcs = { $fcs_bad } con FCS erróneo
vdl2-mhz-bursts-frames-noise-floor = { $mhz } MHz, { $what } — { $bursts } ráfagas, { $frames } tramas, suelo de ruido { $floor_dbfs } dBFS
vdl2-tune = SINTONIZAR { $mhz }
vdl2-held-2 = RETENIDO +{ $since }
vdl2-use-worldwide-where-every-link = { $what } — en uso en todo el mundo, y donde empieza cada enlace. El que hay que conservar si solo conservas uno.
vdl2-assigned-which-channels-carry-anything = Asignado a un { $what }. Qué canales llevan algo depende de dónde estés: déjalos todos activados salvo que sepas otra cosa.
vdl2-mhz-db-snr-dbfs-evm = { $mhz } MHz · { $snr_db } dB SNR · { $rssi_dbfs } dBFS · EVM { $evm_deg }° · { $freq_err_hz } Hz · { $rs_corrected } símbolos RS corregidos
vdl2-characters = { $parity_errors } caracteres
vdl2-parameter-sdroxide-does-not-read = { $unknown ->
        [one] { $unknown } parámetro que SDRoxide no lee
       *[other] { $unknown } parámetros que SDRoxide no lee
    } — mostrado arriba como identificador y bytes

## wefax: messages with values (src/app/panels/wefax.rs)

wefax-lines = { $lines } líneas
wefax-lines-held = { $live_size } líneas retenidas
wefax-pixels-per-line = { $width } píxeles por línea
wefax-shift-picture-pixels = Desplazar la imagen { $px } píxeles
wefax-charts-are-saved-click-copy =
    Las cartas se guardan { $place }

    Haz clic para copiar la ruta
wefax-finished-charts-are-saved-collect = Las cartas terminadas se guardan { $place } y se acumulan aquí.
wefax-pixels-right-click-delete =
    { $when_full }
    { $size } × { $size2 } píxeles
    { $value }

    Clic derecho para eliminar
wefax-older-store = { $older } más antiguas en el almacén
wefax-older-load-more = { $older } más antiguas — cargar más
wefax-published-carrier-khz-dial-khz = Portadora publicada { $f } kHz → dial { $value } kHz USB
wefax-delete-chart-2 = Eliminar esta carta { $dir }

## widgets: messages with values (src/app/panels/widgets.rs)

widgets-cq-zone-itu-zone = { $continent } · zona CQ { $cq_zone } · zona UIT { $itu_zone }
widgets-new-entity = Entidad nueva en { $band_label }
widgets-worked-before = Trabajada antes en { $band_label }
widgets-calling-cq-includes-you = Llamando CQ { $target } — eso te incluye
widgets-calling-cq-not-aimed-you = Llamando CQ { $target } — no dirigido a ti

## wspr: messages with values (src/app/panels/wspr.rs)

wspr-rx-heard-us = { $heard_us } rx · { $heard_us2 } nos oyeron
wspr-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] baliza
       *[other] balizas
    }
wspr-beaconing-duty = emitiendo baliza — { $wspr_power_dbm }, { $wspr_tx_percent } % de ciclo
wspr-listening-duty = escuchando ({ $wspr_tx_percent } % de ciclo)
wspr-beacon-one-two-minute-slot = Emite baliza en una ranura de dos minutos de cada { $pct } — el { $pct2 } % de ellas, espaciadas por igual — a { $wspr_power_dbm }. La ranura en que empieza el ciclo se sortea a partir de tu indicativo, así que dos estaciones con este programa no empiezan juntas.
wspr-transmitting-set-general-tab-settings = Transmitiendo como { $call } en { $sent } — fijado en la pestaña General de Ajustes.
wspr-utc-ago = { $t }:{ $t2 } UTC — hace { $slot_utc }

## publicsdr: messages with values (src/app/publicsdr.rs)

publicsdr-antenna =
    { $address }
    { $device }
    antena: { $antenna }
    { $e3 }
publicsdr-receivers-fetched-ago = { $entries } receptores · { $per_net } · obtenidos hace { $age }
publicsdr-showing-search-narrow = mostrando { $shown } de { $n } — busca para acotar
publicsdr-match = { $n } coincidencia
publicsdr-replace-taking-receiver-points-same =
    ¿Sustituir { $this } por { $entry }?

    { $this2 } está en { $iface }. Tomar este receptor apunta la misma pestaña a { $address }, la renombra «{ $entry2 }» y limita el dial a { $range_label } que publica el receptor — aquí no se transmite nada.

    Los ajustes de { $iface2 } se quedan donde están, y también los rangos indicados para ellos: volver a cambiar la interfaz en Ajustes → Radio devuelve la radio tal como estaba.
publicsdr-receptions-will-reported = {" "}Las recepciones se informarán desde { $value }.
publicsdr-pointing-radio = Apuntando esta radio a { $entry }…{ $site }
publicsdr-opening-another-radio = Abriendo { $entry } como otra radio…

## qo100: messages with values (src/app/qo100.rs)

qo100-khz-3 = +{ $park_lo_hz } … +{ $park_hi_hz } kHz
qo100-before-switching-tune-so-beacon = Antes de ACTIVAR: sintoniza de modo que los dos lóbulos de la baliza queden entre +{ $park_lo_hz } y +{ $park_hi_hz } kHz en la franja (el carril sombreado).
qo100-receiver-currently-listening-around-mhz = El receptor está escuchando ahora en torno a { $mhz } MHz — lejos de la baliza de 10489,750 MHz, así que aquí no hay nada que dibujar. El decodificador sigue buscando de todos modos.
qo100-receiver-currently-covers-only-khz = el receptor cubre ahora solo ±{ $khz } kHz aquí — el resto de la franja queda en blanco
qo100-null-db-sym-snr-db = nulo { $est_null_depth_db } dB sim { $est_symmetry } snr { $est_snr_db } dB
qo100-hz-s-hz-s-de = { $est_drift_hz_s } Hz/s { $est_drift_accel_hz_s2 } Hz/s² (desrotado antes de decodificar)
qo100-hz-s-de-rotated-before = { $est_drift_hz_s } Hz/s (desrotado antes de decodificar)
qo100-found-empty = { $est_updates } encontradas, { $est_misses } vacías
qo100-last-s-ago = {"   "}última { $auto_last_hz } (hace { $auto_last_unix } s)
qo100-over-correction = { $auto_total_hz } en { $auto_applies } { $auto_applies ->
        [one] corrección
       *[other] correcciones
    }{ $last }
qo100-sync-32-err = sinc { $sync_bit_errors } / 32 err ×{ $sync_matches }
qo100-frame-buffer = búfer de trama { $frame_fill } %
qo100-blocks-tried-decoded = { $blocks_tried } bloques probados, { $blocks_locked } decodificados
qo100-last-applied-s-ago-hz = última aplicada hace { $ago } s: { $old } → { $new } Hz ({ $old2 } Hz)
qo100-locked-block-tried-locked = enganchado — { $blocks_tried } { $blocks_tried ->
        [one] bloque
       *[other] bloques
    } probados, { $blocks_locked } enganchados
qo100-searching-block-tried-none-locked = buscando — { $blocks_tried } { $blocks_tried ->
        [one] bloque
       *[other] bloques
    } probados, ninguno enganchado aún

## rds: messages with values (src/app/rds.rs)

rds-auto = Auto ({ $standard })
rds-type = tipo { $class }
rds-groups = · { $groups } grupos
rds-blocks-error = · { $ber } % de bloques con error
rds-clean-corrected-lost = { $blocks_ok } limpios · { $blocks_corrected } corregidos · { $blocks_bad } perdidos

## sat: messages with values (src/app/sat.rs)

sat-utc-local = { $unix } UTC ({ $offset } local)
sat-aos = AOS { $rise_unix } { $rise_unix2 }
sat-los = LOS { $set_unix } { $set_unix2 }
sat-above-horizon = { $rise_unix } sobre el horizonte
sat-min = en { $value } min
sat-h-min = en { $value } h { $value2 } min
sat-searching = buscando… { $next } de { $entries }
sat-locked-2 = ● ENGANCHADO — { $t }
sat-pass-until-max = Paso hasta { $set_unix } · máx. { $max_el }°
sat-next-pass-rises-max = Próximo paso { $rise_unix } { $rise_unix2 } ({ $now }) · sale { $rise_az }° { $rise_az2 } · máx. { $max_el }°
sat-rigctld-client-connected-satellite-software = ⚠ { $clients } { $clients ->
        [one] cliente rigctld conectado
       *[other] clientes rigctld conectados
    } — un programa de satélites que orientara allí el dial corregiría el Doppler dos veces
sat-rotator-antenna-az-el = Rotor: antena a { $az }° de acimut { $el }° de elevación
sat-rotator-2 = Rotor: { $error }
sat-up-now-2 = ● visible ahora, { $el }°
sat-inv = { $mode } · inv
sat-norad-2 = NORAD { $norad_id }

## scanner: messages with values (src/app/scanner.rs)

scanner-holding-mhz = retenido en { $here } MHz
scanner-scanning-mhz = barriendo · { $here } MHz

## spectrum: messages with values (src/app/spectrum.rs)

spectrum-renderer-will-not-hold-texture = Este renderizador no admite una textura de más de { $max_texture_dim } píxeles de ancho.
spectrum-mem = Mem: { $f } / { $value }
spectrum-mem-2 = Mem: { $value }

## speech: messages with values (src/app/speech.rs)

speech-voice-2 = Voz: { $voice }
speech-speech-unavailable = La voz no está disponible: { $error }

## spots: messages with values (src/app/spots.rs)

spots-transmissions = { $stations } transmisiones

## winlink: messages with values (src/app/winlink.rs)

winlink-last-session-failed = la última sesión falló — { $err }
winlink-last-session-received-sent = última sesión: { $last_received } recibidos, { $last_sent } enviados
winlink-empty = { $to_lowercase } está vacío
winlink-showing = mostrando { $entries } de { $total }
winlink-fetching = obteniendo { $mid }…
winlink-from-2 = de { $msg } { $date }
winlink-attachment-bytes = adjunto: { $att } ({ $data } bytes)

## aprsmap: messages with values (src/aprs_map.rs)

aprsmap-object =
    {""}
    objeto de { $reported_by }
aprsmap-heard-ago =
    {""}
    oído hace { $age }

## download: messages with values (src/download.rs)

download-reading = leyendo { $path }: { $error }

## help: messages with values (src/help.rs)

help-missing-image = {"["}falta la imagen: { $path }]

## hfdlmap: messages with values (src/hfdl_map.rs)

hfdlmap-mhz =
    {""}
    { $e3 } MHz
hfdlmap-fix =
    {""}
    { $fixes } { $fixes ->
        [one] posición
       *[other] posiciones
    }
hfdlmap-heard-ago =
    {""}
    oído hace { $last_at }

## multi: messages with values (src/multi.rs)

multi-radio = Radio { $n }
multi-could-not-add-radio = No se pudo añadir una radio: { $error }
multi-was-closed-station = { $value } se cerró en la estación.
multi-could-not-open = No se pudo abrir { $peer }: { $error }
multi-has-tab-own-now = { $value } tiene ahora una pestaña propia.

## remote: messages with values (src/remote.rs)

remote-protocol-error = error de protocolo: { $error }

## solarnet: messages with values (src/solar3d/net.rs)

solarnet-bad-message = mensaje erróneo: { $error }
solarnet-solar-protocol-mismatch-server-client = discrepancia del protocolo solar: servidor { $proto }, cliente { $value }

## smeter: messages with values (src/widgets/smeter.rs)

smeter-forward-power-w-measured-radio =
    {""}

    Potencia directa — { $w } W, medida por el sensor propio de la radio.
smeter-po-radio-s-own-power =
    {""}

    PO — el medidor de potencia de salida propio de la radio, al { $clamp } % de su fondo de escala. La posición de una aguja tal como la informa la radio, no una potencia en vatios calibrada; lo que hay que vigilar es si baja mientras la Excitación se queda quieta.
smeter-swr-1-match-antenna-presenting =
    {""}

    ROE — { $swr }:1, la adaptación que presenta la antena, tal como la mide la radio. Por debajo de 2:1 no tiene nada de particular; pasado 3:1 la escala se vuelve roja.
smeter-received-signal-strength-s9-db = Intensidad de señal recibida: S9 + { $over } dB ({ $s_dbm } dBm en la banda pasante).
smeter-received-signal-strength-s-dbm = Intensidad de señal recibida: S{ $value } ({ $s_dbm } dBm en la banda pasante).
smeter-radio-reports-c =
    {""}

    La radio informa de { $t } °C.
smeter-ps-puresignal-correcting-db-compression =
    {""}

    PS — PureSignal está corrigiendo { $correction_db } dB de compresión{ $held } (la realimentación coincidió con la transmisión en { $score }).
smeter-ps-puresignal-switched-but-has =
    {""}

    PS −− — PureSignal está activado pero no ha encontrado la transmisión en el camino de realimentación (mejor coincidencia { $score }), así que el transmisor sale sin corregir. Comprueba que una muestra de la salida del amplificador llega realmente a una entrada de recepción que el conmutador T/R no quita al transmitir, y que su atenuador no se la traga. El lazo escucha solo este receptor — un acoplador cableado a un segundo receptor (RX2, ADC2) nunca se lee como realimentación.

## specview: messages with values (src/widgets/spectrum_view.rs)

specview-rit-hz = RIT { $hz } Hz
specview-xit-hz = XIT { $hz } Hz

## decodes: nested messages

decodes-you-transmit-s-everyone-else = Transmites { $off } s { $after } los demás — la razón habitual por la que las llamadas quedan sin respuesta. Sincroniza el reloj de tu ordenador.

## publicsdr: nested messages

publicsdr-noise-floor-score = puntuación del suelo de ruido { $snr }
publicsdr-up-khz-i-q = hasta { $khz } kHz de I/Q

## txaudio: more messages

txaudio-gr-gate-de-ess-comp = GR puerta { $gate_gr_db } de-ess { $deess_gr_db } comp { $comp_gr_db } lim { $limiter_gr_db }

## amp: more messages

amp-h-step = ~{ $l_step } µH · paso { $l_step2 }
amp-pf-step = ~{ $c_step } pF · paso { $c_step2 }

## sstv: more messages

sstv-bit-octets-chunks-first-pass = { $from } · { $filename } · { $width }×{ $height } { $bits_per_pixel } bits · { $media_type } / { $content_encoding } · { $encoded_size } octetos en { $chunk_count } fragmentos ({ $chunks_first_pass } en la primera pasada) · sesión { $session }
sstv-baud-cpfsk-hz-khz-occupied = { $value } — { $symbol_rate } baudios CPFSK, ±{ $deviation_hz } Hz, { $bandwidth_hz } kHz de ancho de banda ocupado

## Clock, and a few short labels with values.

time-utc-and-local = { $utc } UTC · { $local } local
sat-in-seconds = en { $s } s
winlink-to-list = para { $to }
winlink-cc-list = cc { $cc }
spots-n-of-total = { $n } de { $total }
wefax-n-of-total = { $n } de { $total }
adsb-range-bearing = { $km } km a { $bearing }°
ais-range-bearing = { $km } km a { $bearing }°

## More values from sdroxide-types, as pickers name them (src/i18n/labels.rs).
rtl-sdr-hf-mode-off = Desactivado (solo sintonizador, desde 24 MHz)
rtl-sdr-hf-mode-auto = Automático
rtl-sdr-hf-mode-direct-q = Muestreo directo (rama Q)
rtl-sdr-agc-manual = Manual (sin AGC)
rtl-sdr-agc-tuner = AGC del sintonizador
rtl-sdr-agc-rtl = AGC digital del RTL
rtl-sdr-agc-both = AGC del sintonizador + RTL
mode-control-cat = CAT
mode-control-radio = Controlado por radio
line-state-none = Ninguna
line-state-high = Alta
line-state-low = Baja
elad-tx-input-usb-audio = Audio USB
elad-tx-input-mic = Micrófono
elad-tx-input-auto = Auto
elad-tx-input-leave = Dejar como esté en la radio
tr-usdx-audio-one-cable = Un cable (audio en el flujo CAT)
tr-usdx-audio-sound-card = Tarjeta de sonido USB (jack de 3,5 mm)
spy-server-format-uint8 = 8 bits (menor ancho de banda)
spy-server-format-int16 = 16 bits
spy-server-format-float32 = Coma flotante de 32 bits
sound-format-iq = IQ (estéreo)
sound-format-demod-audio = Audio demodulado
sdr-play-duo-tuner-tuner1 = Sintonizador 1 (50 Ohm / Hi-Z)
sdr-play-duo-tuner-tuner2 = Sintonizador 2 (50 Ohm)
sdr-play-duo-role-diversity = Una segunda antena (diversidad / supresión de QRM)
sdr-play-duo-role-second-radio = Una segunda radio, en su propia frecuencia
sdr-play-agc-off = Desactivado (ganancia de FI manual)
sdr-play-agc-hz5 = 5 Hz
sdr-play-agc-hz50 = 50 Hz
sdr-play-agc-hz100 = 100 Hz
rotator-protocol-rotctld = Hamlib rotctld (TCP)
rotator-protocol-pst-rotator = PSTRotator (UDP)
pluto-ptt-off = Desactivado
pluto-ptt-gpo01 = GPO0 = RX, GPO1 = TX
pluto-ptt-gpo23 = GPO2 = RX, GPO3 = TX
pluto-duplex-fdd = FDD (ambos a la vez)
pluto-duplex-tdd = TDD (de uno en uno)
pluto-agc-manual = Manual
pluto-agc-slow-attack = Ataque lento
pluto-agc-fast-attack = Ataque rápido
pluto-agc-hybrid = Híbrido
parity-none = Ninguna
parity-even = Par
parity-odd = Impar
panadapter-tap-antenna = Antena (compartida con la radio)
panadapter-tap-if-output = La salida de FI de la radio
panadapter-audio-attached = El receptor conectado
panadapter-audio-transceiver = El transceptor
kenwood-send-ts2000 = Estilo TS-2000 (TX;)
kenwood-send-ts590 = Estilo TS-590 (TX1;)
digi-mode-usb = USB
digi-mode-data = DIGI
digi-mode-radio = Controlado por radio
cw-keying-cat = Manipulador del equipo (CAT)
cw-keying-audio = Tarjeta de sonido (MCW)
audio-route-both = Ambos canales
audio-route-left = Solo izquierdo
audio-route-right = Solo derecho
amp-model-hr500 = Hardrock-500
amp-model-hr50 = Hardrock-50 (sin probar)
alert-sound-ding = Ding
alert-sound-two-tone = Dos tonos
alert-sound-triplet = Tresillo
alert-sound-warble = Gorjeo
alert-sound-digital = Digital
alert-reply-tone = Tono
alert-reply-voice = Voz
alert-reply-both = Tono + voz
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
icom-model-other = Otro (fija la dirección a mano)
backend-auto = Autodetección (SoapySDR / CAT)
backend-soapy = SoapySDR
backend-cat = CAT / Audio
backend-hpsdr = HPSDR (red)
backend-tci = TCI (red)
backend-icom-net = Icom LAN (red)
backend-smart-sdr = SmartSDR / FlexRadio (red)
backend-pluto = PlutoSDR (red)
backend-rtl-sdr = RTL-SDR (USB)
backend-rtl-tcp = RTL-SDR mediante rtl_tcp (red)
backend-spy-server = SpyServer (red)
backend-spy-server-vfo = SpyServer VFO+FFT, bajo ancho de banda (red)
backend-kiwi-sdr = KiwiSDR / Web-888 (red)
backend-rx888 = RX-888 (USB)
backend-airspy-hf = Airspy HF+ (USB)
backend-airspy = Airspy R2 / Mini (USB)
backend-hack-rf = HackRF One / Pro (USB)
backend-sdr-play = SDRplay RSP (USB)
backend-elad = ELAD FDM-DUO / FDM-S (USB)
backend-lime = LimeSDR + LimeRFE (LimeSuite)
backend-hydra-sdr = HydraSDR RFOne (USB)
backend-fobos = RigExpert Fobos SDR (USB)
backend-usb-audio = Radio con audio USB (tarjeta de sonido)
backend-none = Sin configurar
cat-family-xiegu = Xiegu
cat-family-icom = Icom
cat-family-yaesu = Yaesu
cat-family-kenwood = Kenwood
cat-family-elecraft = Elecraft
cat-family-elad = ELAD
cat-family-qrp-labs = QRP Labs
cat-family-rs-hfiq = RS-HFIQ
cat-family-tr-usdx = (tr)uSDX
cat-family-rigctld = Hamlib rigctld (red)
cat-family-flrig = flrig (red)
publicsdr-its-interface = su interfaz

## Values from sdroxide-types drawn on chips, menus and lists (src/i18n/labels.rs).
agc-mode-off = Off
agc-mode-slow = Lento
agc-mode-med = Med
agc-mode-fast = Rápido
airspy-gain-linearity = Linealidad (mejor manejo de señales fuertes)
airspy-gain-sensitivity = Sensibilidad (mejor con señales débiles)
ais-kind-class-a = Clase A
ais-kind-class-b = Clase B
ais-kind-base-station = estación base
ais-kind-sar-aircraft = aeronave SAR
ais-kind-aid-to-navigation = ayuda a la navegación
ais-kind-sart = SART / MOB / EPIRB
ais-kind-craft = embarcación asociada
alert-event-called = Una estación me está llamando
alert-event-cq = Un CQ al que podría responder
alert-event-new-dxcc = Nueva entidad DXCC
alert-event-new-dxcc-band = Nueva entidad en esta banda
alert-event-new-grid = Nuevo cuadrado de locátor
audio-routing-both = Izquierdo + derecho
audio-routing-left = Solo izquierdo
audio-routing-right = Solo derecho
button-mode-momentary = Mantener
button-mode-toggle = Alternar
contest-mode-none = Ninguno
contest-mode-eu-vhf = EU VHF Contest
converter-tx-off = Desactivado al convertir
converter-tx-transverter = Por el mismo conversor
converter-tx-own = Desplazamiento propio
cw-engine-neural = NEURONAL
cw-engine-timing = TEMPORIZ.
decode-sort-none = Ninguno
decode-sort-signal = SNR
decode-sort-distance = Dist
decode-sort-country = País
diversity-mode-cancel = Cancelar — anular una fuente de ruido
diversity-mode-combine = Combinar — recepción en diversidad
dxped-mode-normal = Normal
dxped-mode-hound = Hound
dxped-mode-fox = Fox
fail-safe-refuse-tx = Negarse a transmitir
fail-safe-warn-only = Transmitir de todos modos, avisar
ftp-security-plain = FTP (sin cifrado)
ftp-security-explicit-tls = FTPS explícito (AUTH TLS)
ftp-security-implicit-tls = FTPS implícito
hpsdr-filter-board-none = Ninguna — las salidas se quedan apagadas
hpsdr-filter-board-n2adr = Placa de filtros N2ADR
hpsdr-filter-board-alex = Código de banda Alex / Hermes (Zeus, HiQSDR, Quisk)
hpsdr-filter-board-custom = Personalizada — mi propia tabla de abajo
hpsdr-io-rx-input-radio = Entrada propia de la radio
hpsdr-io-rx-input-io-board = Placa IO J9
hpsdr-io-rx-input-io-board-pure-signal = Placa IO J9, PureSignal en transmisión
hydra-sdr-gain-linearity = Linealidad (mejor manejo de señales fuertes)
hydra-sdr-gain-sensitivity = Sensibilidad (mejor con señales débiles)
icom-rx-source-af = AF — la radio demodula
icom-rx-source-if12k = FI de 12 kHz — sdroxide demodula
icom-scope-span-radio = Como esté en la radio
icom-scope-span-khz2-5 = 5 kHz (±2,5k)
icom-scope-span-khz5 = 10 kHz (±5k)
icom-scope-span-khz10 = 20 kHz (±10k)
icom-scope-span-khz25 = 50 kHz (±25k)
icom-scope-span-khz50 = 100 kHz (±50k)
icom-scope-span-khz100 = 200 kHz (±100k)
icom-scope-span-khz250 = 500 kHz (±250k)
icom-scope-span-khz500 = 1 MHz (±500k)
ism-family-weather = METEO
ism-family-meter = CONTADORES
ism-family-home-auto = DOMÓTICA
ism-family-lo-ra = LORA
ism-family-unidentified = DESCONOCIDO
ism-protocol-la-crosse-it = LaCrosse IT+
ism-protocol-fine-offset = Fine Offset
ism-protocol-bresser = Bresser
ism-protocol-z-wave = Z-Wave
ism-protocol-homematic = Homematic
ism-protocol-rtl433 = rtl_433
ism-protocol-unidentified = sin identificar
js8-speed-normal = NORMAL
js8-speed-fast = RÁPIDA
js8-speed-turbo = TURBO
js8-speed-slow = LENTA
lime-aux-role-off = Sin usar
lime-aux-role-diversity = Una segunda antena (diversidad / supresión de QRM)
lime-aux-role-pure-signal = Realimentación de transmisión (predistorsión PureSignal)
lookup-provider-none = Desactivada
lookup-provider-qrz = QRZ.com
lookup-provider-ham-qth = HamQTH
mail-folder-inbox = ENTRADA
mail-folder-outbox = SALIDA
mail-folder-sent = ENVIADOS
mail-folder-archive = ARCHIVO
memory-sort-name = Nombre
memory-sort-freq = Frec.
memory-sort-band = Banda
memory-sort-stored = Guardado
mouse-button-middle = Central
mouse-button-extra1 = Atrás (extra 1)
mouse-button-extra2 = Adelante (extra 2)
nr-level-off = Off
nr-level-low = Bajo
nr-level-medium = Medio
nr-level-high = Alto
nr-level-rnn-low = RNN Bajo
nr-level-rnn-med = RNN Medio
nr-level-rnn-high = RNN Alto
nr-level-spec-low = SPEC Bajo
nr-level-spec-med = SPEC Medio
nr-level-spec-high = SPEC Alto
nr-level-nr2-low = NR2 Bajo
nr-level-nr2-med = NR2 Medio
nr-level-nr2-high = NR2 Alto
nr-level-df-low = DFNR Bajo
nr-level-df-med = DFNR Medio
nr-level-df-high = DFNR Alto
nr-strength-low = Baja
nr-strength-med = Media
nr-strength-high = Alta
orbit-rings-none = ninguno
orbit-rings-curated = curados
orbit-rings-all = todos
qso-step-idle = Inactivo
qso-step-wait-cq = Esperar CQ
qso-step-calling-cq = Llamando CQ
qso-step-tx-grid = Tx locátor
qso-step-tx-report = Tx informe
qso-step-tx-r-report = Tx R+informe
qso-step-tx-rr73 = Tx RR73
qso-step-tx73 = Tx 73
qso-step-confirming = Confirmando
rds-standard-auto = Auto
rds-standard-rds = RDS
rds-standard-rbds = RBDS
region-r1 = Región 1 — Europa, África, Oriente Medio, Asia septentrional
region-r2 = Región 2 — América
region-r3 = Región 3 — Asia meridional y oriental, Australasia, Pacífico
relative-mode-absolute = Absoluto (fader)
relative-mode-sign-magnitude = Relativo: signo/magnitud
relative-mode-twos-complement = Relativo: complemento a dos
relative-mode-binary-offset = Relativo: centrado en 64
relay-link-off = Ninguno
relay-link-serial = Placa de relés USB (serie)
relay-link-serial-lines = Línea serie RTS/DTR
relay-link-hid = Placa de relés USB (HID)
relay-link-cm108 = GPIO de tarjeta de sonido CM108/CM119
relay-link-gpio = Línea GPIO de Linux
relay-link-command = Comando externo
relay-role-unused = Sin usar
relay-role-sdr-antenna = Poner a masa la antena del SDR
relay-role-amplifier = Activar un amplificador / relé T-R
relay-role-aux = Auxiliar
relay-role-band-decoder = Decodificador de banda (filtro / transversor)
rfe-channel-wb1000 = Banda ancha 1 – 1000 MHz
rfe-channel-wb4000 = Banda ancha 1 – 4 GHz
rfe-channel-ham0030 = HF, hasta 30 MHz
rfe-channel-ham0070 = 6 m / 4 m (50 – 70 MHz)
rfe-channel-ham0145 = 2 m (140 – 150 MHz)
rfe-channel-ham0220 = 1,25 m (220 – 225 MHz)
rfe-channel-ham0435 = 70 cm (400 – 450 MHz)
rfe-channel-ham0920 = 33 cm (902 – 928 MHz)
rfe-channel-ham1280 = 23 cm (1220 – 1420 MHz)
rfe-channel-ham2400 = 13 cm (2,3 – 2,5 GHz)
rfe-channel-ham3500 = 9 cm (3,3 – 3,7 GHz)
rfe-channel-cell-band01 = Banda celular 1
rfe-channel-cell-band02 = Banda celular 2
rfe-channel-cell-band03 = Banda celular 3
rfe-channel-cell-band07 = Banda celular 7
rfe-channel-cell-band38 = Banda celular 38
rfe-channel-not-selected = Ninguno seleccionado
rfe-link-off = Sin conectar
rfe-link-serial = Su propio cable USB (serie)
rfe-link-board = A través del LimeSDR (GPIO / I²C)
rfe-mode-control-auto = Automático (sigue el cableado)
rfe-mode-control-rx = Siempre recepción
rfe-mode-control-tx = Siempre transmisión
rfe-mode-control-tx-rx = Siempre ambos (solo conectores separados)
rifp-encoding-auto = Auto
rifp-encoding-png = PNG
rifp-encoding-jpeg = JPEG
rifp-encoding-raw = Crudo
rifp-encoding-rle8 = RLE8
rifp-encoding-zlib = Zlib
rifp-encoding-group4 = G4
rifp-encoding-group3 = G3
scan-kind-memories = MEM
scan-kind-range = RANGO
scan-resume-carrier = PORTADORA
scan-resume-timed = TIEMPO
scan-resume-manual = MANUAL
sense-line-off = Sin cablear
sense-line-cts = CTS
sense-line-dsr = DSR
sense-line-dcd = DCD (detección de portadora)
shift-simplex = SÍMPLEX
shift-minus = −
shift-plus = +
speed-slow = Lenta
speed-medium = Media
speed-fast = Rápida
speed-faster = Más rápida
speed-fastest = La más rápida
tls-verify-system = Almacén de confianza del sistema
tls-verify-pinned = Huella fijada
tls-verify-off = No verificar
tone-mode-off = OFF
tone-mode-ctcss = CTCSS
tone-mode-dcs = DCS
tx-voice-stage-gate = Puerta
tx-voice-stage-eq = Ecualizador
tx-voice-stage-de-ess = De-esser
tx-voice-stage-comp = Compresor
upload-naming-sequence = image1 … imageN (rotar)
upload-naming-original = Nombre de archivo original
wheel-action-zoom = Zoom
wheel-action-tune = Sintonía
wheel-action-none = Nada

# Undocked windows (from sdroxide-brown)
frame-detached-title = { $flavor } — { $name }
frame-dock-band-menu-hint = DOCK — devolver el selector de banda y modo a la ventana principal, igual que cerrar esta sin buscar su casilla de cierre.
frame-dock-panel-hint = DOCK — volver a poner este panel bajo la cascada
frame-dock-spectrum-hint = DOCK — devolver este espectro a la ventana principal, igual que cerrar esta sin buscar su casilla de cierre.
frame-every-module-own-window = cada módulo está en su propia ventana
frame-module-aux-panadapter = panadapter auxiliar
frame-module-band-mode = banda y modo
frame-module-controls = controles
frame-module-panadapter = panadapter
frame-module-panel = panel
multi-dock-all-windows = ACOPLAR TODAS LAS VENTANAS
multi-module-panel = Panel de operación
multi-undock-all-windows = DESACOPLAR TODAS LAS VENTANAS
settings-ui-dock-all-hint = Devuelve cada ventana desacoplada a esta.
settings-ui-docked = Acoplada en esta ventana
settings-ui-module-aux = AUX SP (segundo espectro)
settings-ui-module-band-menu = Selector de banda y modo
settings-ui-module-controls = Controles
settings-ui-module-panadapter = Panadapter
settings-ui-undock-all-hint = Pone cada módulo en su propia ventana; la ventana principal queda con el espectro y los decodificadores. Un módulo sin nada que mostrar, como el panel de operación en un modo de voz, no abre ventana.
settings-ui-undock-hint =
    Desacoplado, este módulo pasa a su propia ventana y la ventana principal devuelve el espacio a lo que queda: ponga la cascada en un segundo monitor, o los controles en un tercero. Cerrar su ventana lo vuelve a acoplar.
    
    En Wayland se recupera el tamaño, pero el compositor decide dónde va una ventana: hágala flotante y fíjela a un monitor con una regla de ventana que coincida con el app-id «{ $app_id }».
settings-ui-undocked = Desacoplada — su propia ventana
settings-ui-window-label = Ventana { $module }
frame-window-chip-hint = WINDOW — mover esta ventana a una ventana propia del sistema, para ponerla en otro monitor
frame-dock-panel-pane-hint = DOCK — devolver este panel al panel bajo la cascada
frame-panel-window-title = { $mode } — panel
frame-roll-up-hint = Enrollar — dejar solo la barra de título y devolver el espacio a la cascada
frame-unroll-hint = Desenrollar — mostrar de nuevo el panel
frame-window-panel-hint = WINDOW — dar a este panel una ventana propia, para ponerla en otro monitor
topbar-dock-all = ACOPLAR TODO
topbar-dock-all-hint = Traer todas las ventanas de módulos de vuelta a esta ventana
topbar-module-windows-title = Ventanas de módulos
topbar-undock-all = DESACOPLAR TODO
topbar-undock-all-hint = Poner cada módulo en una ventana propia — esta ventana queda como el espectro y los decodificadores
topbar-windows-chip-hint =
    ▣  Izquierdo: traer a casa todas las ventanas de módulos
    Derecho: la lista de ventanas de módulos
topbar-wname-aux = SP2 (AUX) — un segundo espectro
topbar-wname-band-menu = Banda y modo
topbar-wname-controls = Control RX
topbar-wname-panadapter = SP1 — el espectro
topbar-wname-panel = Panel de operación
topbar-wstate-float = Sacar { $name } del panel y hacerlo flotar sobre esta ventana
topbar-wstate-floating-panel-only = Flotante es un estado propio del panel de operación — un espectro o una consola tiene un solo lugar en esta ventana
topbar-wstate-here-now = { $name } está aquí ahora
topbar-wstate-no-panel = No hay panel de operación en { $mode } — esto se aplica en un modo que lo tenga
topbar-wstate-own-window = Dar a { $name } una ventana propia, en cualquier monitor
topbar-wstate-put-back = Volver a poner { $name } en esta ventana
frame-workspace-missing = No hay ningún espacio de trabajo { $n } guardado — guarda uno en Ajustes › Interfaz › Espacios de trabajo y esta tecla lo recuperará.
settings-ui-workspaces = Espacios de trabajo
settings-ui-workspaces-note = Una disposición de ventanas con nombre: qué módulos están en su propia ventana y dónde están esas ventanas. Guarde la actual o restaure una guardada. Hasta diez.
settings-ui-workspaces-none = Aún no hay espacios de trabajo guardados.
settings-ui-workspace-restore-home = Restaurar esta disposición: todos los módulos en la ventana principal
settings-ui-workspace-restore-out = Restaurar esta disposición: { $n } módulo(s) en su(s) propia(s) ventana(s)
settings-ui-workspace-forget = Olvidar este espacio de trabajo
settings-ui-workspace-name = poner nombre a esta disposición
settings-ui-workspace-full = Diez espacios de trabajo es el límite que mantiene SDRuno; borre uno primero
settings-ui-workspace-save = GUARDAR ACTUAL
settings-ui-workspace-save-hint = Guardar la disposición actual con este nombre
settings-ui-workspace-name-first = Escriba primero un nombre
topbar-band-window2-hint = WINDOW — abrir el selector en una ventana propia, tan ancha como quieras, en cualquier monitor
tnc-link = ENLACE
tnc-connected = CONECTADO
tnc-calling = LLAMANDO
tnc-idle = ESCUCHANDO
tnc-tx = TX
tnc-busy = OCUPADO
tnc-busy-hint = Hay otra estación en el canal. Una llamada ahora caería encima de ella.
tnc-setup = ⚙ CONFIG.
tnc-peer = Corresponsal
tnc-width = Ancho
tnc-speed = Velocidad
tnc-snr = S/R
tnc-bytes = Bytes
tnc-queued = En cola
tnc-host = Host
tnc-ports = Puertos
tnc-ports-off = apagados
tnc-nobody = nadie
tnc-holder-panel = este panel
tnc-holder-winlink = la ventana MAIL
tnc-holder-tcp = un programa en { $addr }
tnc-tune = SINTONÍA
tnc-abort = ABORTAR
tnc-abort-hint = Cortar el enlace ya, lo maneje quien lo maneje. El otro extremo agotará su tiempo en vez de ser avisado.
tnc-terminal = TERMINAL
tnc-callsign = indicativo
tnc-connect = CONECTAR
tnc-disconnect = DESCONECTAR
tnc-disconnect-hint = Colgar como es debido: primero sale lo que está en cola, luego el otro extremo acepta.
tnc-held-by = El módem está en uso por { $holder }.
tnc-no-callsign = Primero configure un indicativo (⚙ CONFIG.)
tnc-clear = BORRAR
tnc-clear-hint = Vaciar la transcripción. El enlace no se toca.
tnc-nothing-yet = Nada todavía. Escriba un indicativo arriba y pulse CONECTAR, o active Escuchar en ⚙ CONFIG. para aceptar llamadas.
tnc-type-here = escriba aquí, Intro envía
tnc-not-connected = no conectado
tnc-send = ENVIAR
setup-tnc-bandwidth = Ancho de banda
setup-tnc-listen = Atender llamadas
setup-tnc-listen-check = Escuchar desde el panel
setup-tnc-listen-hint = Aceptar llamadas entrantes en el terminal del panel. Sin efecto mientras los puertos TCP del host estén activos, porque un programa conectado ahí quiere las llamadas.
setup-tnc-host = Host TCP
