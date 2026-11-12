# sdroxide — the interface's English text, the reference every translation
# is checked against. See crates/sdroxide-ui/src/i18n.rs for the rules.
#
# Ids are grouped by where they are shown, and named after it:
# settings-ui-* is the UI tab of the Settings window, and so on. A message
# shown in capitals is written in capitals here; the code does not upper-case
# translated text.

## The language's own name, as the language picker lists it.
language-name = English

## Values from sdroxide-types, as the screen names them (src/i18n/labels.rs).

layout-mode-auto = Auto
layout-mode-desktop = Desktop
layout-mode-tablet = Tablet
layout-mode-small = Small screen
layout-mode-phone = Phone

ui-theme-default = Default
ui-theme-light = Light
ui-theme-high-contrast = High contrast
ui-theme-green-phosphor = Green phosphor
ui-theme-amber-phosphor = Amber phosphor
ui-theme-teal-orange = Teal / orange
ui-theme-rainbow = Rainbow
ui-theme-nord = Nord
ui-theme-nord-dark = Nord dark
ui-theme-gruvbox = Gruvbox
ui-theme-everforest = Everforest
ui-theme-solarized-dark = Solarized dark
ui-theme-solarized = Solarized
ui-theme-dracula = Dracula
ui-theme-catppuccin-mocha = Catppuccin mocha
ui-theme-catppuccin-latte = Catppuccin latte
ui-theme-modern-minimalist = Modern minimalist

chrome-style-angled = Angled
chrome-style-rectangular = Rectangular
chrome-style-rounded = Rounded
chrome-style-gradient = Gradient
chrome-style-bevel = 3D bevel
chrome-style-terminal = Terminal

font-size-small = Small
font-size-medium = Medium
font-size-large = Large

bandplan-kind-ham = Ham
bandplan-kind-cw = CW
bandplan-kind-digi = Digital
bandplan-kind-phone = Voice
bandplan-kind-beacon = Beacon
bandplan-kind-broadcast = Broadcast
bandplan-kind-am = AM / LW / MW
bandplan-kind-cb = CB

speech-verbosity-terse = Terse
speech-verbosity-normal = Normal
speech-verbosity-full = Full

# The examples in brackets are what the (English) voice says.
speech-freq-style-dial = Dial (14 point 074)
speech-freq-style-digits = Every digit
speech-freq-style-full = Dial with units

speech-callsign-style-phonetic = Phonetic (kilo one alpha)
speech-callsign-style-letters = Letters (kay one ay)
speech-callsign-style-as-is = As written

## Settings window, UI tab (src/app/settings/ui_tab.rs).

settings-ui-display = Display
settings-ui-moved-note = Panadapter detail, the spectrum's reaction and the waterfall's scroll speed are set from the SPEC button on the display strip — the DISP menu on a narrow window.
settings-ui-language = Language
    .hint = The language of the interface. The manual behind F1 and the voice announcements are in English whatever is picked here.
settings-ui-language-system = System ({ $language })
settings-ui-layout = Layout
settings-ui-theme = Theme
settings-ui-button-style = Button style
settings-ui-window-style = Window style
settings-ui-tooltip-delay = Tooltip delay
settings-ui-tooltip-delay-hint = Pointer rest before a tooltip shows
settings-ui-tooltip-grace = Tooltip follow-on
settings-ui-tooltip-grace-hint = After a tooltip: how long the next shows without delay; 0 = every one waits
settings-ui-frame-rate = Screen update rate
settings-ui-palette = Waterfall palette
settings-ui-tuning-buttons = Tuning buttons
settings-ui-tuning-buttons-show = Show on phone and tablet
    .hint = A row of step-down / step / step-up buttons under the control strip on a touched screen. There is no wheel to scroll a digit with and no dial, so without them the only way to move a known step is to type the whole frequency in. Never drawn on a desktop.
settings-ui-tuning-step = Step: { $step }
settings-ui-tuning-snap = First press snaps to the step
    .hint = A dial left between two multiples of the step goes to the next one in the direction pressed — 7 074 300 goes up to 7 075 000 at a 1 kHz step, down to 7 074 000. After that the buttons move by the step as usual. Off, every press moves by exactly the step.
settings-ui-smoothing = Waterfall smoothing
settings-ui-smoothing-interpolate = Interpolate
    .hint = Blend each screen pixel with the bins and rows around it, so a signal looks continuous where the display is wider than the transform. Turn it off for a rectangular waterfall — one block per bin, one per row — which is what reading a signal's shape off the picture needs: an interpolated signal cannot be told apart from a genuinely wider one. A bigger FFT (the FFT chip) is the other half of that.
settings-ui-clock = Clock
settings-ui-clock-show = UTC + local on the panadapter
    .hint = A wall clock at the top-right of the panadapter, in UTC and your own time.
settings-ui-background = Spectrum background
settings-ui-background-gradient = Gradient
settings-ui-background-top = top
settings-ui-background-bottom = bottom
settings-ui-spot-colours = Spot label colours
    .hint = The tint each spot source wears: the boxes along the bottom of the waterfall, the badges in the SPOTS list and the dots on the world map.
settings-ui-spot-colour = Colour for { $kind } spots
settings-ui-spot-colours-reset = Put every spot colour back to its default
settings-ui-reset = Reset
settings-ui-bandplan-colours = Band plan colours
    .hint = The shade each class of allocation is painted in on the band-plan strip along the bottom of the waterfall. The blocks are drawn semi-transparent over the waterfall, so they land darker there than in the swatch here.
settings-ui-bandplan-colour = Colour for { $kind } allocations
settings-ui-bandplan-colours-reset = Put every band-plan colour back to its default
settings-ui-skimmer-font = Skimmer font size
settings-ui-waterfall-font = Waterfall / spectrum font size
settings-ui-interface-font = Interface font size
    .hint = Scales the whole interface — menus, dialogs, windows, the radio tabs, the top bar and its buttons. The two sizes above are relative to it.
settings-ui-cities = Cities on maps
    .hint =
        Draw the world's cities — a dot per place, with its name beside it where there is room — on the flat maps: FT8/WSPR, APRS, ADS-B and AIS.

        They are most of what says *where* a dot is on a map of the whole world, and they are also the busiest thing on it. Turn them off if the names are getting in the way of the stations you are reading. The 3D globe is unaffected — its cities are night-side lights rather than markers, and nothing there is written across a contact.
settings-ui-cities-show = show cities and their names
settings-ui-update-check = Check for updates at startup
    .hint = Asks sdroxide.com once per start whether a newer release has been published, and says so in a dismissable banner above the waterfall. Nothing is sent beyond the request itself.
settings-ui-band-conditions-download = Download the band-conditions forecast
    .hint = Fetches HAMQSL.com's HF band-conditions forecast, at most once an hour, to colour the band menu. Off, the band menu shows no forecast unless the 3D view is open. Nothing is sent beyond the request itself.
settings-ui-broadcast-schedule-download = Download the broadcast schedule
    .hint = Fetches the EiBi shortwave broadcast schedule (about 1 MB) from eibispace.de when a new broadcasting season starts. Off, the schedule built into sdroxide (or the last one downloaded) stays in use, and Settings → Spots → Download schedule now still fetches it on request.
settings-ui-aoki-import = IMPORT AOKI LIST…
    .hint = Nobuo Aoki's shortwave list (NDXC) as extra label candidates for LABELS BY S/N. Not downloaded by sdroxide: fetch the season's ZIP from www1.s2.starcat.ne.jp/ndxc, unpack it and pick the text file here. Stored in the configuration folder; no network request.
settings-ui-aoki-imported = { $n } lines imported
settings-ui-aoki-refused = Not imported: { $why }
settings-ui-aoki-present = Aoki list imported
settings-ui-aoki-absent = No Aoki list (optional)
settings-ui-3d-view = 3D view
settings-ui-clouds = Cloud rendering
settings-ui-clouds-layered = Layered
settings-ui-clouds-volumetric = Volumetric
settings-ui-clouds-note = How the CLOUDS layer in the 3D view draws the weather. Layered stacks slices through the troposphere and is the cheap option. Volumetric walks a ray through it instead, so the Sun casts the cloud tops onto the deck below and lightning glows out through the storm making it rather than only brightening its outside — at several times the cost per pixel.

## Settings window, UI tab: voice announcements.

speech-title = Voice announcements
speech-enabled = Speak changes to the radio
    .hint = Reads out what changed, so the radio can be operated without seeing it
speech-voice = Voice
speech-voice-shipped = Shipped voice
speech-speed = Speed
    .hint = The voice stretches or compresses its own phrasing, so the pitch does not change. Past about 2× it stops getting shorter.
speech-volume = Volume
speech-output = Output
speech-detail = Detail
speech-duck = Duck receiver
speech-duck-while = While speaking
speech-test = Test
speech-what = What to announce
speech-cat-frequency = Frequency
speech-cat-mode-band = Mode and band
speech-cat-vfo-split = VFO and split
speech-cat-agc-gain = AGC and gain
speech-cat-levels = Drive, tune and mic
speech-cat-ptt = Transmit and receive
speech-cat-rit-xit = RIT and XIT
speech-cat-memory-scan = Memories and scanning
speech-cat-band-edge = Leaving an amateur band
speech-cat-notices = Warnings and messages
speech-cat-filters = Filters, squelch and noise reduction
    .hint = Off by default: these move constantly while chasing a signal
speech-decoded = Decoded messages
speech-ft8-to-me = FT8 calls to me
speech-ft8-cq = FT8 CQs I could answer
    .hint = A busy evening on twenty metres is a hundred of these a minute
speech-js8 = JS8 messages to me
speech-js8-allcall = JS8 @ALLCALL too
speech-fsq = FSQ messages to me
speech-include-snr = Include the report
    .hint = Only where the message carries none of its own — a decode that already reports a number does not also get ours
speech-ft8-qso = My own FT8 exchange
    .hint = What the sequencer is about to send, and the contact ending
speech-reading-text = Reading decoded text aloud
speech-text-cw = CW
speech-text-rtty-psk = RTTY, PSK, Olivia, THOR, FSQ
speech-text-cw-locked = CW only while the decoder is locked
    .hint = Reading an unlocked decoder's output is worse than silence
speech-text-note = Both are off by default. A decoder produces text faster than speech reads it, so anything that falls too far behind the live audio is dropped rather than queued.
speech-tuning-up = Tuning up
speech-swr-while-tuning = Read the SWR out while TUNE is held
speech-swr-every = Every
speech-swr-summary = Report the best match on release
speech-swr-alarm = Warn about high SWR during any transmission
speech-how-read = How things are read
speech-frequencies = Frequencies
speech-callsigns = Callsigns
speech-quiet-on-tx = Stay quiet while transmitting
    .hint = Speech goes to your speakers, and therefore into your microphone. High-SWR warnings still get through.
speech-device-note = Announcements play on their own sound device, so they are never recorded and never sent to anyone listening remotely. Keys for speaking the status, repeating the last announcement and stopping mid-sentence are on the Controls tab.

## Settings window (src/app/settings/mod.rs)

settings-waiting-station-s-network-configuration = Waiting for the station's network configuration…
settings-device-reports = as the device reports
settings-transverters = Transverters
settings-one-row-per-box-while = One row per box. While the dial is inside a row's band the radio is tuned to dial + offset and that row's transmit rule and drive limit apply; outside every row the radio is on its own bands. Rows are tried top to bottom.
settings-name = Name
settings-band-low = Band low
settings-band-high = Band high
settings-offset = Offset
settings-transmit = Transmit
settings-max-drive = Max drive
settings-off-takes-transverter-out-line = Off takes this transverter out of the line without losing the row — the band goes back to whatever the radio reaches on its own.
settings-t-2-m = 2 m
settings-what-you-call-shown-log = What you call it. Shown in the log when the dial selects it.
settings-bottom-band-transverter-works-dial = The bottom of the band this transverter works, on the dial. 144 for 2 m.
settings-top-band-dial-148-2 = The top of the band, on the dial. 148 for 2 m in Regions 2 and 3, 146 in Region 1.
settings-radio-ends-up-dial-offset = The radio ends up on dial + offset, so a transverter that brings a band down to an I.F. is negative: 2 m into a 28 MHz I.F. is -116, because 144 - 116 = 28. Drag to trim it a hertz at a time for an oscillator that is slightly off.
settings-off-while-converting-receive-only =
    Off while converting: receive only — the row is a converter, not a transverter, and nothing is keyed on this band.

    Through the same converter: it works both ways, and transmit takes the same offset.
settings-ceiling-transmit-drive-while-transverter =
    A ceiling on transmit drive while this transverter is selected, as a percentage of full. A transverter's I.F. input takes milliwatts and the drive that is right for the radio's own bands will destroy it, so this is the row's most important field. 100% is no limit.

    Your Drive setting is held under this rather than moved, so the number you use on HF comes back when the dial leaves the band.
settings-remove = REMOVE
settings-add-transverter = ADD TRANSVERTER
settings-takes-effect-apply-reconnect-like = Takes effect on Apply / reconnect, like the converter offset above.
settings-maximum-transmit-drive = Maximum transmit drive
settings-hard-limit-drive-tune-controls = A hard limit the Drive and TUNE controls cannot be taken past, applied after the band calibration below so nothing can lift the drive back over it. Set it where the radio makes its rated power and the whole of the Drive control becomes usable — on a transmitter that modulates its own samples (HPSDR, LimeSDR, PlutoSDR, HackRF) full drive is the transmitter wide open, which on a high-gain amplifier is well past what its finals are rated for. Off means the controls reach full drive.
settings-limit-drive = Limit drive to
settings-per-cent-full-drive-key = Per cent of full drive. Key the radio into a dummy load and bring this down until the meter reads the power the amplifier is rated for.
settings-applies-immediately-every-band-tune = Applies immediately, on every band and to TUNE as well.
settings-transmit-drive-band = Transmit drive by band
settings-every-amplifier-has-different-gain = Every amplifier has a different gain on every band, so one Drive setting makes a different power on each. Measure the output on each band, then trim the bands that come out high until they all match: set Drive for the band that needs the most (usually the highest), and take the others down to it. Decibels of output power, applied to voice, digital and TUNE alike. Zero everywhere — the default — changes nothing.
settings-other = Other
settings-clear-all = CLEAR ALL
settings-back-no-calibration-drive-setting = Back to no calibration: the Drive setting reaches every band whole.
settings-applies-immediately-band-you-would = Applies immediately, and to the band you would transmit on — behind a transverter that is the band on the dial, and the transverter's own drive limit still wins over anything set here.
settings-settings = Settings
settings-enter-server-s-address-first = enter the server's address first
settings-enter-receiver-s-address-first = enter the receiver's address first, or pick one from Public SDRs
settings-no-radio-selected-press-discover = no radio selected — press Discover, or enter an address
settings-enter-radio-s-address-first = enter the radio's address first
settings-general = General
settings-radio = Radio
settings-ui = UI
settings-alerts = Alerts
settings-controls = Controls
settings-spots = Spots
settings-freedv = FreeDV
settings-uploads = Uploads
settings-winlink = Winlink
settings-servers = Servers
settings-t-r-switch = T/R switch
settings-amplifier = Amplifier
settings-remote = Remote
settings-tle = TLE
settings-profiles = Profiles
settings-station = Station
settings-enter-digital-mode-ft8-sstv = Enter a digital mode (FT8 / SSTV / …) once to load the saved values.
settings-callsign = Callsign
settings-grid-square = Grid square
settings-your-callsign-grid-shared-across = Your callsign and grid, shared across FT8/FT4/FT2, SSTV image headers, and the logbook. Also editable from the FT8 / SSTV setup dialog.
settings-iaru-region = IARU region
settings-which-radio-interface-server-uses = Which radio interface the server uses is set on the machine it runs on.
settings-radio-interface = Radio interface
settings-waiting-machine-radio-attached-interface = Waiting for the machine the radio is attached to. Its interface settings are below and can be changed from here meanwhile.
settings-converter = Converter
settings-manual-type-offset-below = Manual — type an offset below
settings-what-box-reads-when-offset = What this box reads when the offset is not one of the presets. Set it by typing in the Offset field below; there is nothing to select here.
settings-frequency-converter-front-receiver-pick = A frequency converter in front of the receiver. Pick one, or type an offset beside it.
settings-how-far-converter-moves-signal =
    How far a converter moves the signal on its way to the receiver, in Hz — the same number and sign every converter's documentation and every other SDR program states. The radio ends up on dial + offset: positive for an upconverter (a Ham It Up is 125000000), negative for anything that brings a band down to an I.F. — a satellite LNB, or a 2 m transverter on a 28 MHz I.F., which is -116000000 (28 − 144). 0 = no converter.

    Drag to trim it a hertz at a time, which is what a converter whose oscillator is slightly off wants.

    This is the receive path. What is in the transmit line is the row below.

    Takes effect on Apply.
settings-what-transmit-line-while-converter =
    What is in the transmit line while a converter is set.

    Off while converting: nothing is transmitted — the safe default, and right for a receive-only accessory.

    Through the same converter: one box converts both ways, a transverter. Transmit takes the offset above and follows it when it is trimmed.

    Its own offset: the transmit line is different from the receive one. Nought is the common case — a converter (an LNB, a Ham It Up) on receive with the transmitter on its own antenna, which is the QO-100 station.

    The amateur-band check still applies, on the frequency you are transmitting on.

    Takes effect on Apply.
settings-how-far-transmit-line-moves =
    How far the transmit line moves the signal, in Hz, on the same sign rule as the receive offset: the radio transmits at dial + offset.

    0 = the transmitter is wired to its own antenna and works on the frequency the dial says — a QO-100 station's 2.4 GHz uplink.

    A transmit converter that takes an I.F. *up* is a negative number: the radio works below the dial.
settings-rx-range = RX range
settings-which-frequencies-radio-receives-mhz =
    Which frequencies this radio receives, in MHz: 144-146, 430-440. Leave empty to use whatever the device reports about itself.

    Band buttons outside the range are greyed out and the dial will not go there.

    These are dial frequencies. With a converter set, state the band you tune — 144-148 for a 2 m transverter — not the I.F. the radio is really on.

    Takes effect on Apply.
settings-tx-range = TX range
settings-which-frequencies-radio-transmits-mhz =
    Which frequencies this radio transmits on, in MHz: 144-146, 430-440. Leave empty to use whatever the device reports — and if it reports nothing, the driver is taken at its word and any frequency is allowed.

    Dial frequencies, like the receive range above.

    This is a limit you set, not a licence: transmitting outside the amateur bands is refused regardless unless you have turned that off in config.toml. Nor does it give a receive-only device a transmitter.

    Takes effect on Apply.
settings-antenna = Antenna is
settings-where-radio-listens-station-antenna =
    Where this radio listens from.

    At the station: the antenna is yours, so your own locator says where it is. The default, and right for everything in the shack.

    Somewhere else: an online receiver, or your own set up on a hilltop. What it hears is reported to PSK Reporter, WSPRnet and FreeDV Reporter from the locator beside this, never from yours — and ADS-B places aircraft against it too.

    Picking a receiver under "Public SDRs" fills this in for you.

    Takes effect on Apply.
settings-receiver-s-maidenhead-locator-jn88ec =
    The receiver's Maidenhead locator — JN88ec, DO30db. Four or six characters.

    Leave it empty if you do not know where the receiver is: nothing is then reported at all, which is the only honest answer. Reporting from your own square would put somebody else's reception on the wrong continent.
settings-audio = Audio to
settings-gain-per-band = Gain per band
settings-remember-receive-gain-per-band = Remember receive gain per band
settings-each-band-keeps-receive-gain = Each band keeps the receive gain you last set on it, and gets it back when the dial returns — a band button, a drag across a band edge or the radio's own knob. General coverage is split into HF, VHF, UHF and above. A band you have not set a gain on keeps whatever is in force. Off: one gain everywhere.
settings-s-meter-calibration = S-meter calibration
settings-added-radio-s-s-meter =
    Added to this radio's S-meter reading. An SDR's meter starts out reading dBFS — level against the converter's full scale — so a receiver with plenty of gain can show S9 with no antenna connected. Set this to the difference between what the meter shows and a level you know: a signal generator, or a quiet band you trust to sit around S2–S3. Applies at once.

    On top of cal_offset_db in config.toml, which is one figure for the whole station.
settings-t-0-db = 0 dB
settings-antenna-somewhere-else-no-locator = The antenna is somewhere else and no locator is set, so nothing this radio hears is reported to PSK Reporter, WSPRnet or FreeDV Reporter. Type the receiver's locator above to report from it.
settings-ranges-are-mhz-low-high = Ranges are in MHz, low-high, separated by commas: 144-146, 430-440 — that is 144000000-146000000 Hz and 430000000-440000000 Hz. They are dial frequencies: a converter offset does not move them. The converter offset above is the field in hertz. Leave a range empty to use whatever the device reports about itself; a device that reports nothing is taken at its word.
settings-dwell-recentre = Centre the window on a resting dial
settings-dwell-recentre-hover = After five seconds of rest more than a twentieth of the window off its middle, the I/Q window is centred on the dial, once per resting place. Off: the window stays where it is until the dial leaves it.
settings-careful-rtl-sdr-blog-v4 = Careful on an RTL-SDR: the Blog V4 upconverts on its own below 28.8 MHz, so a negative offset that lands the hardware there shifts twice.
settings-transmit-audio = Transmit audio
settings-use = in use
settings-every-stage-off = every stage off
settings-open-tx-audio = Open TX Audio…
settings-microphone-processing-chain-gate-eq = The microphone processing chain — gate, EQ, de-esser, compressor and limiter — with a test recording to tune it by ear without transmitting. Voice modes only. Also opened by clicking Mic in the TX box.
settings-pick-radio-interface-above-configuration = Pick a radio interface above (this configuration used the removed auto-detect mode).
settings-configuration-used-removed-auto-detect = This configuration used the removed auto-detect mode. Pick an interface above once the machine the radio is attached to has answered.
settings-radio-has-no-interface-yet = This radio has no interface yet — pick one above and press Apply / reconnect.
settings-radio-has-no-interface-yet-2 = This radio has no interface yet. One can be picked above once the machine the radio is attached to has answered.
settings-apply-reconnect = Apply / reconnect
settings-switch-interface-now-no-restart = Switch to this interface now — no restart needed
settings-switch-server-s-radio-interface = Switch the server's radio to this interface now — no restart needed. The old one keeps running if the new one cannot be opened.
settings-switches-live-radio-without-restarting = Switches the live radio without restarting.
settings-everything-above-server-s-own = Everything above is the server's own configuration, and changes are saved there. Most settings apply as you change them; the ones fixed when the device is opened — the sample rate, an address, the interface itself — need this button.
settings-dx-cluster-telnet = DX cluster (telnet)
settings-enabled = Enabled
settings-host = Host
settings-port = Port
settings-login-call = Login call
settings-commands = Commands
settings-one-per-line-e-g = one per line, e.g. SET/FT8
settings-reverse-beacon-network = Reverse Beacon Network
settings-read-world-s-cw-rtty = Read the world's CW/RTTY skimmers and feed the propagation map with them. This is what makes the map show bands this radio is not listening to. On by default: it puts nothing on the air, needs no account, and uses the callsign from the General tab. RBN spots do not appear in the spot list — they are measurements, not invitations.
settings-t-7000-cw-rtty-feed-7001 = 7000 is the CW/RTTY feed, 7001 the FT8/FT4 one
settings-one-per-line-e-g-2 = one per line, e.g. set/filter cont=eu
settings-sent-after-login-place-narrow = Sent after login. The place to narrow the feed — without a filter this is every skimmer on Earth.
settings-rbn-paths-are-placed-country = RBN paths are placed from country centres, not locators — accurate for a small country, out by a long way for a large one. They are a separate layer on the propagation map and can be switched off there.
settings-pota-sota-psk-reporter = POTA / SOTA / PSK Reporter
settings-pota-activator-spots = POTA activator spots
settings-sota-spots = SOTA spots
settings-psk-reporter-current-band = PSK Reporter (current band)
settings-upload-my-ft8-ft4-ft2 = Upload my FT8/FT4/FT2/JS8 decodes
settings-report-what-station-hears-pskreporter = Report what this station hears to pskreporter.info, so it appears there as a receiver. Uses the callsign and grid from the General tab.
settings-antenna-2 = Antenna
settings-collector = Collector
settings-t-4739-live-collector-14739-test = 4739 is the live collector, 14739 the test one
settings-max-age-s = Max age (s)
settings-wsprnet = WSPRnet
settings-upload-my-wspr-decodes = Upload my WSPR decodes
settings-send-every-wspr-reception-wsprnet = Send every WSPR reception to wsprnet.org. On by default: it puts nothing on the air, and reporting what you hear is what makes a WSPR receiver part of the network rather than a private curiosity. A slot that decoded nothing is reported too, which is how the network tells a shut band from a receiver that was switched off.
settings-download-who-heard-me = Download who heard me
settings-ask-wsprnet-org-which-stations = Ask wsprnet.org which stations decoded this one. WSPR has no acknowledgement of any kind, so this is the only way a transmitting beacon learns anything about its own reach.
settings-ask-every = Ask every
settings-looking-back = looking back
settings-apply = {" "}APPLY{" "}
settings-persist-re-connect-feeds = Persist and (re)connect the feeds
settings-broadcast-stations = Broadcast stations
settings-winlink-account = Winlink account
settings-password = Password
settings-winlink-account-password-not-gateway = The Winlink account password, not the gateway password. It is case-sensitive — enter it exactly as it was issued.
settings-locator = Locator
settings-how-connect = How to connect
settings-route = Route
settings-internet = Internet
settings-forward-cms-over-internet = Forward with the CMS over the internet
settings-radio-packet = Radio (packet)
settings-call-rms-gateway-air-radio = Call an RMS gateway on the air. The radio must be in PACKET or PACKET-HF.
settings-gateway = Gateway
settings-via = Via
settings-digipeaters-order-separated-spaces-usually = Digipeaters, in order, separated by spaces. Usually empty — a gateway you can hear directly is a gateway you should call directly.
settings-speed = Speed
settings-most-rms-gateways-answer-1200 = Most RMS gateways answer at 1200. Calling a 1200 gateway at 9600 sounds exactly like a gateway that is off the air, so set this from what the gateway's owner publishes rather than guessing. 9600 also needs the radio's data port at both ends. 300 is HF.
settings-frequency = Frequency
settings-clear = CLEAR
settings-zero-leaves-dial-alone-which = Zero leaves the dial alone, which is what you want when you park on one channel. Anything else tunes the radio when the session starts.
settings-my-gateways = My gateways
settings-winlink-s-published-gateway-list = Winlink's published gateway list needs an API key sdroxide does not have, so keep your own. The two or three gateways reachable from one location are learned by trying, and they rarely change.
settings-use-2 = USE
settings-call-one-next-connect = Call this one on the next connect
settings-current-dial = current dial
settings-forget = FORGET
settings-forget-gateway = Forget this gateway
settings-add-gateway = + ADD GATEWAY
settings-save-gateway-above-list = Save the gateway above to the list
settings-internet-gateway = Internet gateway
settings-cms-address = CMS address
settings-client-name = Client name
settings-winlink-s-production-servers-only = Winlink's production servers only accept client names they know, and answer an unknown one with "Unknown client types are not allowed on production servers". Until sdroxide is registered with the Winlink Development Team, connecting needs a name they recognise.
settings-automatic-connection = Automatic connection
settings-connect-timer = Connect on a timer
settings-every = Every
settings-persist-winlink-account = Persist the Winlink account
settings-callsign-lookup = Callsign lookup
settings-provider = Provider
settings-auto-fill-name-qth-grid = Auto-fill name/QTH/grid on spot click & QSO
settings-choose-provider-enter-login = Choose a provider to enter its login.
settings-qrz-user = QRZ user
settings-qrz-pass = QRZ pass
settings-hamqth-user = HamQTH user
settings-hamqth-pass = HamQTH pass
settings-one-hamqth-account-does-both = One HamQTH account does both — this login is the one the HamQTH upload tab uses.
settings-upload = Upload
settings-auto-upload-each-new-qso = Auto-upload each new QSO
settings-master-switch-which-services-qso = The master switch. Which services a QSO goes to is set on each tab below.
settings-turn-auto-upload-each-new = Turn on "Auto-upload each new QSO" above first — a service ticked here is not pushed until it is.
settings-auto-upload-off-above-so = Auto-upload is off above, so the service ticks are disabled and nothing is pushed automatically yet — the per-QSO UP button in the logbook still works.
settings-qrz-log-key = QRZ log key
settings-qrz-logbook-api-key-your = The QRZ logbook API key from your QRZ logbook settings — not the XML-lookup login.
settings-eqsl-user = eQSL user
settings-eqsl-pass = eQSL pass
settings-eqsl-qth = eQSL QTH
settings-qth-nickname-eqsl-profile-upload = The QTH Nickname of the eQSL profile to upload to and download from — needed only when the account owns more than one; leave it blank otherwise.
settings-same-login-hamqth-callsign-lookup = The same login as the HamQTH callsign lookup — filling in either one fills in both.
settings-club-log-email = Club Log email
settings-club-log-pass = Club Log pass
settings-club-log-key = Club Log key
settings-wrl-api-key = WRL API key
settings-developer-api-key-world-radio = The developer API key from World Radio League → Integrations → Developer API. It is shown once when you generate it, so copy it then. Contacts go to your default logbook — set one in WRL if you keep more than one.
settings-confirmations-download = Confirmations (download)
settings-lotw-user = LoTW user
settings-lotw-pass = LoTW pass
settings-lotw-upload-uses-tqsl-export = LoTW upload uses TQSL — export ADIF from the logbook and sign it. LoTW/eQSL confirmations are downloaded here to mark worked-vs-confirmed.
settings-sync-confirmations = SYNC CONFIRMATIONS
settings-fetching-subscriptions = Fetching subscriptions…
settings-spectrum-waterfall-everything-read-off =
    Its spectrum, its waterfall and everything read off them are on that radio's tab, which is why it has none of its own. The interface settings above are still this radio's and still apply — press Apply / reconnect after changing them and both radios come back up together.

    To use it on its own again, clear the Panadapter receiver box on that radio's page.
settings-panadapter = Panadapter
settings-give-radio-wideband-panadapter-borrowing = Give this radio a wideband panadapter by borrowing another radio's receiver — an SDR on the same antenna, or one fed from this radio's I.F. output. The spectrum, the waterfall, the sub receiver, the digital modes and the skimmer all come from that receiver; the dial, the mode, the filter and the transmitter stay here.
settings-receiver = Receiver
settings-none = None
settings-which-radio-receives-one-leaves =
    Which radio receives for this one. It leaves the tab strip while it is lent — its front end belongs to this radio's engine — and comes straight back when this is set to None.

    Takes effect on Apply.
settings-connected = Connected to
settings-how-far-above-radio-s =
    How far above this radio's dial the receiver sits, in hertz — 0 for a receiver on the same antenna. For an I.F. tap, the intermediate frequency the receiver is parked on: 8215000 for a K3's KXV3B, 9000000 for a 9 MHz I.F.

    This is not the Converter field above, which retunes the radio, and not the CAT tab's I/Q centre offset, which is about this radio's own sound card. Nothing typed here is ever sent to either radio.

    Takes effect on Apply.
settings-audio-2 = Audio from
settings-follow-dial = Follow the dial
settings-keep-receiver-radio-s-dial = Keep the receiver on this radio's dial, and move the dial when you tune on the panadapter. Off parks the receiver where it is — a fixed watch on one segment while the radio goes elsewhere.
settings-invert-spectrum = Invert spectrum
settings-mirror-receiver-s-span-about = Mirror the receiver's span about its centre, for a tap whose oscillator sits above the signal and hands the band over the wrong way round. Leave it off unless the waterfall fills with signals that are all on the wrong side of the dial.
settings-mute-transmit = Mute on transmit
settings-silence-receive-audio-while-radio = Silence receive audio while this radio is transmitting. On by default: with the receiver on the same antenna, or on this radio's I.F., what it hears during an over is your own transmitter. Overs keyed at the radio itself — the microphone's PTT, a foot switch, MOX — count too, as soon as the CAT link says the rig has gone to transmit.
settings-blank-transmit = Blank on transmit
settings-stop-panadapter-waterfall-while-radio = Stop the panadapter and waterfall while this radio is transmitting. On by default: a transmitter painted across the whole span erases the band behind it. Turn it off to watch the band — or your own signal — through an over, which is worth having with the receiver on a separate antenna. Applies to overs sdroxide keys: one keyed at the radio is received right through, so the picture keeps running whatever this says.
settings-per-mode-offsets-radio-whose = Per-mode offsets, for a radio whose I.F. moves with the mode. A mode left empty uses the offset above.
settings-computer = On this computer
settings-add-radio-computer = Add a radio on this computer
settings-radio-s-settings-are-below = This radio's settings are below. Drag to move it along the strip.
settings-switch-radio-dialog-follows-drag = Switch to this radio (the dialog follows). Drag to move it along the strip.
settings-tx = ● TX
settings-station-radio = Station:
settings-station-radio-tip = The station radio. It runs the station's network services (spot feeds, rotator, amplifier, T/R switch) and takes the command line's settings, so it stays in the list. To rest it, switch it off with LINK; to use other hardware, choose another interface below.
settings-mute-radio-s-audio = Mute this radio's audio
settings-link = LINK
settings-close-radio-configuration-kept = Close this radio (its configuration is kept)
settings-close-tab-close-radio-station = Close this tab, or close the radio on its station
settings-close-tab = Close this tab
settings-transverters-full = { $max } is the most this table holds
settings-dialling = Dialling { $url }…
settings-region-note = Where the station is. Sets every band plan: the band edges, the CW / data / phone sub-segments on the waterfall strip, where the skimmers listen, the calling frequencies offered, and what counts as out of band for transmit. In Region { $region } that makes 70 cm { $region ->
        [1] 430–440 MHz — so 446 is out of band, and 40 m stops at 7.200
        [2] 420–450 MHz, and 40 m runs to 7.300
       *[3] 430–450 MHz, and 80 m stops at 3.900
    }. Takes effect at once, and applies to every radio at this station.
settings-country = Country
settings-country-off = — (no channel plans)
settings-country-system = Like this computer: { $country }
settings-country-germany = Germany
settings-country-note = Which licence-free channels the waterfall shows; national law sets these, not the IARU region. Germany: CB channels 1–80 (41–80 FM only, data channels in amber), Freenet 149.025–149.1125 MHz and PMR446 446.00625–446.19375 MHz, each as a block on the band-plan strip with channel tiles above it. Applies to this screen; by default it follows the country the computer is set to.
settings-rx-grid-reported = Receptions through this radio are reported from { $grid }, not from your own locator.
settings-rx-grid-invalid = { $grid } is not a Maidenhead locator, so nothing this radio hears is reported. Four or six characters: two letters, two digits, and optionally two more letters.
settings-bypassed = bypassed
# $state is one of settings-bypassed, settings-use or settings-every-stage-off.
settings-tx-audio-chain = Chain { $state }.
settings-gateway-via = { " " }via { $via }
settings-auto-upload-to = Auto-upload each new QSO to { $service }
settings-tle-updated = Updated { $n } { $n ->
        [one] subscription
       *[other] subscriptions
    }: { $total } satellites.
settings-tle-updated-failed = Updated { $ok } of { $n }; { $failed } failed — see the rows above.
settings-panadapter-of = This radio is { $radio }'s panadapter
settings-radio-n = radio { $n }
settings-on-station = On { $station }
settings-add-radio-on-station = Add a radio on { $station } — the station serves it straight away
settings-panadapter-receiver-for = Panadapter receiver for { $radio }
settings-hang-up-stays-on = Hang up. The radio stays on { $station }.
settings-close-on-station = Close { $radio } on { $station }? Its configuration stays on that machine; the tab here closes with it, and so does everyone else's.
settings-close-it-on = Close it on { $station }
settings-locator-hint = locator

## Settings window, Alerts (src/app/settings/alerts.rs)

settings-alerts-audible-alerts = Audible alerts
settings-alerts-sound-alarm-when-decode-matters = Sound an alarm when a decode matters
settings-alerts-plays-over-own-audio-output = Plays over its own audio output, so it is heard even when the band is in a different speaker than this screen.
settings-alerts-volume = Volume
settings-alerts-output = Output
settings-alerts-test = Test
settings-alerts-what-sound = What to sound
settings-alerts-each-station-quiet-while-after = Each station is quiet for a while after an alert, so a busy band does not ring every slot. Voice and Tone + voice are read by the spoken-announcement voice — switch it on in Settings → UI.

## Settings window, Amplifier (src/app/settings/amp.rs)

settings-amp-amplifier = Amplifier
settings-amp-waiting-station-s-amplifier-configuration = Waiting for the station's amplifier configuration…
settings-amp-hobbypcb-hardrock-500-hardrock-50 = A HobbyPCB Hardrock-500 or Hardrock-50 on its USB serial port. SDRoxide reads its meters, switches its band, mode and antenna, and runs its tuner from the AMP window. The amplifier is still keyed by the radio's PTT line as before.
settings-amp-enable = Enable
settings-amp-model = Model
settings-amp-untested-panel-has-only-been = Untested: this panel has only been tested with a Hardrock-500. Meter limits for this model are scaled from the Hardrock-500's and may be wrong. Watch the amplifier's own display.
settings-amp-port = Port
settings-amp-choose = — choose —
settings-amp-baud = Baud
settings-amp-must-match-amplifier-s-menu = Must match the amplifier's menu. HobbyPCB's own control panel needs 115200.
settings-amp-follow-radio-s-band = Follow the radio's band
settings-amp-sends-transmit-frequency-amplifier-every = Sends the transmit frequency to the amplifier on every band change, so its filters follow the radio without a CAT cable of their own. Never sent during a transmission.
settings-amp-show-power-swr-tx-meter = Show its power and SWR on the TX meter
settings-amp-only-where-radio-does-not = Only where the radio does not measure them itself, and only while the amplifier is in line.
settings-amp-off = Off
settings-amp-connecting = Connecting…
settings-amp-status = Status
settings-amp-apply = {" "}APPLY{" "}
settings-amp-save-re-open-amplifier-s = Save and (re)open the amplifier's port

## Settings window, Controls (src/app/settings/controls.rs)

settings-controls-keyboard = Keyboard
settings-controls-click-shortcut-rebind-then-press = Click a shortcut to rebind it, then press the key combination (Esc cancels). Bindings are ignored while you are typing in a text field.
settings-controls-shortcut = Shortcut
settings-controls-does = Does
settings-controls-step-mode = Step / mode
settings-controls-accel = Accel
settings-controls-on = On
settings-controls-press-key = press a key…
settings-controls-remove-binding = Remove this binding
settings-controls-add-shortcut = + Add shortcut
settings-controls-restore-defaults = Restore defaults
settings-controls-bind-hold-talk-space = Bind hold-to-talk to Space
settings-controls-hold-space-transmit-releasing-losing = Hold Space to transmit; releasing it — or losing window focus — unkeys
settings-controls-unkey-held-ptt-after = Unkey a held PTT after
settings-controls-backstop-against-stuck-key-controller = Backstop against a stuck key or a controller that stops reporting. 0 disables.
settings-controls-panadapter-mouse = Panadapter mouse
settings-controls-wheel = Wheel
settings-controls-wheel-shift = Wheel + Shift
settings-controls-tune-step = Tune step
settings-controls-zoom-rate = Zoom rate
settings-controls-click-tune-rounding = Click-tune rounding
settings-controls-invert-wheel-direction = Invert wheel direction
settings-controls-left-drag-tunes-well-pans = Left-drag tunes as well as pans
settings-controls-off-makes-left-drag-pan = Off makes left-drag pan the view only, like right-drag.
settings-controls-scroll-digit-frequency-readout-tune = Scroll a digit on the frequency readout to tune it
settings-controls-both-wheel-actions-are-tune = Both wheel actions are Tune — there is no way left to zoom.
settings-controls-restore-mouse-defaults = Restore mouse defaults
settings-controls-mouse-buttons = Mouse buttons
settings-controls-left-right-buttons-are-reserved = The left and right buttons are reserved for tuning and panning; the middle and extra buttons are free. A side button held for PTT behaves like a footswitch.
settings-controls-add-mouse-button = + Add mouse button
settings-controls-f1-always-opens-manual-even = F1 always opens this manual, even while typing, so it is not rebindable.
settings-controls-midi-controller = MIDI controller
settings-controls-midi-controllers-need-native-app = MIDI controllers need the native app — the browser client has no MIDI access.
settings-controls-any-class-compliant-midi-surface = Any class-compliant MIDI surface works: a DJ controller's jog wheel makes a fine VFO knob, its pads make PTT and band buttons, its faders make gain controls.
settings-controls-enable = Enable
settings-controls-controller = Controller
settings-controls-feedback = Feedback to
settings-controls-rescan-ports = Rescan ports
settings-controls-not-connected = Not connected.
settings-controls-move-control-see-here = Move a control to see it here.
settings-controls-control = Control
settings-controls-reads = Reads as
settings-controls-led = LED
settings-controls-move = move it…
settings-controls-click-then-move-control-you = Click, then move the control you want to bind
settings-controls-speed-sensitivity-spin-faster-tune = Speed sensitivity: spin faster to tune faster
settings-controls-send-current-value-back-light = Send the current value back, to light an LED or move a fader
settings-controls-add-midi-control = + Add MIDI control
settings-controls-clear-all = Clear all
settings-controls-endless-jog-encoders-send-relative = Endless (jog) encoders send a relative step rather than a position, in one of three encodings that look alike from small movements. LEARN guesses from a clockwise turn; if the knob then tunes the wrong way, tick “rev”.
settings-controls-none = — none —

## Settings window, General (src/app/settings/general.rs)

settings-general-system-default = System default
settings-general-remote-access = Remote access
settings-general-signed-in-station = Signed-in station
settings-general-sign-out = {" "}SIGN OUT{" "}
settings-general-sign-out-forgets-device = Forgets this device's sign-in. The station has to ask again on the next connection; this one stays up until you close it.
settings-general-password-kept-no-cookie = Remembering a password in this browser for { $stations }, because it answered no sign-in cookie. Older stations do this; it is not the same thing.
settings-general-what-remote-client-browser-page = What a remote client — the browser page, or another sdroxide started with --connect — has to give before this station will let it operate. Applies in server mode (--server); the next sign-in picks up a change, with no restart.
settings-general-username = Username
settings-general-password = Password
settings-general-clients-must-give-password-leaving = Clients must give the password. Leaving the username empty is fine.
settings-general-clients-must-sign = Clients must sign in.
settings-general-empty-anyone-who-can-reach = ⚠ Empty: anyone who can reach the server's port can operate this radio, on your callsign. Set a password before forwarding the port.
settings-general-stored-clear-config-toml-like = Stored in the clear in config.toml, like the other passwords sdroxide keeps.
settings-general-receive-audio-gain = Receive audio gain
settings-general-t-0-db = 0 dB
settings-general-extra-gain-everything-radio-sends =
    Extra gain on everything this radio sends to the speakers, on top of the volume control. Leave it at 0 dB unless the radio is quiet at full volume: the volume rail's top is the audio as it arrives, so it can turn a radio down but never up, and some transceivers' USB sound output sits well below full scale.

    Go up 6 dB at a time. Too much clips — the audio is limited at full scale rather than allowed to wrap round, so overdoing it sounds harsh rather than loud. Recordings are taken ahead of this and are not affected.
settings-general-reload-band-plan = RELOAD BAND PLAN
settings-general-re-read-bandplan-json-machine = Re-read bandplan.json on the machine the radio is attached to and apply it — band edges, sub-segments and skimmer windows — without restarting.
settings-general-running-station-s-own-band = Running on the station's own band plan.
settings-general-running-built-iaru-tables = Running on the built-in IARU tables.
settings-general-settings-file = Settings file
settings-general-settings-are-machine-radio-attached = The settings are on the machine the radio is attached to. Export them there.
settings-general-export = EXPORT…
settings-general-write-every-setting-station-radios = Write every setting at this station — the radios, the modes, the servers, the memories, the band plan — to one file you can carry to another installation. Your logbook and any saved server password stay here.
settings-general-import = IMPORT…
settings-general-replace-station-s-settings-ones = Replace this station's settings with the ones in a file exported from another installation. Restart sdroxide afterwards.
settings-general-import-overwrites-what-here-file = An import overwrites what is here, file for file, and takes effect the next time sdroxide starts — the settings already in memory would otherwise be written straight back over it. Anything the file does not mention is left as it is, so a bundle from a one-radio station does not remove a second radio here. Your logbook is never in the file: export it as ADIF from the LOG window if you want to move that too.
settings-general-sdroxide-settings = sdroxide settings
settings-general-swr-guard = SWR guard
settings-general-stop-transmitting-high-swr = Stop transmitting on high SWR
settings-general-trip = Trip at
settings-general-stops-transmission-when-radio-reports =
    Stops the transmission when the radio reports an SWR at or above this figure, and keeps transmit locked out until you acknowledge it. Catches a disconnected antenna, a failed feeder, or a switch left on the wrong port.

    Tuning is treated differently, because feeding a mismatch is the point of it: an antenna tuner gets double the limit and about five seconds before the guard applies at all. A manual tuner that takes longer than that wants the guard switched off for the session.

    Needs a rig that reports SWR over CAT. Ignores the first fifth of a second of each transmission, and does not wait for high power.
settings-general-per-mode-settings = Per-mode settings
settings-general-reset-every-mode = RESET EVERY MODE
settings-general-forget-every-mode-s-remembered = Forget every mode's remembered AGC, squelch, noise reduction, notch and stereo switches, and put each mode's own defaults back.
settings-general-selecting-mode-lays-mode-s = Selecting a mode lays that mode's own starting values for AGC, squelch, noise reduction, the notch and the stereo switches on the receiver, and changing one remembers it for that mode alone. The circular-arrow chip at the end of the receiver's filter/noise row lists what has been changed and puts the current mode back.
settings-general-your-audio-speakers-microphone = Your audio (speakers / microphone)
settings-general-output = Output
settings-general-channels = Channels
settings-general-which-speaker-channel-s-received = Which speaker channel(s) the received audio reaches. Left or right puts everything, mixed to mono, on that side only.
settings-general-input = Input

## Settings window, network services (src/app/settings/net.rs)

settings-net-operator = Operator
settings-net-callsign-grid-are-set-general = Callsign and grid are set on the General tab.
settings-net-set-your-callsign-grid-general = ⚠ Set your callsign and grid on the General tab.
settings-net-freedv-reporter = FreeDV Reporter
settings-net-enable = Enable
settings-net-connects-whenever-enabled-your-station = Connects whenever enabled. Your station is only shown to others while the radio is in RADE — in any other mode you stay connected but hidden.
settings-net-station = Station
settings-net-message = Message
settings-net-receive-only-i-cannot-transmit = Receive only (I cannot transmit)
settings-net-server = Server
settings-net-host = Host
settings-net-port = Port
settings-net-tls-wss = TLS (wss://)
settings-net-not-yet-implemented-freedv-gui = Not yet implemented — FreeDV GUI uses plain ws:// too.
settings-net-reporting = Reporting
settings-net-report-stations-i-decode = Report stations I decode
settings-net-sends-rx-report-each-callsign = Sends an rx_report for each callsign recovered from a RADE End-of-Over frame.
settings-net-show-other-reporter-stations-spots = Show other reporter stations as spots
settings-net-adds-them-panadapter-overlay-world = Adds them to the panadapter overlay, world map and SPOTS window under the FREEDV filter.
settings-net-set-your-callsign-grid-general-2 = ⚠ Set your callsign and grid on the General tab. Without both, the connection is view-only: you will see other stations but will not appear yourself.
settings-net-apply = {" "}APPLY{" "}
settings-net-persist-re-connect = Persist and (re)connect
settings-net-longwave-shortwave-stations-labelled-waterfall = The longwave and shortwave stations labelled on the waterfall. The shortwave schedule is downloaded from eibispace.de for the current broadcasting season and re-downloaded when the season changes.
settings-net-season = Season
settings-net-downloading = downloading…
settings-net-using-built-copy = using the built-in copy
settings-net-your-own-stations-corrections-go = Your own stations and corrections go in this file. sdroxide never writes it, and an entry here replaces a scheduled one with the same name and frequency.
settings-net-reload = Reload
settings-net-re-read-your-station-file = Re-read your station file after editing it
settings-net-download-schedule-now = Download schedule now
settings-net-fetch-season-s-schedule-again = Fetch this season's schedule again, replacing the cached copy
settings-net-broadcast-stations-labelled-waterfall-come = The broadcast stations labelled on the waterfall come from the list built into this build. Editing them needs the desktop app.
settings-net-ask-service-whether-these-credentials = Ask the service whether these credentials work. Applies the settings above first, and logs nothing: it only reads.
settings-net-checking = checking…

## Settings window: messages with values filled in.

settings-amp-connected-to = Connected to the { $model }
settings-controls-last-midi = Last message: { $message }  value { $value }
settings-general-band-plan-file = Every band edge and sub-segment comes from { $path }, written from the built-in IARU tables the first time and yours to edit after that — narrow a band to your licence and sdroxide will refuse to transmit outside it. Frequencies are in MHz; delete the file for a fresh copy of the defaults. This is the regional allocation, not your licence: your own conditions may be narrower, and national plans differ inside a region.
settings-general-exported = Exported { $note } — choose where to save it.
settings-general-export-failed = Export failed: { $error }
settings-general-import-failed = Import failed: { $error }
settings-general-imported = { $summary } — restart sdroxide to use them.
settings-general-import-skipped = Skipped { $path }: { $why }
settings-general-swr-tripped = ⚠ Currently tripped at { $swr }:1 — transmit is locked out.
settings-net-call-grid-set-on-general = { $call } / { $grid } — set on the General tab
settings-net-reporting-as = Reporting as { $call } / { $grid } — SDRoxide { $version }
settings-net-schedule-download-failed = ⚠ Download failed: { $error }. Using the built-in schedule.
settings-net-test-service = Test { $service }

## Settings window (src/app/settings/mod.rs), continued

settings-transmit-off-while-converter-set = Transmit is off while a converter is set — say what is in the transmit line above to turn it back on.
settings-transmit-goes-through-same-converter = Transmit goes through the same converter, so the radio works the same offset away from the dial in both directions.
settings-transmit-not-converted-radio-transmits = Transmit is not converted: the radio transmits on the frequency the dial shows, while receive comes through the converter.
settings-transmit-takes-own-offset-separate = Transmit takes its own offset, separate from the receive one.
settings-no-enabled-subscriptions-update = No enabled subscriptions to update.
settings-add-radio-here-station-you = Add a radio — here, or on a station you are connected to

## Settings window, General, continued

settings-general-every-band-edge-sub-segment = Every band edge and sub-segment comes from bandplan.json on the machine the radio is attached to. This is the regional allocation, not your licence: your own conditions may be narrower, and national plans differ inside a region.

## Settings window, picture upload (src/app/settings/picup.rs)

settings-picup-received-pictures-ftp = Received pictures → FTP
settings-picup-uploads-every-received-sstv-picture = Uploads every received SSTV picture (and, if ticked, every weather chart) as it is saved, laid out the way QSSTV does it so web pages written for QSSTV work unchanged.
settings-picup-upload-received-pictures = Upload received pictures
settings-picup-server = Server
settings-picup-port = Port
settings-picup-security = Security
settings-picup-user = User
settings-picup-password = Password
settings-picup-certificate = Certificate
settings-picup-trusted-when-computer-s-operating = Trusted when this computer's operating system trusts it, as a browser here would. A private CA can be added below (PEM).
settings-picup-extra-ca = Extra CA
settings-picup-only-certificate-fingerprint-accepted-way = Only the certificate with this fingerprint is accepted — the way to trust a self-signed server. TEST shows the server's.
settings-picup-any-certificate-accepted-link-encrypted = ⚠ Any certificate is accepted. The link is encrypted, but anyone in the path can pose as the server and read the password.
settings-picup-plain-ftp-sends-password-pictures = ⚠ Plain FTP sends the password and the pictures unencrypted.
settings-picup-sstv = SSTV
settings-picup-sstv-dir = SSTV dir
settings-picup-weather-fax = Weather fax
settings-picup-wefax-dir = WEFAX dir
settings-picup-empty-login-directory = Empty is the login directory.
settings-picup-file-names = File names
settings-picup-format = Format
settings-picup-add-extension = add extension
settings-picup-image1-png-rather-than-qsstv = image1.png rather than QSSTV's bare image1
settings-picup-caption-bar = Caption bar
settings-picup-time-frequency-mode-sender-s = Time, frequency, mode and the sender's FSK ID across the top, as QSSTV draws it. Only the uploaded copy is marked.
settings-picup-fit-960-768 = Fit 960 × 768
settings-picup-qsstv-s-heartbeat-tiny-picture = QSSTV's heartbeat: a tiny picture whose height is the minute, uploaded beside each one, and made two pixels wide when the program closes.
settings-picup-nat-workaround = NAT workaround
settings-picup-ignore-private-address-server-s = Ignore a private address in the server's passive-mode reply and use the server's own — for a server behind NAT that does not know its public address.
settings-picup-apply = {" "}APPLY{" "}
settings-picup-test = TEST
settings-picup-connect-settings-above-applied-not = Connect with the settings above — applied or not — log in, and write and delete a small file in each upload directory.
settings-picup-checking = checking…
settings-picup-trust-certificate = Trust this certificate
settings-picup-pin-switch-pinned-fingerprint-one = Pin it: switch to "Pinned fingerprint" with this one. Check it against the server first — this is what the server said, not proof of who it is. Then APPLY.
settings-picup-last-upload = Last upload

## Settings window, Profiles (src/app/settings/profiles.rs)

settings-profiles-profiles-station-s-saved-setups = Profiles: the station's saved setups
settings-profiles-profile-captures-whole-working-setup = A profile captures the whole working setup — the dials and VFOs, the mode and filters, the gains, drive and antennas, your callsign and message templates, and the per-band registers — and applies it back in one click. The hardware (back end, audio devices, converters) is deliberately left alone: a profile is the way you work the station, not the radio it runs on.
settings-profiles-save-current-setup = Save the current setup
settings-profiles-profile-name = profile name
settings-profiles-save = Save
settings-profiles-applies-whatever-sdroxide-attached-dials = Applies to whatever sdroxide is attached to: dials and VFOs, mode and filters, levels, gains, drive, antennas, the digital identity and the band stacks.
settings-profiles-no-profiles-saved-yet-name = No profiles saved yet — name one above and press Save.
settings-profiles-saved-profiles = Saved profiles
settings-profiles-apply = Apply

## Settings window, T/R switch (src/app/settings/relay.rs)

settings-relay-transmit-receive-switch = Transmit/receive switch
settings-relay-waiting-station-s-t-r = Waiting for the station's T/R switch configuration…
settings-relay-closes-contact-while-station-transmits = Closes a contact while this station transmits — to disconnect and ground the SDR's antenna input, and to key an amplifier or an outboard T/R relay in sequence with it. Works with the cheap USB relay boards (LCUS, KMtronic, Numato), with a serial RTS/DTR line into any interface that wants a PTT closure, and with anything else through a command hook.
settings-relay-hardware = Hardware
settings-relay-board = Board
settings-relay-lcus-four-byte-a0-command = LCUS is the four-byte A0-command board sold under a dozen names; KMtronic and Numato can also be asked what they are set to
settings-relay-port = Port
settings-relay-choose = — choose —
settings-relay-device = Device
settings-relay-gpio-pin = GPIO pin
settings-relay-pin-3-every-homebrew-plan = Pin 3 on every homebrew plan and every commercial board — it is the one at the end of the package that a wire can be tacked to. If the relay does not click, the card is a clone that does not implement these pins at all.
settings-relay-gpio-chip = GPIO chip
settings-relay-linux-only-gpiodetect-lists-chips = Linux only. `gpiodetect` lists the chips on the machine.
settings-relay-transmit = On transmit
settings-relay-receive = On receive
settings-relay-nothing-switched = Nothing is switched.
settings-relay-starting-program-costs-unpredictable-amount = Starting a program costs an unpredictable amount of time, and none of it is counted in the lead — so leave a wider margin here than you would for a relay board. The contact table's numbers mean nothing for this link: your commands are run once on key-down and once on key-up.
settings-relay-contacts = Contacts
settings-relay-contact-1-rts-contact-2 = Contact 1 is RTS and contact 2 is DTR.
settings-relay-no = No.
settings-relay-gpio = GPIO
settings-relay-name = Name
settings-relay-job = Job
settings-relay-tx-closes = TX closes
settings-relay-lead = Lead
settings-relay-hold = Hold
settings-relay-line-s-offset-chip-bcm = The line's offset on this chip — the BCM number on a Raspberry Pi header, which is not the physical pin number
settings-relay-sdr-antenna = SDR antenna
settings-relay-decides-default-timings-what-log = Decides the default timings and what the log calls it — and, for a band decoder, that the contact follows the band table below
settings-relay-transmitting-energises-coil-off-transmitting =
    On: transmitting energises the coil. Off: transmitting releases it.

    This is a wiring decision before it is a setting — see the note below.
settings-relay-how-long-before-rf-contact = How long before RF this contact moves from its band's RX word to its TX word. A retune moves it at once; this is for key-down, when a transmit-only filter has to be in circuit before the power is.
settings-relay-closed-long-before-rf-small = Closed this long before RF. A small coax relay throws in 5 to 15 ms. Transmit waits for the longest of these, so a large value is an audible gap in the receive audio; anything over 250 ms is ignored.
settings-relay-how-long-after-rf-stops = How long after RF stops this contact goes back from its TX word to its RX word.
settings-relay-opened-long-after-rf-stops = Opened this long after RF stops. Longer than the lead on purpose: letting the antenna back a moment late costs nothing, and letting it back early costs a front end. Zero is a real answer, and the right one for an amplifier's key line.
settings-relay-test = {" "}TEST{" "}
settings-relay-not-while-station-air = Not while the station is on the air
settings-relay-close-contact-half-second-so = Close this contact for half a second, so you can hear the relay and check the wiring with the transmitter cold
settings-relay-remove = Remove
settings-relay-add-contact = + add a contact
settings-relay-if-will-not-answer = If it will not answer
settings-relay-switch-exists-protect-receiver-not = A switch that exists to protect a receiver is not worth having if it silently stops protecting it — so refusing is the default. Choose the other where a loose USB cable ending your contest is the worse outcome.
settings-relay-order-timings-longest-lead-closes = The order is the timings: the longest lead closes first, and the longest hold opens last. So give the antenna relay the longer lead and the longer hold, and the amplifier the shorter of each — then the antenna is always switched before the amplifier is keyed, and the amplifier always unkeys before the antenna comes back.
settings-relay-transmit-sense-input = Transmit sense input
settings-relay-when-transceiver-keys-itself-own = When a transceiver keys itself — its own microphone button, foot switch, VOX or keyer — sdroxide finds out by asking over CAT, and that answer arrives a few hundred milliseconds into the over. Wire the rig's SEND line, through an opto-isolator, into a handshake input on this same port and it is seen in a few milliseconds instead. The meter and the transmit interlock get faster with it.
settings-relay-line = Line
settings-relay-transmitting = Transmitting is
settings-relay-high-line = a high line
settings-relay-most-opto-isolated-interfaces-pull = Most opto-isolated interfaces pull the line down when the rig keys, so this is usually off
settings-relay-belongs-radio = Belongs to radio
settings-relay-which-radio-tab-sensed-transceiver = Which radio tab the sensed transceiver is. On a station with one radio this is 1 and nothing depends on it.
settings-relay-needs-serial-port-handshake-lines = Needs a serial port — the handshake lines are on it.
settings-relay-wiring-only-fail-safe-survives = Wiring is the only fail-safe that survives this program.
settings-relay-choose-tx-closes-so-de = Choose “TX closes” so that the *de-energised* contact is the state you want when the board is unplugged, the computer is off, or sdroxide is not running. Wired so that de-energised means grounded, a dead relay leaves the SDR deaf — annoying, and safe. Wired the other way, a dead relay plus one transmission is a dead front end. With a real amplifier behind it, take the first.
settings-relay-cannot-protect-first-instant-over = This cannot protect the first instant of an over you start at the radio. Key the rig from sdroxide and the contacts always lead the RF; wire the sense input above and they follow within milliseconds. For a front end genuinely worth protecting, use an RF-sensed hardware T/R switch as well — no program on a computer can be the whole answer.
settings-relay-contacts-transmit = contacts in TRANSMIT
settings-relay-not-open = Not open.
settings-relay-status-unknown-press-apply = Status unknown — press APPLY.
settings-relay-station-air-apply-will-take = The station is on the air — APPLY will take effect after this over.
settings-relay-band-decoder-outputs = Band decoder outputs
settings-relay-one-control-word-per-band = One control word per band: bit 0 is contact 1, bit 1 contact 2 and so on, matching the numbering above. RX follows the receive dial's band; TX the band of whichever radio keyed, switched in on each contact's lead before RF and back after its hold — give them the same value for a filter that does not care which way the RF is going, and different ones for a receive-only preamp bypass or a transmit-only LPF bank. Bands left at 0 assert nothing. Applies on Apply / reconnect.
settings-relay-band = Band
settings-relay-rx = RX
settings-relay-tx = TX
settings-relay-outputs-asserted = Outputs asserted
settings-relay-other = Other
settings-relay-everywhere-outside-amateur-bands-short = Everywhere outside the amateur bands — short-wave listening, and anything a transverter's dial lands on that no band covers.
settings-relay-apply = {" "}APPLY{" "}
settings-relay-persist-re-open-switch = Persist and (re)open the switch

## Settings window, Remote (src/app/settings/remote.rs)

settings-remote-connect-server = Connect to a server
settings-remote-drive-sdroxide-running-somewhere-else = Drive an sdroxide running somewhere else — the shack machine, a remote site — over the network. Start it there with --server, and enter its address here. The radio, the decoders and the logbook stay on that machine; this screen gets the waterfall and the audio, and its speakers and microphone are the ones you use.
settings-remote-address = Address
settings-remote-host-name-ip = host name or IP
settings-remote-machine-running-sdroxide-server-host = The machine running sdroxide --server: a host name, an IPv4 or IPv6 address, or a complete ws:// (or wss://) URL if the server sits behind a reverse proxy — a URL is used exactly as typed and ignores the port and the switch below.
settings-remote-port = Port
settings-remote-port-server-listens-server-port = The port that server listens on — server_port in its config.toml. 4950 unless it was given --port.
settings-remote-secure = Secure
settings-remote-wss-tls = wss:// (TLS)
settings-remote-tick-where-server-sits-behind = Tick this where the server sits behind something that terminates HTTPS for it — a reverse proxy on port 443, which is the usual way a station is reached across the open internet. sdroxide's own server speaks plain ws:// and does not want this.
settings-remote-connect = {" "}CONNECT{" "}
settings-remote-open-station-radio-tab-own = Open this station as a radio tab of its own
settings-remote-client-cannot-hold-second-connection = This client cannot hold a second connection.
settings-remote-station-appears-tab-beside-radios =
    The station appears as a tab beside the radios already open here, and the tab strip at the top of the window switches between them (⊞ puts two side by side). Close it from the roster at the top of the Radio tab — that hangs up; nothing on the server is changed.

    If the server asks for a username and password, its sign-in screen comes up in the new tab. A plain ws:// link carries none of this encrypted, so across the open internet either put it through a VPN or an SSH tunnel, or terminate HTTPS in front of the server and tick Secure above. A proxy with a self-signed certificate is fine: the radio's tab shows its fingerprint and asks before trusting it.

## Settings window, Servers (src/app/settings/servers.rs)

settings-servers-wsjt-x-udp-broadcast = WSJT-X UDP broadcast
settings-servers-waiting-station-s-wsjt-x = Waiting for the station's WSJT-X broadcast configuration…
settings-servers-sends-decodes-station-status-logged = Sends decodes, station status and logged QSOs the way WSJT-X does, so GridTracker, JTAlert, N1MM+ and Log4OM work with sdroxide unchanged. Output only — nothing on this socket can touch the radio.
settings-servers-enable = Enable
settings-servers-send = Send to
settings-servers-t-127-0-0-1-reaches = 127.0.0.1 reaches clients on this machine; a LAN address or a multicast group (224.0.0.1) reaches others
settings-servers-port = Port
settings-servers-t-2237-port-every-client-defaults = 2237 is the port every client defaults to
settings-servers-identify = Identify as
settings-servers-name-clients-see-some-loggers = The name clients see. Some loggers only accept traffic identifying itself as WSJT-X.
settings-servers-n1mm-contactinfo-broadcast = N1MM+ contactinfo broadcast
settings-servers-same-news-n1mm-s-words = The same news in N1MM's words: one XML datagram per logged contact, for the loggers that listen for N1MM rather than for WSJT-X — the World Radio League's bridge takes either. Contacts only; there is no decode stream in this protocol. Its own switch and its own port, because a station may want both.
settings-servers-t-127-0-0-1-reaches-2 = 127.0.0.1 reaches clients on this machine. N1MM's own advice for a contest network is this subnet's broadcast address — 192.168.1.255 for a 192.168.1.n network — which reaches every position at once.
settings-servers-t-12060-port-n1mm-s-documentation = 12060 is the port N1MM's documentation recommends
settings-servers-station-name = Station name
settings-servers-what-n1mm-calls-stationname-name = What N1MM calls the StationName: the name of the computer that sent the packet. Loggers show it to tell one operating position from another.
settings-servers-decoded-cw-over-udp = Decoded CW over UDP
settings-servers-cw-panel-s-copy-plain = The CW panel's copy as plain text, one UDP datagram per piece as it settles — for a station controller, a logger's CW window or a script. No protocol: a listener just appends what arrives (try nc -ul 9999). Only settled text is sent, never the tail the decoder is still revising. Each radio has its own switch.
settings-servers-t-127-0-0-1-reaches-3 = 127.0.0.1 reaches programs on this machine; a LAN address or this subnet's broadcast address reaches others.
settings-servers-t-9999-default = 9999 by default
settings-servers-apply = {" "}APPLY{" "}
settings-servers-persist-re-open-broadcast-sockets = Persist and (re)open the broadcast sockets
settings-servers-hamlib-rigctld-server = Hamlib rigctld server
settings-servers-waiting-station-s-rigctld-configuration = Waiting for the station's rigctld configuration…
settings-servers-listen = Listen on
settings-servers-t-127-0-0-1-machine = 127.0.0.1 (this machine)
settings-servers-t-0-0-0-0-whole = 0.0.0.0 (whole network)
settings-servers-t-4532-port-every-rigctld-client = 4532 is the port every rigctld client assumes
settings-servers-rig-name = Rig name
settings-servers-reported-clients = reported to clients
settings-servers-max-clients = Max clients
settings-servers-allow-clients-transmit = Allow clients to transmit
settings-servers-off-refuses-every-key-request = Off refuses every key request and stops advertising a transmit range, so Hamlib itself declines to key.
settings-servers-rigctld-has-no-authentication-0 = ⚠ rigctld has no authentication. On 0.0.0.0, anyone who can reach this port can tune and key your transmitter.
settings-servers-t-1-client = 1 client
settings-servers-not-running = Not running.
settings-servers-status-unknown-press-apply = Status unknown — press APPLY.
settings-servers-persist-re-bind-server = Persist and (re)bind the server
settings-servers-lets-any-hamlib-capable-program = Lets any Hamlib-capable program drive this radio: frequency, mode, PTT, split, RIT and power. In WSJT-X, fldigi or CQRLOG choose the rig “Hamlib NET rigctl” (model 2) and point it at this address; in GPredict and N1MM enter the host and port directly. Unlike the TCI server it carries control only — no audio, no IQ.
settings-servers-antenna-rotator = Antenna rotator
settings-servers-waiting-station-s-rotator-configuration = Waiting for the station's rotator configuration…
settings-servers-points-motorized-antenna-satellite-you = Points a motorized antenna at the satellite you are locked onto, through a Hamlib rotctld daemon (run e.g. “rotctld -m 603 -r /dev/ttyUSB0” for a GS-232B rotator, or “rotctld -m 1” to try it without hardware) — one daemon covers every rotator Hamlib drives — or through PSTRotator, with UDP Control enabled in its Setup.
settings-servers-program = Program
settings-servers-host = Host
settings-servers-t-4533-rotctld-s-default-pstrotator = 4533 is rotctld's default. PSTRotator's UDP control port is 12000 by default, and it answers position queries on the next port up, 12001 — which has to be free on this machine.
settings-servers-min-elevation = Min elevation
settings-servers-below-rotator-parks-instead-tracking = Below this the rotator parks instead of tracking — set it to your local horizon or roofline
settings-servers-azimuth-offset = Azimuth offset
settings-servers-added-every-command-rotator-whose = Added to every command, for a rotator whose north is off
settings-servers-min-movement = Min movement
settings-servers-steps-smaller-than-are-not = Steps smaller than this are not sent — chasing tenths of a degree wears motors for nothing
settings-servers-park-position = Park position
settings-servers-drive-position-when-idle = drive to a position when idle
settings-servers-off-leaves-antenna-wherever-last = Off leaves the antenna wherever the last pass ended
settings-servers-not-connected = Not connected.
settings-servers-persist-re-connect-daemon = Persist and (re)connect to the daemon
settings-servers-waiting-station-s-tci-server = Waiting for the station's TCI server configuration…
settings-servers-built-tci-server = Built-in TCI server
settings-servers-device-name = Device name
settings-servers-off-leaves-control-receive-streams = Off leaves control and the receive streams working, but every key request is refused.
settings-servers-tci-has-no-authentication-0 = ⚠ TCI has no authentication. On 0.0.0.0, anyone who can reach this port can tune and key your transmitter.
settings-servers-tci-client = TCI Client
settings-servers-when-you-tune-send-radio = When you tune, send this radio's VFO to another TCI server so that rig tunes along. Control only — no audio, no IQ, and it does not key the transmitter.
settings-servers-server = Server
settings-servers-same-address-server-above-client = This is the same address as the server above. The client connects only when that server is not the one listening — leave it off, or on another port, when 590Ctrl is using this port.
settings-servers-tci-client-connected = ● TCI Client connected
settings-servers-tci-client-connecting = TCI Client connecting…
settings-servers-tci-client-press-apply = TCI Client — press APPLY.
settings-servers-persist-re-bind-server-connect = Persist and (re)bind the server, and connect the TCI client
settings-servers-server-lets-tci-capable-programs = The server lets TCI-capable programs drive this radio: frequency and mode control, wideband IQ, receive audio, and transmit audio. In WSJT-X choose the SunSDR (TCI) rig type, point it at this address, and set PTT to TCI. TCI Client is the other direction: sdroxide's dial is sent as a VFO command to 590Ctrl (or any TCI server) at the address above. It does not pass audio.

## Settings window, TLE (src/app/settings/tle.rs)

settings-tle-satellites-element-sets-frequencies = Satellites: element sets and frequencies
settings-tle-waiting-station-s-satellite-configuration = Waiting for the station's satellite configuration…
settings-tle-tracker-already-fetches-celestrak-s = The tracker already fetches CelesTrak's amateur group on its own. This is for everything else: the NOAA weather birds, a cubesat too new to be in the group, or a fresher element set than the one that arrived. The listings are fetched — and kept — where the radio engine runs, so this is the same set of satellites wherever the app is open.
settings-tle-subscriptions = Subscriptions
settings-tle-listings-fetched-kept-current-same = Listings fetched and kept current, on the same six-hourly cadence as the amateur set. Refreshed while the solar window is open, and by UPDATE NOW here.
settings-tle-fetch-track-listing = Fetch and track this listing
settings-tle-remove-subscription = Remove this subscription
settings-tle-orbits = Orbits
settings-tle-which-satellites-listing-get-orbit = Which satellites in this listing get an orbit ring and a label. A whole group wants "curated": ninety rings at once is unreadable, and none at all leaves ninety anonymous dots.
settings-tle-nothing-listing-sdroxide-s-curated = Nothing in this listing is in sdroxide's curated list — that list is ten amateur satellites, so this would behave exactly like "none".
settings-tle-all-satellites = all satellites
settings-tle-catalogue-numbers-keep-comma-separated = Catalogue numbers to keep, comma separated. Empty tracks everything the listing carries.
settings-tle-not-fetched-yet = not fetched yet
settings-tle-subscription = + Subscription
settings-tle-update-now = {" "}UPDATE NOW{" "}
settings-tle-ask-radio-engine-fetch-every = Ask the radio engine to fetch every enabled subscription now
settings-tle-celestrak-groups = CelesTrak groups
settings-tle-pasted-element-sets = Pasted element sets
settings-tle-one-off-these-do-not = For a one-off. These do not update themselves, and SGP4 stops propagating an element set once it is a fortnight past its epoch — subscribe instead for anything you mean to keep.
settings-tle-track-one = Track this one
settings-tle-show-two-element-lines = Show the two element lines
settings-tle-remove = Remove
settings-tle-paste-two-three-line-element = Paste two- or three-line element sets here
settings-tle-add-pasted = + Add pasted
settings-tle-nothing-paste-box-looked-like = Nothing in the paste box looked like an element set.
settings-tle-clear-box = Clear box
settings-tle-frequencies = Frequencies
settings-tle-shown-under-pass-table-solar = Shown under the pass table in the solar window. An entry here replaces the built-in one for that catalogue number outright, so start from a copy of it unless you mean to drop the rest.
settings-tle-remove-satellite-s-entry = Remove this satellite's entry
settings-tle-link = LINK
settings-tle-downlink = DOWNLINK
settings-tle-uplink = UPLINK
settings-tle-mode = MODE
settings-tle-note = NOTE
settings-tle-fm-repeater = FM repeater
settings-tle-fm = FM
settings-tle-ctcss-67-0-hz = CTCSS 67.0 Hz
settings-tle-link-2 = + Link
settings-tle-copy-built = Copy built-in
settings-tle-frequency-either-one-number-145 = A frequency is either one number (145.800) or a transponder passband written 145.950-145.970. Leave a direction blank for a beacon.
settings-tle-norad = NORAD
settings-tle-satellite = + Satellite
settings-tle-frequency-entry-needs-satellite-s = A frequency entry needs the satellite's NORAD catalogue number.

## Settings window, Wavelog (src/app/settings/wavelog.rs)

settings-wavelog-address = Address
settings-wavelog-api-key = API key
settings-wavelog-your-wavelog-s-front-page = Your Wavelog's front-page address — https://log.example.org or http://192.168.1.10/wavelog — and a key from its API page. Uploading needs a read/write key.
settings-wavelog-plain-http-address-internet-api = Plain http:// to an address on the internet: the API key travels unencrypted. Fine inside your own network; use https:// across the internet.
settings-wavelog-trusted-cert = Trusted cert
settings-wavelog-forget-certificate = Forget this certificate
settings-wavelog-go-back-ordinary-checking-against = Go back to ordinary checking against the public certificate authorities — for a server that now has a proper certificate.
settings-wavelog-server-presented-certificate-sha-256 = The server presented this certificate (SHA-256):
settings-wavelog-compare-your-server-s-e = Compare it with your server's — e.g. `openssl x509 -noout -fingerprint -sha256 -in cert.pem` — and trust it only if they match. From then on this exact certificate is accepted and any other is refused.
settings-wavelog-trust-certificate = Trust this certificate
settings-wavelog-station-location = Station location
settings-wavelog-none = — none —
settings-wavelog-press-test-above-list-station = Press Test above to list the station locations defined in your Wavelog.
settings-wavelog-default = Default
settings-wavelog-where-qsos-go-when-operating = Where QSOs go when the operating profile does not name a location.
settings-wavelog-profile = This profile
settings-wavelog-use-default = Use the default
settings-wavelog-not-set = not set
settings-wavelog-no-station-location-chosen-uploads = No station location chosen — uploads will fail until one is.
settings-wavelog-qsos-not-wavelog = QSOs not in Wavelog
settings-wavelog-every-qso-logbook-wavelog = Every QSO in the logbook is in Wavelog.
settings-wavelog-everything-logbook-wavelog-has-not = Everything in the logbook Wavelog has not acknowledged — including contacts from before auto-upload. Ones it already has come back as duplicates and are simply marked as sent.
settings-wavelog-set-address-key-station-location = Set the address, key and a station location first.
settings-wavelog-yes-send-them = Yes, send them
settings-wavelog-cancel = Cancel
settings-wavelog-download-into-logbook = Download into this logbook
settings-wavelog-downloading-runs-where-radio-attached = Downloading runs where the radio is attached, into that machine's logbook — set it up there.
settings-wavelog-download-qsos-start-every = Download QSOs at start and every
settings-wavelog-downloaded-qsos-join-logbook-so = Downloaded QSOs join the logbook, so FT8 and the spot lists know what you have worked from other programs and rigs. Each download fetches only what is new.
settings-wavelog-press-test-above-choose-which = Press Test above to choose which station locations to download.
settings-wavelog-sync-now = Sync now
settings-wavelog-fetch-what-wavelog-has-added = Fetch what Wavelog has added since the last download. Applies the settings above first.
settings-wavelog-full-resync = Full resync…
settings-wavelog-download-ticked-locations-whole-compare = Download the ticked locations whole and compare them with the log: QSOs edited or deleted in Wavelog, or from a location no longer ticked, are brought into line. You see the changes before anything happens.
settings-wavelog-full-resync-changes-make = Full resync — changes to make
settings-wavelog-qsos-logged-here-imported-file = QSOs logged here or imported from a file are never changed or removed — only ones that were downloaded from Wavelog.
settings-wavelog-apply = Apply
settings-wavelog-discard = Discard
settings-wavelog-live-radio-status = Live radio status
settings-wavelog-send-frequency-mode-wavelog-they = Send frequency and mode to Wavelog as they change
settings-wavelog-radio-name = Radio name
settings-wavelog-pick-radio-wavelog-s-live = Pick this radio in Wavelog's live logging and its form follows the dial. Needs a read/write key. From the station's main radio only.

## Settings window, picture upload: messages with values

settings-picup-server-fingerprint = Server certificate SHA-256: { $fingerprint }

## Settings window, Profiles: messages with values

settings-profiles-apply-hint = Put the station back onto “{ $name }”: dials, VFOs, mode, filters, gains, drive, antennas, identity and band stacks.
settings-profiles-delete = Delete “{ $name }”

## Settings window, T/R switch: messages with values

settings-relay-not-usable = Not usable yet: { $why }
settings-relay-receiving = receiving
settings-relay-rx-tx-words = { $rx } / TX { $tx }

## Settings window, Servers: messages with values

settings-servers-n-clients = { $n ->
        [one] { $n } client
       *[other] { $n } clients
    }
settings-servers-listening = ● Listening on { $addr } — { $clients }
settings-servers-failed = Failed: { $error }
settings-servers-rotator-at = ● Connected — antenna at { $az }° az, { $el }° el
settings-servers-tci-client-error = TCI Client: { $error }

## Settings window, TLE: messages with values

settings-tle-subscription-age = { $count } satellites · { $age } old
settings-tle-subscribed = Subscribed to { $name }. Press UPDATE NOW to fetch it.
settings-tle-norad-too-stale = NORAD { $norad } · { $age } old — too stale to propagate
settings-tle-norad-age = NORAD { $norad } · { $age } old
settings-tle-norad-n = NORAD { $norad }
settings-tle-added = Added { $n } element { $n ->
        [one] set
       *[other] sets
    }.
settings-tle-added-refreshed = Added { $added } and refreshed { $r } element { $r ->
        [one] set
       *[other] sets
    }.
settings-tle-links = { $n } { $n ->
        [one] link
       *[other] links
    }
settings-tle-replace-links = Replace these links with the built-in ones for { $name }

## Settings window, Wavelog: messages with values

settings-wavelog-call-mismatch = Location "{ $location }" is { $station_call }, but you log as { $call } — Wavelog will refuse these QSOs.
settings-wavelog-grid-mismatch = Location "{ $location }" is at { $station_grid }, but your grid is { $grid } — Wavelog will refuse these QSOs.
settings-wavelog-location-n = location #{ $id }
settings-wavelog-saved-with-callsign = Saved with the callsign ({ $call }) in Settings → Profiles, so a portable profile can log to a portable location.
settings-wavelog-owed = { $n } { $n ->
        [one] QSO
       *[other] QSOs
    } logged since auto-upload was switched on still to send — retried every 5 minutes.
settings-wavelog-send-unsent = Send { $n } { $n ->
        [one] QSO
       *[other] QSOs
    } not yet in Wavelog…
settings-wavelog-send-confirm = Send { $n } QSOs to { $location }? Settings are applied first.
settings-wavelog-plan-add = • { $n } QSOs in Wavelog to add to the log
settings-wavelog-plan-update = • { $n } downloaded QSOs changed in Wavelog to update
settings-wavelog-plan-remove = • { $n } downloaded QSOs no longer in Wavelog (or from an unticked location) to remove
settings-wavelog-plan-mark = • { $n } of your own QSOs to mark as being in Wavelog

## Settings window, Radio tab (src/app/settings/radio.rs)

settings-radio-antenna = Antenna
settings-radio-which-socket-back-radio-using =
    Which socket on the back the radio is using — its own ANT command, the same setting as the ANT button on the front panel.

    Applies immediately, and it is the radio's setting rather than a copy kept here: the socket the radio is on is read back when the session opens.

    The choice is remembered per band. Switch to the beam on 2 m and the wire on 40, and each comes back the next time the dial crosses into that band — and the next time sdroxide starts. A memory channel stored here carries its socket too.
settings-radio-receive-antenna = Receive antenna
settings-radio-radio-s-separate-receiving-antenna =
    The radio's separate receiving antenna — RX ANT on the back — switched into the receive path or out of it. Its own setting, the same one as the radio's RX ANT button.

    The aerial on the main socket stays on transmit either way: this is an extra input for receiving, a loop or a beverage, not a choice of socket.

    Unlike the antenna above, this is *not* remembered here. The radio holds it per band itself, so sdroxide reads it back after every band change and shows what the radio says; ticking this box is the only thing that moves it.
settings-radio-receive-path = In the receive path
settings-radio-radio-power = Radio power
settings-radio-switch-radio-itself-off-back =
    Switch the radio itself off, and back on again, over the control link. This is the one true on/off in the program — not sdroxide's own LINK switch, which closes sdroxide's end and leaves the radio running.

    For the switch back on to reach anything, the radio's control end has to stay awake while it is off. Over the network that is what Network Control does. Over a serial cable it is the radio's CI-V port, which stays powered from the mains supply on a set that is switched off at the front rather than unplugged; sdroxide sends the wake-up run Icom's own documentation asks for in front of the power-on.

    Switching off ends the audio and the meters — there is no radio behind them — and sdroxide keeps the control link open so the switch back on has somewhere to go.
settings-radio-on = On
settings-radio-off = Off
settings-radio-waiting-configuration-machine-radio-attached = Waiting for the configuration of the machine the radio is attached to.
settings-radio-sound-format = Sound format
settings-radio-invert-spectrum = Invert spectrum
settings-radio-swap-i-q = Swap I/Q
settings-radio-mirror-panadapter-about-tuned-frequency =
    Mirror the panadapter about the tuned frequency, for a rig that carries I and Q the other way round on its sound card. The giveaway is a waterfall full of convincing signals that are all on the wrong side of the dial, with SSB coming out on the opposite sideband — swapping the two cables at the sound card would fix it just as well.

    Receive only: transmit hands the radio one real audio signal, which has no sideband to invert.
settings-radio-i-q-sample-rate = I/Q sample rate
settings-radio-how-fast-radio-s-i =
    How fast the radio's I/Q sound card is run — and so how much band the panadapter shows, because a quadrature stream spans its whole sample rate: 48 kHz gives ±24 kHz either side of the dial, 192 kHz gives ±96.

    The card has the final say. One that cannot do the rate picked here is opened at the nearest it can, and the panadapter is that much narrower — the log says so at startup when it happens.

    A faster card is also more work: the machine has proportionally less time to empty it, and one that cannot keep up drops samples. That shows up as audio breaking up while the waterfall still looks perfect, and it too is logged. If you see it, come back down a step.
settings-radio-t-48-khz-24-khz = 48 kHz (±24 kHz)
settings-radio-t-96-khz-48-khz = 96 kHz (±48 kHz)
settings-radio-t-192-khz-96-khz = 192 kHz (±96 kHz)
settings-radio-t-384-khz-192-khz = 384 kHz (±192 kHz)
settings-radio-i-q-centre-offset = I/Q centre offset
settings-radio-how-far-radio-s-i =
    How far the radio's I/Q output is centred above its own dial, for a rig whose receive I.F. has been moved off zero — on an Elecraft, MENU:RX SHFT set to 8.0 instead of NOR, which takes the dial off the mixer's DC spike (and stops a strong nearby SSB/AM station being AM-detected).

    Not a converter: the radio still displays and transmits on the real frequency, and nothing here is ever sent to it. This only says where the samples on the sound card sit, and the stream is shifted back onto the dial as it arrives.

    Leave at 0 unless you have turned such a menu entry on. If signals land twice the offset away, the sign is the other way round.
settings-radio-iq-correction = IQ correction
settings-radio-cancel-mirror-image-every-signal =
    Cancel the mirror image of every signal, and the DC spike in the middle of the waterfall.

    A radio's I/Q output is two analogue paths that are never quite equal in gain nor exactly 90° apart, and what that leaves is a copy of every signal reflected about the tuned frequency, usually 30-40 dB down — strong enough to look like a station that is not there, and to be decoded as one. Radios with front-panel balance trimmers are adjusting exactly this; here it is measured off the received noise and needs no setting.

    Leave it on. Turn it off if you are listening to AM tuned dead on the carrier — the carrier is DC, so it goes with the spike — or to check whether a signal is real by watching whether it survives.
settings-radio-cancel-mirror-images = Cancel mirror images
settings-radio-dc-notch = DC notch
settings-radio-widen-hole-taken-out-middle =
    Widen the hole taken out of the middle of the span, for a radio whose centre spike is broader than the offset underneath it.

    0 leaves the ordinary blocker, which is a few tens of hertz wide and removes the offset without touching anything else. Wind it up and the bottom of the span goes with it: a first-order high-pass, 3 dB down at the figure set here and further in below it.

    It is centred where the radio's I/Q is centred, which is the dial unless the offset above has moved it — so anything tuned there goes too. A CW note at 600 Hz is inside a 600 Hz setting.
settings-radio-panadapter-bw = Panadapter BW
settings-radio-cat-family = CAT family
settings-radio-rigctld-address = rigctld address
settings-radio-host-port-running-hamlib-rigctld =
    host:port of a running Hamlib rigctld — 127.0.0.1:4532 is its own default, on this machine.

    Start one with, for example, `rigctld -m 2028 -r /dev/ttyUSB0 -s 38400`, where -m is the Hamlib model number for your radio (`rigctl -l` lists them).

    This is the catch-all: it reaches the frequency, mode, PTT, power, S-meter and SWR and nothing else. Where one of the native families above fits your radio it does more — keying from the rig's own text buffer, the receive filter, and per-model meter scales.
settings-radio-flrig-address = flrig address
settings-radio-host-port-running-flrig-127 =
    host:port of a running flrig — 127.0.0.1:12345 is its own default, on this machine. flrig serves XML-RPC whenever it is running; the port is under its Config → Setup → Server.

    Like the Hamlib option this drives a daemon rather than the radio, but through flrig's own per-model driver — on a number of rigs its power and filter handling is the more faithful of the two. It reaches the frequency, mode, PTT, transmit power (in whole watts), the receive bandwidth, the S-meter, SWR and power-out. CW keys through flrig's own cwio port, which must be configured in flrig itself.
settings-radio-serial-port = Serial port
settings-radio-select = — select —
settings-radio-baud = Baud
settings-radio-data-bits = Data bits
settings-radio-parity = Parity
settings-radio-stop-bits = Stop bits
settings-radio-force-rts = Force RTS
settings-radio-force-dtr = Force DTR
settings-radio-ptt-method = PTT method
settings-radio-how-transmit-keyed = How transmit is keyed.
settings-radio-how-transmit-keyed-network-link = How transmit is keyed. A network link has no control lines, so DTR and RTS key nothing here — use CAT, which asks the daemon to key the radio, or VOX.
settings-radio-mode-control = Mode control
settings-radio-digimode-mode = Digimode mode
settings-radio-what-put-radio-mode-sdroxide =
    What to put the radio in for a mode sdroxide modulates through its sound card. "DIGI" selects the rig's DATA/PKT position, which takes the transmit audio from the USB or ACC input with the microphone path's speech processing out of it; "USB" leaves it on the plain sideband, for a radio already set to modulate SSB from its data input or one with no DATA position at all.

    It covers every digital mode — FT8, FT4, PSK, RTTY, SSTV and the rest — and CW sent as "Sound card", and it overrides Mode control for them. SSTV is the one whose sideband follows the band, so there it means the DATA position on that sideband: USB-D above 40 m, LSB-D at and below it. SSTV-FM is not part of it — an FM carrier has no sideband to choose.
settings-radio-cw-keying = CW keying
settings-radio-how-cw-panel-s-keyer =
    How the CW panel's keyer transmits. "Rig keyer" puts the radio in CW and hands it the text to send with its own keyer. It uses the rig's keyer speed (set from the panel's WPM), and on Yaesu it sends by way of keyer memory 1, overwriting whatever was stored in it.

    It needs break-in on at the radio, which sdroxide turns on itself with each message on Icom, Yaesu and Kenwood — a rig with break-in off takes the message and transmits nothing. It is left on afterwards, and a full break-in is never turned down to semi.

    "Sound card" sends the keyed sidetone as audio instead (MCW), a tone at dial + pitch — and because a rig in CW would ignore its sound card entirely, selecting CW then follows the Digimode mode setting (USB, DATA, or Radio controlled) instead of switching the rig to CW. Pick "Radio controlled" there for rigs whose data position can't be commanded over CAT (a Xiegu's U-D) and park the rig on it yourself, as for FT8.
settings-radio-poll-rate = Poll rate
settings-radio-how-often-radio-asked-what =
    How often the radio is asked what it is doing — its dial, its mode and its meters. This is the half of the control link that runs from the radio back to sdroxide: turn the rig's own VFO knob or change its mode on the front panel and the readout, the band and the panadapter follow within one poll.

    It is also the whole of the control traffic this end generates, and on a modern Icom that traffic is not free: the CI-V port and the sound card sit behind one USB hub inside the radio, so frames asked for here are bus time the audio does not get — heard as dropouts that look like a DSP fault and are not one. The default of 2 Hz is half a second behind the knob and quiet enough to stay out of the audio's way. Raise it where the control port is its own device (a separate USB-serial adapter, a network rigctld); lower it if the radio's audio breaks up.

    An Icom with CI-V Transceive switched on in its own menu needs none of this for the dial: it reports the knob the instant it moves, and sdroxide stands the dial poll down as soon as it sees it do so — and picks it back up again if the radio ever moves without saying so.

    The rate is the dial's. The mode rides along with only every fourth poll, since it is a setting that changes a few times in an evening rather than one that follows a knob.
settings-radio-send-command = Send command
settings-radio-which-transceiver-generation-keys-rig =
    Which transceiver generation keys the rig, for PTT method "CAT". The two disagree about what the TX parameter means and nothing on the wire tells them apart, so pick the one that matches your radio.

    "TS-2000 style (TX;)" — TS-480, TS-570, TS-870, TS-2000: the ordinary send, on the main band.

    "TS-590 style (TX1;)" — TS-590S/SG, TS-890, TS-990: DATA SEND, which keys with the ACC2/USB audio input live. On these rigs the plain send selects the microphone instead and mutes the audio sdroxide transmits.

    Set wrong, a TS-590 transmits silence — but a TS-2000 transmits on the sub-band, which is another band entirely.
settings-radio-radio = Radio
settings-radio-k3-k3s-kx3-kx2-k4 = K3 · K3S · KX3 · KX2 · K4
settings-radio-one-profile-covers-family-k3 =
    One profile covers the family: the K3 command set, which the KX3, KX2 and K4 all answer. There is nothing to pick here — how many watts the Drive slider spans (12 on a bare KX2 or KX3, 110 with a KPA3 or a KXPA100) is read from the rig's own option-module query when the port opens.

    Note the baud rates above: the K3, K3S, KX3 and KX2 go no faster than 38400, and a rig set below the rate chosen here answers nothing at all.
settings-radio-transmit-input = Transmit input
settings-radio-where-radio-takes-transmit-audio =
    Where the radio takes its transmit audio from — the rig's TI command, which is menu 32 "TX IN" at the front panel. Asserted when the port opens.

    "USB audio" is what makes this interface work: the FDM-DUO transmits what sdroxide puts into its USB sound card. A radio left on "Microphone" sends the room instead, and nothing on screen says so.

    "Auto" lets the rig choose — the microphone for a PTT press on the microphone, the USB port for a CAT or RTS key-down.

    "Leave as set on the radio" sends no TI at all, for an operator who sets this at the front panel.
settings-radio-which-two-sockets-back-receiver =
    Which of the two sockets on the back the receiver listens on — the rig's AN command, which is menu 31 "ANTENNAS" at the front panel and the "ANT 1 2" indicator on its display.

    "RTX" is one antenna doing both jobs, on the M-type socket that also carries transmit. "RX only" moves receive to the second socket and leaves transmit on RTX — a receiving antenna, a loop or a beverage, with the beam still on the transmitter.

    Applies immediately, and is read back from the radio when the port opens: this is the rig's own setting rather than a copy kept here.
settings-radio-one-profile-covers-both-there =
    One profile covers both. There is nothing to pick here — the radio names itself when the port opens.

    This is the CAT half only: it drives the dial, the mode, PTT, the S-meter, the SWR and the transmit power over the rig's CAT USB port, and takes audio from its USB Audio port. The FDM-DUO's third USB port carries wideband I/Q, which this interface does not read — select the ELAD interface above for that.

    Note the baud rate: menu 70 "CAT BAUD" on the radio must match, and it ships at 38400.

    CW is keyed by the radio's own key or paddle. The FDM-DUO has no command that accepts text, so the CW panel cannot key it over CAT.
settings-radio-one-profile-covers-range-qrp =
    One profile covers the range: QRP Labs' own command set, which is a subset of the Kenwood TS-480's with a good deal added. There is nothing to pick here — the radio names itself and its firmware version when the port opens, and the version is what decides whether the SWR-protection read is asked for at all.

    Not the Kenwood profile: on a QMX the PC command is the power METER rather than the power control, so a radio driven as a Kenwood would have its meter read written to as if it were a setting, and MD8 — a Kenwood mode — is SWR Tune here.

    The transmit power is set at the radio. There is no CAT command for it, so the Drive slider only reaches the level of the audio going into the sound card.

    The receive filter is the radio's too: it reports the width its mode implies (3.2 kHz in Digi, 300 Hz in CW) and offers nothing to change it with.
settings-radio-baud-rate-ignored-port-usb = Baud rate is ignored — the port is USB
settings-radio-qmx-serves-own-virtual-com =
    A QMX serves its own virtual COM ports over USB, so the rate set above has no effect on either end. It offers up to three of them; if the radio answers nothing, the most likely reason is the wrong one.

    One thing does matter about the port: a carriage return sent to it switches the radio into terminal mode for the rest of the session. sdroxide never sends one — but a terminal program left open on the same port will, and CAT stops working the moment it does.
settings-radio-i-q-mode-switched-radio = I/Q mode is switched on at the radio
settings-radio-radio-s-sound-card-carries =
    The radio's sound card carries either demodulated audio or the raw I/Q its ADC sees, and sdroxide asserts whichever the Sound format above asks for when the port opens (the radio's Q9 command, the "IQ mode" menu entry).

    The centre offset and sample rate above have been set to what a QMX needs: its receiver is a superhet with a 12 kHz I.F., so the synthesiser sits 12 kHz below the dial and everything on the card is 12 kHz above the middle of the span. The card runs at 48 kHz, which makes the panadapter 48 kHz wide.

    In CW the radio moves that I.F. by a further ~700 Hz, which one figure cannot follow — add it by hand if you run the panadapter with the radio in CW. In Digi, where a radio used as an I/Q front end normally sits, there is nothing to add.

    QRP Labs note that I/Q mode is not suitable for WSJT-X and other programs that expect demodulated audio; here the demodulation happens on this side, so it is.
settings-radio-rs-hfiq-5-w-hf = RS-HFIQ (5 W HF transceiver)
settings-radio-hobbypcb-s-rs-hfiq-there =
    HobbyPCB's RS-HFIQ. There is nothing to pick here: the profile is the whole of the radio's command set, and the firmware version is logged when the port opens.

    The control link carries two things and no more — where to put the oscillator, and whether to transmit. There is no mode, power, filter, squelch or meter command, because the radio has none of them. Everything else is sdroxide's: what comes off the sound card is complex baseband centred on the dial, so the mode, the filter and the modulation all happen on this side.

    The frequency command covers 3–30 MHz. Ask for anything outside that and the radio refuses it and stays where it is; the log says so.

    CW is keyed as audio through the transmit chain — this radio has no keyer that takes text. (It has an internal CW generator; sdroxide never uses it, on the firmware's own advice.)
settings-radio-sound-format-baud-ptt-are = Sound format, baud and PTT are the radio's, not settings
settings-radio-all-three-have-been-filled =
    All three have been filled in and none of them is a preference:

    • Sound format is I/Q (stereo) — the card carries complex baseband and nothing else.
    • 57600 baud, 8N1 — the firmware's one rate, with no menu to change it. Any other is a silent link, so it is pinned when the port opens.
    • PTT over CAT — the *X command is the only transmit switch the interface has.

    The centre offset has been set to zero: the synthesiser runs at four times the dial into a quadrature detector, so the middle of the span is the dial. Set the sample rate above to whatever your sound card is actually running at; that is what makes the panadapter as wide as it is.
settings-radio-dl2man-pe1nnz-s-pocket-qrp =
    DL2MAN/PE1NNZ's pocket QRP transceiver and the open uSDX firmware it grew from. It emulates a Kenwood TS-480, but the subset is thin: the dial, the mode, PTT and RIT/XIT clear are all it answers. No S-meter, no SWR, no power control, no VFO B, no split and no CAT keyer, so the Drive slider and the meters do nothing over CAT — the audio level is the only transmit control there is.

    CW is keyed as audio (MCW) through the sideband, or with the key or paddle at the radio; there is no command that takes text.
settings-radio-audio = Audio
settings-radio-tr-usdx-has-no-sound =
    A (tr)uSDX has no sound card of its own and two ways to be heard, and which is right is a fact about your shack rather than the radio.

    "One cable" takes the audio inside the CAT serial link itself — the radio's own 8-bit stream, over the same USB cable as the control, with nothing else to plug in. The catch is the firmware: it cannot take a CAT command while its stream is running (a command sent into the stream does not pause it, it kills it), so this mode never polls. Tuning and mode changes are sent as you make them, each pausing the stream for a moment and starting it again, so expect a brief gap on each change; nothing is sent while you are simply listening. The radio's own dial and mode are not followed either — there is no read to follow them with — so drive it from here.

    "USB sound card" takes the audio from an external sound card wired to the radio's 3.5 mm speaker/mic jack, and behaves as any other CAT rig: the dial poll runs, so the radio's own knob and mode are followed. Pick the card under Radio audio below.

    Both modes open the port with DTR held high: the serial adapter's DTR is wired to the radio's reset, so opening the port reboots it and the first second or two is quiet while it comes up. DTR is never used to key.
settings-radio-radio-model = Radio model
settings-radio-which-icom-two-things-ci =
    Which Icom, for the two things CI-V does not do the same way on all of them.

    The transceiver address below is filled in from it: every model ships with a different one, and a frame sent to the wrong address is simply ignored — a radio that answers nothing, with no error anywhere to say why.

    It also decides whether sdroxide can select DATA mode. On CI-V, USB and USB-DATA are the same mode byte and a separate command tells them apart; without it a digital-mode over goes out through the microphone input, with the rig's speech processing and SSB filter in the path.

    "Other" leaves the address to you and sends no DATA-mode command at all.
settings-radio-radio-id-hex = Radio ID (hex)
settings-radio-show-radio-s-spectrum-scope = Show the radio's spectrum scope
settings-radio-scope-span = Scope span
settings-radio-how-wide-sweep-radio-keeps = How wide to sweep it. The radio keeps whatever span was last chosen on its own screen — often a few kHz. Setting a span here also puts the scope into centre mode, so it follows the dial. It changes the radio's own display too; "As set on the radio" leaves it alone.
settings-radio-press-apply-reconnect-switch-without = Press "Apply / reconnect" to switch without a restart.
settings-radio-radio-audio-sound-card = Radio audio (sound card)
settings-radio-waiting-sound-cards-machine-radio = Waiting for the sound cards on the machine the radio is plugged into.
settings-radio-radio-rx = From radio (RX)
settings-radio-radio-tx = To radio (TX)
settings-radio-apply-reconnect = Apply / reconnect
settings-radio-reopen-cat-rig-these-sound = Reopen the CAT rig with these sound cards — no restart
settings-radio-reconnects-radio-without-restarting = Reconnects the radio without restarting.
settings-radio-receive-radio-pc = Receive (radio → PC)
settings-radio-sound-card-radio-s-own = The sound card the radio's own audio comes in on — its headphone socket into the computer's line or mic input. Everything on the panadapter and in the decoders arrives here.
settings-radio-transmit-pc-radio-mic = Transmit (PC → radio mic)
settings-radio-sound-card-carries-audio-radio = The sound card that carries the audio the radio must broadcast — the computer's output into the radio's mic socket. The radio keys itself, by VOX, the moment audio arrives here, so this device is silent unless the radio is being transmitted through.
settings-radio-radio-has-no-control-cable = This radio has no control cable — it keys itself off VOX, so there is no PTT to command. Transmit is audio into the radio's microphone.
settings-radio-reopen-radio-these-sound-cards = Reopen the radio with these sound cards — no restart
settings-radio-devices = Devices
settings-radio-discover = Discover
settings-radio-no-devices-press-discover = no devices — press Discover
settings-radio-manual-ip = Manual IP
settings-radio-optional-e-g-192-168 = optional, e.g. 192.168.1.50
settings-radio-sample-rate = Sample rate
settings-radio-receiver-ddc = Receiver (DDC)
settings-radio-protocol-2-board-carries-several = A Protocol 2 board carries several independently tunable receivers (DDCs) on one connection — run this radio on DDC1 and another radio, same address, on DDC2. The transmitter belongs to the DDC1 radio. Protocol 1 boards have DDC1 only.
settings-radio-lna-gain = LNA gain
settings-radio-front-end-gain-hermes-lite = Front-end gain of a Hermes-Lite 2. Takes effect immediately — no reconnect — and is remembered as the level the radio starts at. Too high clips the ADC and the whole band looks distorted; too low and the receiver goes deaf.
settings-radio-overload-protection = Overload protection
settings-radio-back-lna-gain-off-itself =
    Back the LNA gain off by itself while the board reports its ADC overflowing, and let it back up once it stops.

    A Hermes-Lite 2 samples the whole of 0-38 MHz onto one 12-bit converter with no mixer and no preselector in front of it, so a broadcast station a band away can drive it into overflow while the band you are looking at shows nothing wrong at all — the noise floor climbs, everything intermodulates and the decoders stop. The board knows, and this is what acts on it.

    Off by default: it moves a control you set. Nothing happens while transmitting.
settings-radio-wind-gain-back-when-adc = Wind the gain back when the ADC overflows
settings-radio-applies-apply-reconnect-main-window = Applies on Apply / reconnect. The main window's Gain rail reads the gain the board is actually running, so it follows the loop and you can watch what it does; the slider above is the level the radio starts at, and the loop does not rewrite it.
settings-radio-step = {"  "}Step
settings-radio-how-far-gain-moves-each = How far the gain moves each time, in dB. One is the step the board's own gain register has.
settings-radio-attack-decay = {"  "}Attack / decay
settings-radio-how-often-gain-may-come =
    How often the gain may come down while the converter is overflowing, and how often it may go back up once it has stopped.

    The two are deliberately a hundred times apart. Retreat immediately: every millisecond of overflow is a receiver full of intermodulation. Return slowly: whatever caused it — a neighbour keying, a broadcaster coming up at dusk — has usually not gone away, and a loop that recovered as fast as it retreated would spend the evening oscillating across the threshold. 100 ms and 10 s per decibel is what PowerSDR's Auto S-Att and N1GP's HermesIntf have both used.
settings-radio-range = {"  "}Range
settings-radio-lowest-highest-gain-loop-may = The lowest and highest gain the loop may use, in dB. The ceiling is what stops it deciding how sensitive your receiver should be; the floor is where you say that below some point the overload is somebody else's problem and the answer is a filter, not another twenty decibels.
settings-radio-filter-board = Filter board
settings-radio-accessory-board-hermes-lite-2 = Accessory board on the Hermes-Lite 2's J16 header (or the open-collector outputs of any other openHPSDR board — Protocol 2 included). "N2ADR" picks one relay per band; "Alex / Hermes band code" puts the band number on outputs 1-4, which is what an ANAN's Alex board, a Zeus SDR, a HiQSDR and Quisk expect. "Custom" opens a table below where you state the seven outputs yourself, band by band, on receive and on transmit — for an antenna switch, an amplifier's band decoder or anything else neither preset fits; either preset can be poured into it as a starting point. Leave this at "None" unless something really is fitted: those seven pins are general-purpose open-collector outputs, and operators also wire them to amplifier PTT, antenna relays and transverter switching. Driving them from band data would start operating whatever is connected. Applies on Apply / reconnect.
settings-radio-io-board-rx-input = IO board RX input
settings-radio-where-n2adr-hl2ioboard-takes-receive = Where an N2ADR HL2IOBoard takes its receive signal from. The board itself is found automatically and needs no setting; this one exists only for operators who have wired its own SMA jacks. Leave it at "Radio's own input" otherwise — selecting the IO board's J9 with nothing connected to it leaves the receiver deaf. Applies on Apply / reconnect. Set to either J9 choice, the inputs also appear on the ANT control and are remembered per band.
settings-radio-power-amplifier = Power amplifier
settings-radio-use-hermes-lite-2-s = Use the Hermes-Lite 2's onboard PA
settings-radio-default-what-you-want-unless = On by default, and what you want unless an external amplifier is driven from the board's low-power RF1 output. With it off the radio still keys — the T/R relay throws and any accessory board follows — but the antenna jack makes no power at all, and the relay is deliberately held in receive. Ignored on boards other than a Hermes-Lite.
settings-radio-mirror-board-s-spectrum-about = Mirror the board's spectrum about the tuned frequency, on transmit as well as receive. On by default: a Hermes-Lite 2 needs it. Turn it off only if signals show up on the wrong side of the dial and nothing decodes — the giveaway is a waterfall full of convincing traces while SSB lands on the wrong sideband and FT8 returns no decodes at all.
settings-radio-frequency-correction = Frequency correction
settings-radio-crystal-tcxo-error-ppm-applied = Crystal/TCXO error in ppm, applied to RX and TX. Applies immediately.
settings-radio-ppm = {" "}ppm
settings-radio-transmit-buffer = Transmit buffer
settings-radio-how-far-ahead-real-time = How far ahead of real time transmit audio is fed to the board over the network, before sdroxide slows down to match it. The board itself holds no such buffer — this only widens sdroxide's own margin. Raise it on a WiFi link or a VPN, where the low default (right for a direct wired connection) is not enough headroom against jitter and the transmitted audio or PTT stutters; higher costs transmit latency. Takes effect on APPLY, which reconnects to the board.
settings-radio-puresignal = PureSignal
settings-radio-adaptive-predistortion-linearise-transmitter-sample = Adaptive predistortion: linearise the transmitter from a sample of what it actually emitted, for twenty-odd decibels less intermodulation at the same power. The receiver is the feedback path — the board keeps receiving through an over — so a directional coupler and an attenuator have to put a sample of the amplifier's output into an input the T/R switch does not take away on transmit. On a Hermes-Lite 2 that means the IO board's PureSignal jack, and the receive input above set to match; on a board without one, an ANAN's RX port or an external relay that switches the coupler in for the length of the over. It is this receiver that is read — a board with a second ADC cannot use RX2 as the feedback tap, however its own firmware routes it. With nothing coupled in, the loop never locks and the transmitter is left exactly as it would have been. Applies on Apply / reconnect, and only on the radio that owns the transmitter (DDC1).
settings-radio-correct-transmitter-receiver-s-own = Correct the transmitter from the receiver's own feedback
settings-radio-table-steps = {"  "}Table steps
settings-radio-how-finely-amplifier-s-curve = How finely the amplifier's curve is modelled. More steps follow a sharper knee and take longer to fill in; 32 is a sensible start.
settings-radio-adaptation = {"  "}Adaptation
settings-radio-how-fast-correction-follows-what = How fast the correction follows what the coupler reports. Slow is right — this is averaging an amplifier's curve, which does not move, out of a feedback path that has noise in it.
settings-radio-hold = Hold
settings-radio-stop-adapting-keep-correction-where = Stop adapting and keep the correction where it is — for measuring, and for an operator happy with what it has learned.
settings-radio-manual-ip-overrides-discovery-press = A manual IP overrides discovery. Press "Apply / reconnect" to switch without a restart.
settings-radio-open-collector-outputs-band = Open-collector outputs by band
settings-radio-one-control-word-per-band = One control word per band, as your hardware's documentation states it: bit 0 is output 1, bit 6 is output 7. RX is asserted while receiving on that band and TX while the transmitter is keyed — give them the same value for a filter, and different ones for anything that belongs on one side of the changeover only (an amplifier's key line, a receive preamplifier's bypass). Bands left at 00 assert nothing. Applies on Apply / reconnect.
settings-radio-alex-band-code = ALEX BAND CODE
settings-radio-replace-table-what-preset-would = Replace the table with what this preset would send on every band, as a starting point to edit. Nothing else in the configuration changes.
settings-radio-clear = CLEAR
settings-radio-every-output-off-every-band = Every output off on every band, which is what "None" does.
settings-radio-band = Band
settings-radio-rx = RX
settings-radio-tx = TX
settings-radio-outputs-asserted = Outputs asserted
settings-radio-other = Other
settings-radio-everywhere-outside-amateur-bands-short = Everywhere outside the amateur bands — short-wave listening, and anything a transverter's dial lands on that no band covers.
settings-radio-dongle = Dongle
settings-radio-rescan = Rescan
settings-radio-re-list-usb-bus-no = Re-list the USB bus. No device is opened, so this is safe to press while receiving.
settings-radio-no-dongles-press-rescan = no dongles — press Rescan
settings-radio-first-one-found = — first one found —
settings-radio-rtl2832u-s-resampler-reaches-225 = The RTL2832U's resampler reaches 225–300 kHz and 900 kHz–3.2 MHz, nothing between. Takes effect on Apply.
settings-radio-often-drops-samples = {"  "}(often drops samples)
settings-radio-agc = AGC
settings-radio-manual-setting-measurement-weak-signal = Manual is the setting for measurement and weak-signal digital modes. The tuner and the demodulator have independent automatic loops.
settings-radio-tuner-gain = Tuner gain
settings-radio-applies-immediately-no-reconnect-tuner = Applies immediately — no reconnect. The tuner has 29 discrete steps, so the value snaps to the nearest one it can produce. Ignored while the tuner AGC is running.
settings-radio-crystal-error-parts-per-million = Crystal error in parts per million. Run with RUST_LOG=sdroxide_rtlsdr=debug and the log prints the measured clock error after about 20 seconds — that is the number to enter. Applies immediately.
settings-radio-hf-reception = HF reception
settings-radio-tuner-itself-starts-24-mhz =
    The tuner itself starts at 24 MHz. An RTL-SDR Blog V4 upconverts below that in hardware; other dongles reach HF only by sampling the ADC directly, through the V3's HF port. Automatic hands everything below 24 MHz to the ADC and everything above it to the tuner.

    Direct sampling covers every HF band, 17 m and 15 m included — they arrive in the ADC's second Nyquist zone, the right way up. Nothing filters the ADC's input, though, so whatever is at 28.8 MHz minus the dial comes with them: 10.7 MHz under 17 m, 7.726 MHz under 15 m.

    Switching modes briefly interrupts the stream.
settings-radio-removes-dongle-s-own-dc =
    Removes the dongle's own DC spike from the centre of the span, and the mirror image every signal leaves reflected about it, by measuring the imbalance in the samples themselves — no calibration, and it applies immediately. The tuner has no offset-tuning mode, so this is the only way to clear the centre.

    An AM carrier tuned dead on the dial is at DC too, so it goes with the spike: tune a kilohertz off it, or switch this off.
settings-radio-remove-centre-spike-mirror-image = Remove the centre spike and mirror image
settings-radio-bias-tee = Bias tee
settings-radio-feed-4-5-v-dc = Feed ~4.5 V DC up the coax
settings-radio-bias-tee-never-connect-transceiver = Bias tee is ON. Never connect a transceiver, a grounded antenna, or a preamp powered from elsewhere while this is enabled — the DC goes straight down the feedline.
settings-radio-receive-only-dongle-sample-rate = Receive only. The dongle and sample rate take effect on Apply; everything else applies as you change it.
settings-radio-server-address = Server address
settings-radio-where-rtl-tcp-listening-address =
    Where rtl_tcp is listening: an address, or an address and port. The port defaults to 1234, which is rtl_tcp's own default.

    On the far end, start it as `rtl_tcp -a 0.0.0.0` — bound to 127.0.0.1, which is what it does with no -a, it only accepts connections from that same machine.

    Takes effect on Apply.
settings-radio-host-host-port-e-g = host or host:port, e.g. raspberrypi.local:1234
settings-radio-requested-server-same-resampler-limits =
    Requested of the server, with the same resampler limits as a local dongle — it is the same silicon on the far end.

    The figure beside each rate is what it costs on the link: the samples are sent uncompressed, and a rate the network cannot carry makes rtl_tcp drop the connection rather than degrade. Takes effect on Apply.
settings-radio-wired-link = {"  "}(wired link)
settings-radio-runs-server-s-dongle-manual = Runs on the server's dongle. Manual is the setting for measurement and weak-signal digital modes.
settings-radio-applies-immediately-no-reconnect-sent = Applies immediately — no reconnect. Sent in tenths of a dB and snapped by the server to a step its tuner has; the protocol has no replies, so what it settled on cannot be read back and this slider keeps showing what was asked for. Ignored while the tuner AGC is running.
settings-radio-crystal-error-server-s-dongle =
    Crystal error of the *server's* dongle, in parts per million — a property of that hardware, so it is set here and not on this machine's dongles. Applies immediately.

    The measured clock error the USB interface prints is not available here: over a network what that measurement sees is the buffering, not the crystal, and it is wrong by thousands of ppm. Calibrate the dongle on USB once and carry the number over, or tune a broadcast station of known frequency and adjust until it sits on the dial.
settings-radio-tuner-starts-24-mhz-below =
    The tuner starts at 24 MHz; below that the far end needs help. A Blog V4 upconverts by itself and Automatic leaves it alone — which is the only thing it can do, since the protocol reports the tuner chip and nothing else, and a V4 is indistinguishable from a plain R828D over the wire.

    On a V3 or any other dongle, Automatic switches the server to direct sampling below the tuner's own 24 MHz floor — which covers every HF band, 17 m and 15 m arriving in the ADC's second Nyquist zone with 28.8 MHz minus the dial folded on top. Choose Direct sampling explicitly for a plain R828D that hears nothing on HF. Switching briefly interrupts the stream.
settings-radio-removes-dongle-s-dc-spike =
    Removes the dongle's DC spike and mirror image here, from the samples as they arrive — they are artefacts of the hardware, so they travel over the network with everything else. Applies immediately.

    An AM carrier tuned dead on the dial sits at DC too, so it goes with the spike: tune a kilohertz off it, or switch this off.
settings-radio-powers-preamp-far-end-s = Powers a preamp from the far end's dongle. Older servers do not implement the command and ignore it silently — the protocol has no way to say no — so a bias tee that does not come on is not necessarily this end's doing.
settings-radio-feed-4-5-v-dc-2 = Feed ~4.5 V DC up the remote coax
settings-radio-bias-tee-hardware-somewhere-else = Bias tee is ON, on hardware that is somewhere else. Whatever is on the other end of that feedline — a transceiver, a grounded antenna, a preamp already powered — is not in front of you to check.
settings-radio-sdrplay-server-rsp-tcp = SDRplay server (rsp_tcp)
settings-radio-sdrplay-server-greets-exactly-like = An SDRplay server greets exactly like a dongle, so these are shown always rather than when one is detected. Against an ordinary rtl_tcp server they are simply ignored — the protocol has no replies and discards commands it does not know. The Device tab names the server as rsp_tcp once one has identified itself.
settings-radio-input = Input A
settings-radio-input-b = Input B
settings-radio-hi-z = Hi-Z
settings-radio-lna-state = LNA state
settings-radio-step-index-not-db-figure = A step index, not a dB figure: how much each step is worth depends on the RSP model and the band. 0 is the most gain.
settings-radio-if-gain-reduction = IF gain reduction
settings-radio-reduction-so-more-less-signal = A reduction, so more is less signal. Only obeyed with the RSP's AGC off.
settings-radio-enable = Enable
settings-radio-dbfs = {" "}dBfs
settings-radio-level-rsp-s-agc-aims = The level the RSP's AGC aims to hold.
settings-radio-notches = Notches
settings-radio-am = AM
settings-radio-fm = FM
settings-radio-dab = DAB
settings-radio-rf = RF
settings-radio-reference-out = Reference out
settings-radio-t-24-mhz-clock-out = 24 MHz clock out
settings-radio-rsp2-rspduo-only = RSP2 and RSPduo only.
settings-radio-receive-only-address-sample-rate = Receive only. The address and sample rate take effect on Apply; everything else applies as you change it. A dropped connection is retried on its own, so a server that is restarted comes back without touching anything here.
settings-radio-where-spyserver-listening-address-address =
    Where spyserver is listening: an address, or an address and port. The port defaults to 5555, which is spyserver's own default.

    On the far end, check that its config file binds an address other machines can reach — bound to 127.0.0.1 it only accepts connections from that same machine.

    Takes effect on Apply.
settings-radio-host-host-port-e-g-2 = host or host:port, e.g. raspberrypi.local:5555
settings-radio-i-q-bandwidth = I/Q bandwidth
settings-radio-how-wide-slice-band-arrives =
    How wide a slice of the band arrives as I/Q — and so how much of the link this uses. This is the *demodulated* window: it follows the dial, and everything wider than it is the server's FFT in the strip above the panadapter.

    Automatic aims at about 96 kHz, which carries every mode here including wide FM and still fits down a cellular uplink. Takes effect on Apply.
settings-radio-which-stage-server-s-own =
    Which stage of the server's own rate ladder to ask for. Every receiver has a different ladder — it is its maximum rate halved stage by stage — so this is stored as the stage rather than as a figure in hertz, and the same setting still means something sensible pointed at a different server.

    Automatic aims at about 1 Msps. Press Test connection to see what the stages come to on this server. Takes effect on Apply.
settings-radio-automatic = Automatic
settings-radio-sample-format = Sample format
settings-radio-bits-per-component-wire-so =
    Bits per component on the wire, and so what this costs on the link: 16-bit is twice 8-bit, and 32-bit float is four times it for no more information than the receiver's ADC had.

    8-bit is what makes a remote receiver work over a domestic uplink and is right for almost everything. A server may override this — some are configured to insist on one format — and says so in the log when it does. Takes effect on Apply.
settings-radio-gain = Gain
settings-radio-server-s-gain-stage-index =
    The server's gain stage, as an index — not a number of decibels. What each index means belongs to the receiver on the far end and changes with the band, and the protocol never says, so nothing here can turn it into dB without inventing a figure.

    The real range is the server's and is only known once connected; an index past it is clamped. Applies immediately — no reconnect.
settings-radio-index = index{" "}
settings-radio-digital-gain = Digital gain
settings-radio-how-far-server-scales-samples =
    How far the server scales its samples up before quantising them for the wire. Automatic watches what actually arrives and holds the peak at half of full scale, which is the only thing that can be right on both a dead band and a crowded one — the two are thirty decibels apart and no fixed figure serves both.

    It matters only at 8 bits, where a signal far below full scale loses its lower bits to the quantiser and one above it is clipped flat. Wider formats have room to spare and get the figure every other client computes, from the gain index and the decimation stage. Applies immediately.
settings-radio-full-band-strip = Full-band strip
settings-radio-server-s-own-fft-whole =
    The server's own FFT of the whole band, drawn in the strip above the panadapter. In this interface it is the only band view there is — the panadapter itself is only as wide as the I/Q being received.

    Switching it off leaves a receiver with no way to see anything it is not already tuned to. Applies immediately, with a short break in the audio while the server changes what it is sending.
settings-radio-ask-server-low-rate-fft =
    Ask the server for a low-rate FFT of the whole band as well as the I/Q, and draw it in the strip above the panadapter.

    It costs almost nothing — a couple of kilobytes a frame, a dozen or so times a second — and it shows the whole receiver rather than the slice being demodulated. Worth switching off only on a link where every byte counts. Applies immediately, with a short break in the audio.
settings-radio-show = Show
settings-radio-whole-band = whole band
settings-radio-how-much-receiver-strip-covers = How much of the receiver the strip covers. The whole band is the widest view there is; narrowing it puts the same number of bins across less spectrum, which is finer detail over a smaller stretch.
settings-radio-strip-db-window = Strip dB window
settings-radio-server-quantises-fft-into-window =
    The server quantises its FFT into this window before sending it, one byte a bin — so these decide how finely the strip is measured, not just how it is drawn. The floor and ceiling the strip is *displayed* with are set by the engine's own auto-levelling and are a separate thing.

    The default 150 dB is the whole protocol range and needs no attention. Narrowing it around the noise floor buys resolution on a receiver whose signals all sit in a small part of the scale. Applies immediately.
settings-radio-top = top{" "}
settings-radio-range-2 = range{" "}
settings-radio-i-q-correction = I/Q correction
settings-radio-remove-dc-spike-mirror-image = Remove the DC spike and the mirror image in DSP, on this side. Whether the receiver on the far end needs it depends on what it is — an Airspy HF+ does not, an RTL-SDR does — and the protocol does not say which it is talking to. Applies immediately.
settings-radio-enabled = Enabled
settings-radio-test-connection = Test connection
settings-radio-connect-read-what-server-says = Connect, read what the server says about its receiver, and disconnect again — without starting a stream, so it is safe to press against a server somebody else is using.
settings-radio-receive-only-panadapter-narrow-i = Receive only. The panadapter is the narrow I/Q window, which follows the dial; the band view is the server's FFT in the strip above it. Press "Apply / reconnect" to switch without a restart.
settings-radio-receive-only-wideband-i-q = Receive only. Wideband I/Q, the same as any local SDR. Press "Apply / reconnect" to switch without a restart.
settings-radio-receiver-address = Receiver address
settings-radio-where-receiver-serves-web-page =
    Where the receiver serves its web page: an address, or an address and port. The port defaults to 8073, which is a KiwiSDR's own.

    Note that a receiver reached through the project's proxy (something.proxy.kiwisdr.com — nearly half the public ones) answers on port 80 instead, so give it explicitly. Picking a receiver from "Public SDRs" fills this in correctly.

    Takes effect on Apply.
settings-radio-host-host-port-e-g-3 = host or host:port, e.g. kiwi.example.org:8073
settings-radio-password = Password
settings-radio-receiver-s-user-password-where =
    The receiver's *user* password, where its operator has set one. Blank on almost every public receiver. Not the admin password.

    Takes effect on Apply.
settings-radio-announce = Announce as
settings-radio-name-connection-shows-up-under =
    The name this connection shows up under, which the receiver's owner and everybody else listening to it can see.

    Left blank it is your station callsign, which is the network's own convention — some operators do block clients that will not identify. Takes effect on Apply.
settings-radio-blank-station-callsign = blank = station callsign
settings-radio-band-view = Band view
settings-radio-ask-receiver-own-waterfall-well =
    Ask the receiver for its own waterfall as well as its I/Q, and draw it in the strip above the panadapter.

    Worth having: the I/Q is only about 12 kHz wide, so without this there is nothing to tune *by*. It is a second connection carrying roughly 20 kB/s at full speed, against the I/Q's 44 kB/s.

    Takes effect on Apply.
settings-radio-show-receiver-s-0-30 = show the receiver's 0-30 MHz waterfall
settings-radio-band-view-speed = Band view speed
settings-radio-how-often-receiver-sends-waterfall =
    How often the receiver sends a waterfall row, 1 (slowest) to 4. The only setting here that changes what the link costs while running, so turn it down on a metered connection.

    Applies immediately.
settings-radio-band-view-span = Band view span
settings-radio-how-much-band-receiver-s =
    How much band the receiver's waterfall covers. Its full 0-30 MHz at the left of the slider, halving with each step to the right.

    The waterfall is always 1024 bins wide however wide the window is, so this is really a resolution control: the whole band is 29 kHz to a bin, which is a band *map* rather than a picture of anything, while a few hundred kilohertz shows individual stations in a broadcast band. Zoomed in, the window follows your dial.

    The whole band is the default and is what makes the strip a thing to tune by; the panadapter below it is the ~12 kHz of I/Q. Applies immediately.
settings-radio-receiver-agc = Receiver AGC
settings-radio-receiver-s-agc-which-far =
    The *receiver's* AGC, which is on the far side of the link and ahead of the I/Q — so it acts before anything sdroxide does.

    On by default, unlike every local SDR here, because the measured alternative was worse: on a live receiver the manual gain was not monotonic across its range and its top end clipped the I/Q at full scale, while the AGC held about -24 dBFS with 7 dB of headroom.

    It does mean the sample amplitude is not a signal level, which is why the S-meter is read from the receiver's own figure instead. Applies immediately.
settings-radio-let-receiver-ride-own-gain = let the receiver ride its own gain
settings-radio-manual-gain = Manual gain
settings-radio-fixed-gain-when-receiver-s = Fixed gain when the receiver's AGC is off, on its own 0-90 scale. Not decibels of anything stated — the protocol calls it manGain and says no more, and it was measured not to be monotonic, so treat it as a dial to find a good spot on rather than a calibration. Applies immediately.
settings-radio-read-receiver-s-status-page = Read the receiver's status page: what it is, what it covers, how many channels are free, and whether its operator allows connections from apps other than a browser. Takes none of its channels.
settings-radio-host-port-e-g-127 = host:port, e.g. 127.0.0.1:50001
settings-radio-iq-sample-rate = IQ sample rate
settings-radio-receiver = Receiver
settings-radio-rig-two-receivers-sunsdr2dx-can = A rig with two receivers (SunSDR2DX) can serve two radio tabs from one connection — run this radio on RX1 and another on RX2. The transmitter belongs to the RX1 radio.
settings-radio-stream-delay = Stream delay
settings-radio-how-long-rig-s-iq = How long the rig's IQ takes to arrive on a new centre after sdroxide moves it. It is the rig's own DSP pipeline, not the network — a server on this machine has one too — and while the panadapter is dragged fully zoomed out, being wrong by this much moves the newest waterfall rows sideways of the history by the error times the speed of the drag. Too high displaces them exactly as far as too low, the other way. Measure it with `cargo run --release -p sdroxide-tci --example retune_latency`; the default is a SunSDR2DX on ExpertSDR3 at 192 kHz.
settings-radio-wideband-iq-receive-audio-transmit = Wideband IQ receive, audio transmit. Press "Apply / reconnect" to switch without a restart.
settings-radio-radio-address = Radio address
settings-radio-address-shown-radio-under-set = The address shown on the radio under SET > Network. Network Control has to be on there, and the radio needs a network user name and password set.
settings-radio-host-ip-e-g-192 = host or IP, e.g. 192.168.1.50
settings-radio-control-port = Control port
settings-radio-t-50001-unless-has-been-changed = 50001 unless it has been changed on the radio.
settings-radio-network-user = Network user
settings-radio-stored-clear-radio-json-protocol = Stored in the clear in radio.json. The protocol obfuscates it reversibly on the wire, so nothing here would make it a secret.
settings-radio-receive = Receive from
settings-radio-af-radio-demodulates-sdroxide-shows = AF: the radio demodulates and sdroxide shows the audio band. 12 kHz IF: the radio sends its DRM intermediate frequency instead and sdroxide demodulates, which brings its own filters, noise reduction and decoders to bear over about ±12 kHz. Either way the wide waterfall is the radio's own scope — no Icom outputs I/Q. An IF preference temporarily uses AF for wider modes such as WFM, then restores IF for compatible modes. An AF preference stays AF. HD Radio is unavailable on this connection.
settings-radio-audio-sample-rate = Audio sample rate
settings-radio-t-12-khz-if-needs-48000 = A 12 kHz IF needs the 48000 Hz stream — at this rate the radio's demodulated audio is used instead.
settings-radio-if-spectrum = IF spectrum
settings-radio-which-way-round-radio-s = Which way round the radio's 12 kHz IF runs. "Automatic" uses what the model is known to do — mirrored on an IC-7760, normal on every other Icom. Set it by hand if SSB comes out on the wrong sideband: the giveaway is having to select USB where the band runs LSB while the radio's own mode display agrees with sdroxide throughout.
settings-radio-normal = Normal
settings-radio-mirrored = Mirrored
settings-radio-displayed-bandwidth = Displayed bandwidth
settings-radio-width-audio-band-panadapter-cat = Width of the audio-band panadapter, as for a CAT rig. Used in the digital modes and where the radio's scope is off; otherwise the panadapter is the scope, and Scope span sets its width.
settings-radio-how-cw-panel-s-keyer-2 = How the CW panel's keyer transmits. "Rig keyer" puts the radio in CW and hands it the text to send with its own keyer (CI-V 0x17). "Sound card" sends the keyed sidetone over the LAN as audio instead (MCW), a tone at dial + pitch — and because a rig in CW would ignore its modulator input, selecting CW then keeps the radio in plain USB, the same mode the digital modes ride.
settings-radio-how-much-audio-radio-holds = How much audio the radio holds before modulating. More survives a worse network, at the cost of transmit latency.
settings-radio-streams-radio-s-own-475 = Streams the radio's own 475-bin sweep. On AF it is the panadapter — the audio the radio sends is what came through its filter, not a picture of the band — and on the 12 kHz IF it is the full-band waterfall above one. Either way it is the radio's picture, not sdroxide's DSP: there is no I/Q to compute one from.
settings-radio-how-wide-sweep-only-wide = How wide to sweep it. This is the only wide view an Icom has, and the radio keeps whatever span was last chosen on its own screen — often a few kHz, which is why the strip can come up no wider than the panadapter under it. Setting a span here also puts the scope into centre mode, so it follows the dial. It changes the radio's own display too; "As set on the radio" leaves it alone.
settings-radio-switch-modulation-input-lan = Switch modulation input to LAN
settings-radio-transmit-audio-only-heard-when = Transmit audio is only heard when the radio's MOD input is set to LAN. sdroxide can write that on a model whose menu numbering it knows; on any other it says so and leaves the menu alone. It is a loan: whatever the radio held there is read first and put back when the session ends, so a rig used on its own afterwards still hears its own microphone.
settings-radio-copy-diagnostic-report = Copy diagnostic report
settings-radio-copies-radio-s-last-session = Copies this radio's last session — its handshake and CI-V trace — to the clipboard, for a bug report. This radio's, not the station's: with two Icoms on the LAN each tab answers about its own address.
settings-radio-control-audio-radio-s-own = Control, audio and the radio's own scope over one network connection — no serial cable and no sound card. Press "Apply / reconnect" to switch without a restart.
settings-radio-testing = Testing…
settings-radio-was-only-check-press-apply = That was only a check — press Apply / reconnect below to start using this radio.
settings-radio-radios = Radios
settings-radio-asks-network-iio-devices-also = Asks the network for IIO devices, and also tries 192.168.2.1 directly — a Pluto on the end of a USB cable is often unreachable by multicast even though the address works.
settings-radio-no-radios-press-discover = no radios — press Discover
settings-radio-address = Address
settings-radio-overrides-selection-above-usb-cable = Overrides the selection above. The USB cable presents the Pluto as a network adapter, not a serial port, so this is an IP address even when the radio is on your desk.
settings-radio-pluto-revision-c-pluto-unlocked = A Pluto+ or a revision-C Pluto unlocked to 2R2T can serve two radio tabs from one box — this radio on RX1 and another on RX2, each on its own antenna. The two chains share the one oscillator, so retuning either radio moves both; what RX2 buys is a second antenna on the same spectrum (diversity), not a second band. The transmitter belongs to the RX1 radio. A stock 1R1T Pluto refuses RX2 when it connects.
settings-radio-width-spectrum-sdroxide-receives-ad9361 = Width of the spectrum sdroxide receives. The AD9361 reaches 61.44 Msps; the USB network link does not, which is what this list is scaled to. The lowest rates need a filter configuration loaded into the AD9361, which sdroxide does not do — a stock Pluto runs them at about 2.084 Msps instead and says so when it connects. Takes effect on Apply.
settings-radio-more-than-usb-2-will = {"  "}(more than USB 2 will carry)
settings-radio-stock-pluto-runs-2-084 = {"  "}(a stock Pluto runs at 2.084)
settings-radio-analog-filter = Analog filter
settings-radio-ad9361-s-baseband-filter-leave = The AD9361's baseband filter. Leave at 0 for automatic, which opens it to nine tenths of the sample rate — wide on purpose, because the receiver parks its oscillator a quarter of a span off the dial to keep signals clear of the DC spike, and a narrow filter would cut off exactly the part it moved them to. Takes effect on Apply.
settings-radio-ad9361-has-four-modes-not = The AD9361 has four modes, not an on/off switch. Slow attack suits SSB and CW; fast attack suits bursty signals; manual is the setting for measurement and weak-signal digital modes. Applies immediately.
settings-radio-rx-gain = RX gain
settings-radio-applies-immediately-no-reconnect-ignored = Applies immediately — no reconnect. Ignored unless the AGC is set to manual, which is the AD9361's own behaviour, not sdroxide's.
settings-radio-tx-gain = TX gain
settings-radio-negative-because-ad9361-expresses-transmit = Negative because the AD9361 expresses transmit level as attenuation: 0 dB is full output. Applies immediately. The transmitter is set to its quietest before this value is applied on connect, so nothing the previous program left behind can be live.
settings-radio-reference-error-parts-per-million = Reference error in parts per million. Run with RUST_LOG=sdroxide_pluto=debug and the log prints the measured clock error after about 20 seconds — that is the number to enter. Applied by sdroxide, not written to the radio's own persistent trim. Applies immediately.
settings-radio-full-duplex = Full duplex
settings-radio-keep-receiving-while-transmitting = Keep receiving while transmitting
settings-radio-leave-off-pluto-reached-over =
    Leave this off on a Pluto reached over its USB cable. That link cannot carry a megasample per second in both directions at once, and an over that starves the transmit buffer goes out chopped — so by default receive stops for the length of an over.

    Turn it on for a board on real Ethernet (a LibreSDR, a Pluto on a gigabit adapter), where there is room for both: you then hear the receiver through your own transmission, which is how a QO-100 station listens to its own downlink.

    An over needs twice the sample rate's worth of link — 2.5 Msps is 10 MB/s each way — so lower the rate if the log starts saying the link is not carrying it. The panadapter still shows the transmitted signal during an over; it is the audio that keeps coming.

    Takes effect on Apply.
settings-radio-duplex = Duplex
settings-radio-whether-ad9361-runs-both-directions =
    Whether the AD9361 runs both directions at once (FDD, which is how a Pluto boots and what sdroxide has always left it in) or one at a time (TDD).

    Leave this on FDD unless you want the PTT pins below — TDD is what those key from, and it rules out Full duplex above. Takes effect on Apply.
settings-radio-ptt-pins = PTT pins
settings-radio-pluto-s-four-gpo-test =
    The Pluto's four GPO test points can key an external power amplifier, LNA or transmit-receive switch by themselves. Pick a pair and one pin is high the whole time the radio receives, the other the whole time it transmits — no host software in the loop and no serial PTT line to wire.

    This puts the radio in TDD whatever the Duplex row says, because the pins follow the AD9361's enable lines and in FDD both of those are asserted the entire session. Full duplex goes off with it, so leave both at their defaults if you work satellites and need to hear your own downlink.

    Analog Devices' own note puts an external LNA on GPO0/GPO1, so use GPO2/GPO3 if your board is wired that way. The pins are about 1.3 V at a few milliamps: drive a transistor or an opto-isolator with them, never a relay coil.

    Takes effect on Apply.
settings-radio-buffer-size = Buffer size
settings-radio-how-much-device-holds-before =
    How much the device holds before each transfer, in complex samples. The default of 32768 is about 16 ms at 2 Msps: long enough that the per-transfer round trip is not the bottleneck, short enough that a retune is not visibly late.

    Halve it if the log reports the receive socket being replaced. A marginal link — a Pluto reached over its USB cable at a high sample rate is the usual one — stalls part-way through a transfer, and a smaller transfer is both less likely to be caught mid-flight and quicker to make good afterwards. Raise it to trade retune latency for fewer round trips.

    Takes effect on Apply.
settings-radio-samples = {" "}samples
settings-radio-rx-tx-port = RX / TX port
settings-radio-ad9361-s-rf-port-select = The AD9361's rf_port_select. A stock Pluto wires one of each, so leave these empty unless you have a board that does not. Takes effect on Apply.
settings-radio-opens-radio-reads-what-says = Opens the radio, reads what it says about itself, and reports the tuning range this particular board has. Does not start a stream.
settings-radio-copies-last-session-s-protocol = Copies the last session's protocol trace to the clipboard, for a bug report.
settings-radio-wideband-iq-receive-transmit-half = Wideband IQ receive and transmit. Half duplex — receive stops for the length of an over, because the USB network link will not carry both at once. No SoapySDR and no libiio needed.
settings-radio-not-yet-verified-against-real = Not yet verified against real hardware. If it misbehaves, please send the diagnostic report — it contains every command exchanged with the radio, and the first bytes of the sample stream.
settings-radio-flexradio-announces-itself-local-network = A FlexRadio announces itself on the local network about once a second. A radio reached through a router or a VPN never broadcasts to you — enter its address below instead.
settings-radio-another-gui-client-has-radio = Another GUI client has this radio and multiFLEX is disabled. Disconnect that client, or enable multiFLEX on the radio.
settings-radio-overrides-selection-above-use-radio = Overrides the selection above. Use this for a radio on another subnet, behind a VPN, or on a non-standard port.
settings-radio-width-spectrum-sdroxide-receives-192 = Width of the spectrum sdroxide receives. 192 kHz is the radio's maximum for a DAX IQ stream, and so the widest span this interface can show.
settings-radio-t-192-khz-widest-dax-iq = 192 kHz is the widest DAX IQ stream a FLEX will send, and the radio ties the panadapter's span to it — so that is the whole span, and no setting here widens it.
settings-radio-dax-iq-channel = DAX IQ channel
settings-radio-radio-has-four-change-only = The radio has four. Change this only if something else on the network is already using channel 1 — the radio refuses a channel twice over.
settings-radio-station-name = Station name
settings-radio-shown-against-session-radio-s = Shown against this session in the radio's client list. The radio also remembers a client by it, so renaming makes the radio treat sdroxide as a new one.
settings-radio-gui-client-id = GUI client ID
settings-radio-identity-radio-restores-session-s = The identity the radio restores this session's slices and panadapters to. Left empty it is derived from the station name — stable, but the same on every sdroxide that kept the default name, and a radio settles a duplicate by throwing the earlier client off. sdroxide spots that on the wire and falls back to a one-session identity, which costs the restore; put a UUID of your own here to keep it.
settings-radio-optional-e-g-uuid-your = optional, e.g. a UUID of your own
settings-radio-mirror-radio-s-i-q =
    Mirror the radio's I/Q about the centre of the panadapter. Off by default, which is how a FLEX-6600 was verified.

    Try it if receive audio is unintelligible on USB *and* on LSB and nothing decodes, while the waterfall looks entirely convincing — that is what a mirrored stream looks like, and it is the one fault with no other symptom (issue #368). If it is not that, turning this on makes it obvious rather than subtle. Applies on Apply / reconnect.
settings-radio-network-mtu = Network MTU
settings-radio-largest-datagram-radio-may-send = Largest datagram the radio may send. 1450 is what SmartSDR itself asks for. Lower it on a path with a smaller MTU — a VPN or a tunnel — where the fragments are dropped and no spectrum arrives at all.
settings-radio-b = {" "}B
settings-radio-checks-radio-answers-without-registering = Checks the radio answers, without registering as a GUI client — so it will not disturb a SmartSDR session already running.
settings-radio-wideband-iq-receive-over-dax = Wideband IQ receive over DAX, audio transmit the radio modulates. Press "Apply / reconnect" to switch without a restart.
settings-radio-not-yet-verified-against-real-2 = Not yet verified against real hardware. If it misbehaves, please send the diagnostic report — it contains every protocol line exchanged with the radio.
settings-radio-devices-soapysdr-can-see = Devices SoapySDR can see
settings-radio-waiting-machine-radio-attached-where = Waiting for the machine the radio is attached to, where the modules are installed. Which device it opens is `device_args` in that machine's config.toml, or its --device.
settings-radio-ask-every-installed-soapysdr-module = Ask every installed SoapySDR module to scan. Nothing is opened, so this is safe while receiving — but it can take a moment.
settings-radio-not-enumerated-yet-press-rescan = Not enumerated yet — press Rescan.
settings-radio-no-soapysdr-devices-found-check = No SoapySDR devices found. Check that the module for your radio is installed and that you may access the device.
settings-radio-not-radio = not a radio
settings-radio-sound-card-listed-above-if = A sound card is listed above as if it were an SDR. It accepts any frequency and ignores it, so opening it shows the sound card's input instead of the band. sdroxide does not pick those automatically — but a device_args line naming one is still obeyed.
settings-radio-which-one-opens-set-device = Which one opens is set by --device or device_args in config.toml; with neither, the first radio listed wins.
settings-radio-no-device = no device
settings-radio-rig-has-no-software-adjustable = This rig has no software-adjustable gains.
settings-radio-rx-gains = RX gains
settings-radio-tx-gains = TX gains
settings-radio-antennas = Antennas
settings-radio-remembered-next-start = Remembered for the next start.
settings-radio-these-controls-appear-once-soapysdr = These controls appear once a SoapySDR device is open — they are the device's own, and it has to be asked.
settings-radio-stream = Stream
settings-radio-follow-app-setting = Follow the app setting
settings-radio-msps = or, in Msps
settings-radio-baseband-filter = Baseband filter
settings-radio-let-driver-choose = Let the driver choose
settings-radio-changing-either-reopens-radio = Changing either reopens the radio.
settings-radio-driver-publishes-no-settings-own = This driver publishes no settings of its own.
settings-radio-applied-once-remembered-next-start = Applied at once, and remembered for the next start.
settings-radio-no-rx-888-found-press = No RX-888 found — press Rescan
settings-radio-adc-clock = ADC clock
settings-radio-t-129-6-msps-needs-superspeed = 129.6 Msps needs a SuperSpeed link and a fast host; 64.8 is the safe default. Changing it reopens the receiver automatically, which takes a moment but needs no restart.
settings-radio-panadapter-width = Panadapter width
settings-radio-hf-only = {"  "}· HF only
settings-radio-usable-both-front-ends = Usable on both front ends.
settings-radio-below-vhf-crossover-ordinary-width = Below the VHF crossover this is an ordinary width. Above it the tuner's 8 MHz IF cannot be centred in a window this wide, so the live spectrum sits off to one side of the panadapter with nothing beside it.
settings-radio-dial-above-crossover-now-so = The dial is above the crossover now, so this width is showing the tuner's 8 MHz off-centre with dead spectrum beside it. Narrow the width to fill the panadapter.
settings-radio-vga-gain = VGA gain
settings-radio-ad8370-variable-gain-amplifier-ahead = AD8370 variable-gain amplifier ahead of the ADC.
settings-radio-attenuator = Attenuator
settings-radio-pe4304-step-attenuator-0-5 = PE4304 step attenuator, in 0.5 dB steps.
settings-radio-adc-range = ADC range
settings-radio-wide-2-25-vp-p = Wide (2.25 Vp-p)
settings-radio-selects-adc-s-wider-input = Selects the ADC's wider input range: more headroom for strong broadcast signals, fewer counts for weak ones. Off selects the more sensitive 1.5 Vp-p range.
settings-radio-dither = Dither
settings-radio-adds-small-dither-signal-ahead = Adds a small dither signal ahead of the ADC: costs a little noise floor, buys spurious-free dynamic range.
settings-radio-randomiser = Randomiser
settings-radio-adc-scrambles-output-so-digital = The ADC scrambles its output so the digital bus stops radiating into the front end; the driver unscrambles it. Leave this on unless you are debugging. Applies on reconnect.
settings-radio-dc-hf-antenna-port = DC on the HF antenna port
settings-radio-powers-active-antenna-preamp-down = Powers an active antenna or preamp down the coax.
settings-radio-vhf-tuner-gain = VHF tuner gain
settings-radio-r828d-rf-gain-used-above = R828D RF gain, used above the automatic HF/VHF crossover. 29 discrete steps; the nearest is used.
settings-radio-auto = Auto
settings-radio-let-tuner-run-own-lna = Let the tuner run its own LNA and mixer loops.
settings-radio-bias-tee-vhf = Bias tee (VHF)
settings-radio-dc-vhf-antenna-port = DC on the VHF antenna port
settings-radio-clock-trim = Clock trim
settings-radio-corrects-reference-oscillator-applied-when = Corrects the reference oscillator. Applied when you let go of the value — reopening on every pixel of a drag would restart the receiver hundreds of times.
settings-radio-receive-only-below-adc-s = Receive only. Below the ADC's Nyquist limit the antenna is sampled directly and retuning is instant, because there is no hardware downconverter — the full ADC stream is converted to baseband on the host. Above it the receiver switches to its R828D tuner and the VHF SMA automatically, so both antennas need to be connected. VHF needs an ADC clock of 32.4 Msps or more for the tuner's IF to fit, and at clocks below 48 Msps there is a gap between the two ranges that nothing can reach. Every setting here applies straight away — there is no Apply button to press.
settings-radio-device = Device
settings-radio-no-devices-press-rescan = no devices — press Rescan
settings-radio-six-rates-are-six-different =
    The six rates are six different FPGA images, not a register — which is why nothing in ELAD's protocol selects between them, and why what this setting means depends on the model.

    FDM-S1 / FDM-S2: a command. Their FPGA is loaded by the computer at every power-up, and sdroxide runs ELAD's own elad-firmware loader to put this rate's image in. Without that loader installed a sampler sends no samples at all, and sdroxide says so on screen.

    FDM-DUO: not a command. The radio boots its own FPGA and arrives at whatever rate it powered up in or FDM-SW2 last left it in (192 kHz on a fresh one), and this only says how the stream is READ. Get it wrong and the panadapter is simply the wrong width, with every frequency inside it scaled to match.

    The stream's real rate is measured a couple of seconds after it starts, and a mismatch is reported on screen. Takes effect on Apply.
settings-radio-fdm-s-fpga-image-loaded = FDM-S: the FPGA image loaded — FDM-DUO: how the stream is read
settings-radio-receiver-s-input-pad-one = The receiver's input pad — one step, in or out. The same control as the main window's Gain slider.
settings-radio-pre-selection-filters = Pre-selection filters
settings-radio-low-pass-bank-front-adc = The low-pass bank in front of the ADC. Bypassing it gives the widest view and the worst behaviour near strong out-of-band signals — a broadcast transmitter a few miles away will put images across the band. Leave it on unless you are deliberately listening outside the filtered range.
settings-radio-filters-circuit = Filters in circuit
settings-radio-rig-control-fdm-duo-only = Rig control — FDM-DUO only
settings-radio-transceiver-s-cat-usb-port = The transceiver's CAT USB port, which is a separate cable from the receive one. Leave the port unset on an FDM-S1 or FDM-S2, which have none — and on an FDM-DUO you would rather drive through its receive cable alone, where it can still be tuned and keyed but no meter can be read.
settings-radio-port-set-radio-s-display = With the port set, the radio's display and this dial are the same number in both directions: the FDM-DUO's receive window is built around its VFO, so tuning here moves the front panel and turning the front-panel knob moves the readout here. The panadapter re-centres on the dial as it goes. With no port — or none the radio answers on — there is no VFO to command, and tuning stays inside the window the radio is already sending.
settings-radio-none-usb-control-only = — none (USB control only) —
settings-radio-must-match-menu-70-cat = Must match menu 70 "CAT BAUD" on the radio, which ships at 38400. The radio has these four rates and no others, and a port opened at any other one is silent both ways — the radio ignores the dial and refuses to key, with nothing to say why.
settings-radio-how-transmit-keyed-cat-sends = How transmit is keyed. "CAT" sends the radio's own TX command and needs nothing set up on the rig; RTS needs menu 54 "PTT" set to PTT+RTS.
settings-radio-where-radio-takes-transmit-audio-2 =
    Where the radio takes its transmit audio from — the rig's TI command, which is menu 32 "TX IN" at the front panel. Asserted when the port opens.

    "USB audio" is what makes transmit work here: the FDM-DUO sends what sdroxide puts into its USB sound card. A radio left on "Microphone" sends the room instead, and nothing on screen says so.
settings-radio-which-two-sockets-back-receiver-2 =
    Which of the two sockets on the back the receiver listens on — the rig's AN command, which is menu 31 "ANTENNAS" at the front panel and the "ANT 1 2" indicator on its display.

    "RTX" is one antenna doing both jobs, on the M-type socket that also carries transmit. "RX only" moves receive to the second socket and leaves transmit on RTX — a receiving antenna, a loop or a beverage, with the beam still on the transmitter.

    It moves this whole receiver: the panadapter, the demodulators and the rig's own audio all come from the socket selected here.

    Applies immediately, and is read back from the radio when the control port opens — this is the rig's own setting, not a copy of it kept here.
settings-radio-how-often-radio-asked-what-2 = How often the radio is asked what it is doing — its dial, its mode and its meters. Turn the rig's own VFO knob and the readout follows within one poll. It is also the whole of the control traffic this end generates, so lower it if the radio's audio breaks up.
settings-radio-transmit-audio = Transmit audio
settings-radio-system-default = — system default —
settings-radio-fdm-duo-s-own-usb = The FDM-DUO's own USB Audio port. Left on the system default it is almost never the radio — it is the machine's speakers.
settings-radio-press-rescan-list-sound-cards = press Rescan to list the sound cards
settings-radio-copies-last-session-s-trace = Copies the last session's trace to the clipboard, for a bug report: every command exchanged with the device, and the first bytes of the sample stream.
settings-radio-only-receiving-fdm-duo-has = Only receiving on an FDM-DUO has been run on real hardware so far. If anything misbehaves, please attach the diagnostic report to a bug report.
settings-radio-cw-keyed-radio-s-own = CW is keyed by the radio's own key or paddle: the FDM-DUO has no command that accepts text, so the CW panel cannot key it.
settings-radio-press-apply-reopen-radio = Press Apply to reopen the radio.
settings-radio-no-receivers-press-rescan = no receivers — press Rescan
settings-radio-which-rates-receiver-has-depends = Which rates a receiver has depends on the model and the firmware. Takes effect on Apply.
settings-radio-every-rate-any-hf-model = every rate any HF+ model offers — connect one to see its own
settings-radio-receiver-s-own-gain-control = The receiver's own gain control. Leave it on for general listening; turn it off to set the attenuator by hand for measurement.
settings-radio-agc-threshold = AGC threshold
settings-radio-high = High
settings-radio-high-trades-little-sensitivity-headroom = High trades a little sensitivity for headroom against strong neighbours — the right setting on a crowded band at night.
settings-radio-front-end-attenuation-gain-0 = Front-end attenuation, as a gain — 0 dB is none. Only obeyed with the AGC off.
settings-radio-preamp = Preamp
settings-radio-hf-low-noise-amplifier-buys = The HF low-noise amplifier. Buys sensitivity at the cost of intermodulation, so it is off by default — which is usually right on a real antenna.
settings-radio-lna = LNA on
settings-radio-frequency-calibration = Frequency calibration
settings-radio-parts-per-billion-receiver-s = Parts per billion — this receiver's own unit, a thousand times finer than the ppm an RTL-SDR uses. Nothing here is ever written to the receiver's flash: this overrides the stored value for the session only.
settings-radio-use-receiver-s-stored-value = Use the receiver's stored value
settings-radio-ppb = {" "}ppb
settings-radio-feed-dc-up-coax = Feed DC up the coax
settings-radio-not-every-hf-has-one = Not every HF+ has one; on a receiver without, this does nothing.
settings-radio-host-dsp = Host DSP
settings-radio-image-balancer-zero-if-offset = The image balancer, the zero-IF offset and the fine-tuning oscillator. Turn it off only to see raw hardware output — with it off, the mirror image appears on the zero-IF rates and the dial is accurate only to the nearest kilohertz.
settings-radio-correct-image-fine-tune = Correct the image and fine-tune
settings-radio-copies-last-session-s-trace-2 = Copies the last session's trace to the clipboard, for a bug report: every command exchanged with the receiver, and the first bytes of the sample stream.
settings-radio-receive-only-no-soapysdr-no = Receive only. No SoapySDR and no libairspyhf needed. Below the synthesiser's floor the host oscillator does the tuning, which is how this receiver reaches VLF. The receiver and sample rate take effect on Apply; everything else applies as you change it.
settings-radio-not-yet-verified-against-real-3 = Not yet verified against real hardware. If it misbehaves, please send the diagnostic report — it contains every command exchanged with the receiver, and the first bytes of the sample stream decoded as I/Q pairs.
settings-radio-no-airspy-r2-mini-found = No Airspy R2 or Mini found — press Rescan
settings-radio-read-receiver = read from this receiver
settings-radio-r2-mini-offer-different-rates = an R2 and a Mini offer different rates and cannot be told apart until one is open
settings-radio-rate-you-get-receiver-s = This is the rate you get. The receiver's ADC runs at twice it — it digitises a real signal and sdroxide makes complex baseband from it on the host.
settings-radio-gain-curve = Gain curve
settings-radio-step-along-curve-above-not = A step along the curve above, not a dB figure — the tuner's LNA, mixer and VGA move together, and how much each step is worth depends on the curve and the band. 0 is the quiet end.
settings-radio-tuner-agc = Tuner AGC
settings-radio-lna-2 = LNA
settings-radio-hands-lna-tuner-s-own = Hands the LNA to the tuner's own loop, overriding the curve.
settings-radio-mixer = Mixer
settings-radio-same-mixer-stage = The same for the mixer stage.
settings-radio-loop-running-gain-slider-no = With a loop running, the gain slider no longer sets the stage it owns — the loop overwrites it.
settings-radio-dc-antenna-port = DC on the antenna port
settings-radio-t-12-bit-packing = 12-bit packing
settings-radio-third-less-usb-traffic-leave = A third less USB traffic. Leave it on: this is a USB 2.0 device and the top rate is 30 MB/s packed against 40 unpacked. Applies on reconnect.
settings-radio-dc-removal = DC removal
settings-radio-remove-adc-s-offset = Remove the ADC's offset
settings-radio-turn-off-see-raw-hardware = Turn it off to see raw hardware output. Worth knowing where the spur goes: the offset lands at the edge of the span, not its centre, because the signal is translated by a quarter of the sample rate on the way through.
settings-radio-every-command-exchanged-receiver-sample = Every command exchanged with the receiver, the sample-rate arithmetic, and the first samples decoded as I/Q.
settings-radio-receive-only-24-1800-mhz = Receive only, 24–1800 MHz. No SoapySDR and no libairspy needed. The receiver, sample rate and packing take effect on Apply; everything else applies as you change it.
settings-radio-not-yet-verified-against-real-4 = Not yet verified against real hardware. If it misbehaves, please send the diagnostic report — it contains every command exchanged with the receiver, the rate arithmetic, and the first samples decoded as I/Q pairs.
settings-radio-no-hydrasdr-rfone-found-press = No HydraSDR RFOne found — press Rescan
settings-radio-these-are-rates-receiver-turned = these are the rates this receiver turned out to have
settings-radio-receiver-only-reports-three-these = the receiver only reports three of these; the rest are in the firmware's alternate table and an older build may not carry them
settings-radio-rf-input = RF input
settings-radio-antenna-sma-only-socket-bias = The antenna SMA — the only socket with the bias tee behind it.
settings-radio-cable-socket-no-bias-tee = A cable socket. No bias tee here; the hardware puts it on ANT alone.
settings-radio-powers-active-antenna-preamp-down-2 = Powers an active antenna or preamp down the coax. Only the ANT socket has one — the two cable ports are plain inputs.
settings-radio-third-less-usb-traffic-leave-2 = A third less USB traffic. Leave it on: this is a USB 2.0 device and the top rate is 36 MB/s packed against 48 unpacked. Applies on reconnect.
settings-radio-one-these-boards-1d50-60a1 = One of these boards is on 1d50:60a1, the USB id HydraSDR's prototypes share with the Airspy R2 and Mini. sdroxide checks the firmware after opening and will say so if the wrong interface has been picked, in either direction.
settings-radio-receive-only-24-1800-mhz-2 = Receive only, 24–1800 MHz. No SoapySDR and no libhydrasdr needed. The receiver, sample rate and packing take effect on Apply; everything else applies as you change it.
settings-radio-re-list-fobos-sdrs-no = Re-list Fobos SDRs. No device is opened, so this is safe to press while receiving.
settings-radio-no-fobos-sdr-found-press = No Fobos SDR found — press Rescan
settings-radio-input-2 = Input
settings-radio-through-tuner-lna-vga-mixer = Through the tuner: LNA, VGA, and the mixer's own frequency range.
settings-radio-direct-sampling-hf1-antenna-input = Direct sampling on the HF1 antenna input — no tuner, no LNA/VGA gain, tuned entirely in software.
settings-radio-direct-sampling-hf2-antenna-input = Direct sampling on the HF2 antenna input, same as HF1.
settings-radio-both-real-adc-channels-once = Both real ADC channels at once, combined by the diversity filter below — a noise source nulled, or two fading paths combined.
settings-radio-direct-sampling-tuner-so-lna = Direct sampling — the tuner (and so LNA/VGA gain) is not in the path. The rate below is a target for the software downconverter, not a hardware setting, so the achieved rate can differ from it.
settings-radio-receiver-s-own-reported-rates = this receiver's own reported rates
settings-radio-measured-one-real-unit-connect = measured on one real unit — connect this receiver and Rescan to see its own list
settings-radio-clock-source = Clock source
settings-radio-external-reference = External reference
settings-radio-diversity-taps = Diversity taps
settings-radio-taps = {" "}taps
settings-radio-one-tap-gain-phase-null = One tap is a gain and a phase — a null at one frequency that gets worse either side of it, which is all an analogue phaser can do. Each further tap buys one sample period of the path difference between HF1 and HF2 that the filter can equalise, which is what turns that notch into a band quiet all the way across.
settings-radio-receive-only-rf-port-tuner = Receive only. RF port (tuner), HF1/HF2 (direct sampling, decoded in software) and HF1+HF2 (both at once, combined by the diversity filter) all work. The receiver, input and sample rate take effect on Apply; gain, clock source and taps apply as you change them — the diversity filter's mode, adapt rate and hold are on the main window's own DIV box once this radio is selected.
settings-radio-no-hackrf-found-press-rescan = No HackRF found — press Rescan
settings-radio-front-end-amplifier-8-db = Front-end amplifier, in 8 dB steps. This is the stage that changes sensitivity — and the stage that overloads first on a real antenna.
settings-radio-baseband-amplifier-after-mixer-2 = Baseband amplifier after the mixer, in 2 dB steps. Turn this up for a weak signal before reaching for the LNA.
settings-radio-rf-amp = RF amp
settings-radio-one-switch-shared-transmit-setting = One switch, shared with the transmit setting below — the radio applies whichever belongs to the direction it is entering. Off is usually right on a real antenna.
settings-radio-hackrf-pro-chooses-itself-three = A HackRF Pro chooses this itself — three quarters of the sample rate, filtered in the FPGA — and ignores anything the host asks for.
settings-radio-leave-automatic-choosing-one-too = Leave on Automatic. Choosing one too narrow does not just soften the band edges — it withdraws the LO offset that keeps the DC spike off your signal.
settings-radio-about-3-v-50-ma = About 3 V at 50 mA down the coax, for an active antenna or a preamp. A HackRF One or Pro only; the Jawbreaker and rad1o have no such circuit.
settings-radio-remove-dc-mirror-image = Remove DC and the mirror image
settings-radio-zero-if-radio-so-own = This is a zero-IF radio, so its own LO leakage sits mid-span and the mixer's quadrature error puts a mirror image across it. Turning this off shows raw hardware output, which is the quick way to tell a driver problem from a DSP one.
settings-radio-corrects-reference-oscillator = Corrects the reference oscillator.
settings-radio-transmit = Transmit
settings-radio-transmitter = Transmitter
settings-radio-off-default-while-off-backend = Off by default. While this is off the backend publishes no transmit channel at all, so nothing can key the radio. Applies on reconnect.
settings-radio-tx-vga-gain = TX VGA gain
settings-radio-transmit-driver-amplifier-drive-applied = The transmit driver amplifier. Drive is applied digitally before this stage, so leave the drive high and set output level here — turning drive down instead runs the DAC at a fraction of full scale and raises intermodulation.
settings-radio-same-switch-receive-setting-above = The same switch as the receive setting above, applied when the radio changes direction.
settings-radio-transmit-armed-into-dummy-load = Transmit is armed. Into a dummy load until you have measured it: a HackRF's harmonics are strong enough to need an external low-pass filter for the band you are on, and it is half duplex, so receive stops for the length of every over.
settings-radio-every-command-exchanged-radio-session = Every command exchanged with the radio this session, in order, including the sequence around each key-down.
settings-radio-dc-7-25-ghz-half = DC – 7.25 GHz, half duplex. No SoapySDR, no libusb and no libhackrf needed. The radio and the sample rate take effect on Apply; everything else applies as you change it. The band edges are the firmware's: a HackRF is specified for 1 MHz – 6 GHz (100 kHz on a Pro) and is heavily attenuated outside that, but it does tune there.
settings-radio-ask-sdrplay-api-service-device = Ask the SDRplay API service for its device list. Nothing is opened, so this is safe to press while receiving.
settings-radio-no-rsps-press-rescan-needs = no RSPs — press Rescan (needs the SDRplay API service)
settings-radio-both-tuners-running-api-fixes = With both tuners running, the API fixes the ADC at 6 MHz and hands back 2 Msps from a low IF — so 2 Msps is the widest span, and the narrower ones are that decimated. Takes effect on Apply.
settings-radio-rates-below-2-msps-run = Rates below 2 Msps run the ADC at 2 Msps and decimate in the service. Takes effect on Apply.
settings-radio-reduced-adc-resolution = {"  "}(reduced ADC resolution)
settings-radio-if-bandwidth = IF bandwidth
settings-radio-tuner-s-analog-filter-auto = The tuner's analog filter. Auto picks the widest one that fits the sample rate. Takes effect on Apply.
settings-radio-rsp-s-own-if-gain = The RSP's own IF-gain loop, run by the API service. Off hands the IF gain slider back to you — the setting for measurement and weak-signal digital modes.
settings-radio-agc-set-point = AGC set point
settings-radio-signal-level-loop-holds-adc = Signal level the loop holds the ADC at. Lower leaves more headroom for signals off-channel. How low it may go depends on the sample rate — above 8.064 Msps the converter has less headroom to give, and above 9.216 Msps less still.
settings-radio-rsp-s-native-gain-unit = The RSP's native gain unit: 20 dB is maximum gain (0 with the extended range below), 59 dB minimum. Applies immediately. Ignored while the AGC is running — the loop owns this value then, and the S-meter shows what it settled on.
settings-radio-extended-if-range = Extended IF range
settings-radio-lets-if-gain-reduction-go = Lets the IF gain reduction go below 20 dB, down to 0 — the last 20 dB of gain the receiver has. Off is the API's own default and the right one for ordinary listening, because the bottom of the range is where an RSP is easiest to overload. Worth having on for weak signals with the LNA already at 0. It also lets the AGC set point go up to 0 dBFS. Applies immediately.
settings-radio-allow-if-gain-reduction-below = Allow IF gain reduction below 20 dB
settings-radio-front-end-attenuation-steps-0 = Front-end attenuation in steps: 0 is maximum gain, each step up switches more attenuation in. Some bands have fewer steps — the driver clamps and keeps your choice for when you tune back, and the rail here ends at what the band the radio is on will take. A higher choice made on another band is kept and still shown beside it. Applies immediately.
settings-radio-t-0-max-gain = 0 = max gain
settings-radio-reference-error-parts-per-million-2 = Reference error in parts per million, applied by the device itself. Applies immediately.
settings-radio-radio-s-tuner = This radio's tuner
settings-radio-tuner = Tuner
settings-radio-which-rspduo-s-two-tuners = Which of the RSPduo's two tuners this radio listens on. The other one belongs to the radio configured for it — give that one the same receiver and the other tuner. Takes effect on Apply.
settings-radio-which-rspduo-s-two-tuners-2 = Which of the RSPduo's two tuners carries the aerial you are listening to. The other one carries the second aerial. Takes effect on Apply.
settings-radio-which-rspduo-s-two-tuners-3 = Which of the RSPduo's two tuners to run (one at a time). Takes effect on Apply.
settings-radio-applies-immediately = Applies immediately.
settings-radio-fm-broadcast-notch = FM broadcast notch
settings-radio-hardware-notch-over-88-108 = Hardware notch over the 88–108 MHz broadcast band, for when a local transmitter overloads everything else. Applies immediately.
settings-radio-dab-notch = DAB notch
settings-radio-hardware-notch-over-165-230 = Hardware notch over the 165–230 MHz DAB band. Applies immediately.
settings-radio-hdr-mode = HDR mode
settings-radio-enable-below-2-mhz = Enable below 2 MHz
settings-radio-rspdx-s-high-dynamic-range =
    The RSPdx's high-dynamic-range path for LF/MF. Applies immediately.

    It does not work yet: on the one RSPdx it has been tried against, switching it on silences the receiver — at every centre, filter, port, gain and rate tried. Whatever the mode needs is not on the documented API surface, and finding it is somebody's next job. Leave this off unless you are the one looking.

    What is known: it is not a mode that follows the dial. The path has a fixed analog filter, built only at the centres listed under the filter below, so tuning anywhere else could not work even once the rest does.
settings-radio-hdr-filter = HDR filter
settings-radio-feed-4-7-v-dc = Feed ~4.7 V DC up the coax
settings-radio-second-tuner = Second tuner
settings-radio-rspduo-two-whole-tuners-one = An RSPduo is two whole tuners on one board, clocked from one reference. Run both and they hear their spans at the same instant from the same clock — which makes two aerials on them coherent, with a relative phase set by the feedlines rather than by chance, and lets the pair be combined. Or leave them apart: the tuners tune separately, so the other one can be a second radio on its own band.
settings-radio-run-both-tuners = Run both tuners
settings-radio-puts-rspduo-api-s-dual = Puts the RSPduo in the API's dual-tuner mode, where the ADC is fixed at 6 MHz and the widest span is 2 Msps. Takes effect on Apply, because the mode is chosen when the device is opened.
settings-radio-used = Used for
settings-radio-what-other-tuner-takes-effect = What the other tuner is for. Takes effect on Apply — like running both at all, this is chosen when the board is opened.
settings-radio-add-second-radio-settings-radio = Add a second radio (Settings → Radio → +), give it this same receiver and the RSPduo's other tuner, and set it to run both tuners too — whichever radio opens the board puts it in dual-tuner mode, so both have to be expecting it. The two then tune independently: HF in one tab and VHF in the other, off one board. They share one ADC clock, so both run at the sample rate whichever radio opened the board asked for, and neither can transmit.
settings-radio-what-do = What to do with it
settings-radio-lna-state-2 = Its LNA state
settings-radio-second-tuner-s-own-front = The second tuner's own front-end attenuation. Set so both aerials show about the same noise floor: this is the adjustment everything else rests on, because combining weights the two branches by their noise, and a second front end driven into overload hands the filter a distorted copy of the interference — which cannot be subtracted from an undistorted one. Applies immediately.
settings-radio-if-gain-reduction-2 = Its IF gain reduction
settings-radio-second-tuner-s-if-gain = The second tuner's IF gain, in the RSP's native unit: 20 dB is maximum gain. Ignored while the AGC is running — and a steady gain is what the filter wants, so switching the AGC off is worth it for a null you intend to hold.
settings-radio-filter-length = Filter length
settings-radio-one-tap-gain-phase-null-2 = One tap is a gain and a phase — a null at one frequency that gets worse either side of it, which is all an analogue phaser can do. Each further tap buys one sample period of the path difference between the two aerials that the filter can equalise, which is what turns that notch into a band quiet all the way across.
settings-radio-adaptation-2 = Adaptation
settings-radio-slow-steady-left-converging-inside = Slow and steady at the left, converging inside a fraction of a second and visibly hunting at the right. Start fast to find the null, then hold it.
settings-radio-stop-filter-moving-reach-moment = Stop the filter moving. Reach for this the moment a null appears: a filter left adapting will re-aim itself at whatever becomes loudest, which on a quiet band is the station you are listening to.
settings-radio-restart = Restart
settings-radio-zero-filter-find-null-again = Zero the filter and find the null again.
settings-radio-nothing-here-can-tell-wanted = Nothing here can tell a wanted signal from an unwanted one — the filter only knows what the two aerials have in common. In Cancel, the second aerial wants to hear the noise source and as little of the band as possible, or it will dutifully cancel the station too. In Combine, both want to hear the same station. How deep the null is going runs to the log every few seconds.
settings-radio-not-yet-verified-against-rspduo = Not yet verified against an RSPduo: dual-tuner operation here is written from SDRplay's API rather than measured on the hardware.
settings-radio-no-receiver-listed-check-against = No receiver is listed to check this against — press Rescan, or read the log for what actually opened.
settings-radio-receive-only-1-khz-2 = Receive only, 1 kHz–2 GHz. Needs the vendor's SDRplay API service (sdrplay.com/api) — the RSPs after the original RSP1 have no open protocol. Device, sample rate, bandwidth and RSPduo tuner take effect on Apply; everything else applies as you change it.
settings-radio-board = Board
settings-radio-first-one-found-2 = First one found
settings-radio-receive-chain = Receive chain
settings-radio-both-chains-tune-together-they = Both chains tune together — they share one synthesiser — but they are separate front ends on separate sockets. Pick the one your aerial is on, which is the setting to reach for when one chain has had the HF matching modification and the other is stock.
settings-radio-receive-gain = Receive gain
settings-radio-one-combined-figure-which-limesuite = One combined figure, which LimeSuite distributes across the LNA, the TIA and the PGA itself. It takes whole decibels, so anything finer is truncated.
settings-radio-receive-port = Receive port
settings-radio-automatic-follows-frequency-lnal-low = Automatic follows the frequency: LNAL low, LNAH high — unless a LimeRFE is connected below, which is one cable into one socket, and then it is LNAW at every frequency. Name the socket yours is wired to if it is not that one. Which chain the socket belongs to is the picker above.
settings-radio-t-0-follows-sample-rate-worth = 0 follows the sample rate. Worth leaving there: a filter narrower than a quarter of the span silently withdraws the zero-IF LO offset, which puts the LO leakage back on top of the signal you are listening to.
settings-radio-following-rate = following the rate
settings-radio-corrections = Corrections
settings-radio-host-iq-dc-correction = Host IQ / DC correction
settings-radio-adaptive-image-dc-removal-side = Adaptive image and DC removal on this side, on top of the chip's own calibration. Turning it off is the one-click way to tell a driver problem from a DSP one.
settings-radio-calibrate-automatically = Calibrate automatically
settings-radio-runs-chip-s-own-dc = Runs the chip's own DC-offset and image calibration when the radio is opened, and again once the dial has settled on a new band or a different socket. Those numbers are measured at one frequency and are wrong elsewhere, which is what a carrier sitting in the middle of the span usually is. Costs about a second each time.
settings-radio-calibrate-now = Calibrate now
settings-radio-stalls-receiver-better-part-second = Stalls the receiver for the better part of a second.
settings-radio-second-aerial = Second aerial
settings-radio-socket = Its socket
settings-radio-same-first = Same as the first
settings-radio-gain-2 = Its gain
settings-radio-set-low-coupled-sample-your = Set this LOW. The coupled sample of your own transmitter is a strong signal, and a feedback chain driven into compression measures the amplifier's curve wrongly — it teaches the correction its own distortion. Start at the bottom and use the coupler's attenuator.
settings-radio-set-so-both-aerials-show = Set so both aerials show about the same noise floor. This is the adjustment the whole thing rests on: combining weights the two branches by their noise, and a second chain driven into compression hands the filter a distorted copy of the interference — which cannot be subtracted from an undistorted one.
settings-radio-table-steps-2 = Table steps
settings-radio-how-finely-correction-follows-amplifier = How finely the correction follows the amplifier's curve. More steps track a sharper knee, but each one has to be learned from the samples that landed in it — and the top of a speech amplitude histogram is thin. Thirty-two is enough for the smooth curve an HF amplifier actually has. Changing it starts the correction again.
settings-radio-how-hard-each-block-feedback = How hard each block of feedback moves the correction. An amplifier's curve does not change, so there is no need to hurry — the middle averages several overs' worth of noise out of it.
settings-radio-keep-correction-curve-learned-clean = Keep the correction as it is. A curve learned on a clean over is worth holding, and the amplifier will not have changed by the next one.
settings-radio-forget-correction-learn-again = Forget the correction and learn it again.
settings-radio-directional-coupler-amplifier-s-output = A directional coupler on the amplifier's output goes into this chain, and the transmitter compares what came back with what it meant to send — then sends the inverse of the difference, so what leaves the amplifier is straight. Twenty-odd decibels less intermodulation on other people's QSOs, with the amplifier keeping its power. The correction stays at unity until the feedback lines up with the transmission, so a coupler that is not connected costs nothing; and it can never ask the converter for more than full scale, so a feedback path reading nonsense cannot over-drive anything. How it is getting on runs to the log while you transmit.
settings-radio-transmit-not-armed-so-there = Transmit is not armed, so there is nothing here to correct.
settings-radio-off-interface-publishes-no-transmit = With this off the interface publishes no transmit channel at all, so nothing can key the radio.
settings-radio-transmit-gain = Transmit gain
settings-radio-transmit-port = Transmit port
settings-radio-board-s-two-transmit-sockets = The board's two transmit sockets are two different matching networks, not two jacks onto the same one: BAND1 (TX_1) carries 30 MHz to 1.9 GHz and BAND2 (TX_2) is the 13 cm port. Keying an amateur band below 23 cm out of BAND2 puts the over into a matching network that passes almost none of it — every setting reads correct, the drive is whatever you set, and the power meter stays at zero. Leave this on Automatic and it follows the dial.
settings-radio-limesdr-transmits-about-100-khz = A LimeSDR transmits from about 100 kHz to 3.8 GHz with no filtering of its own. Use a low-pass filter, an appropriate LimeRFE channel, or a dummy load.
settings-radio-limerfe-front-end = LimeRFE front end
settings-radio-connected = Connected by
settings-radio-limerfe-s-own-micro-usb = The LimeRFE's own micro-USB port, not the radio's. 9600 baud, fixed by its firmware.
settings-radio-receive-connector = Receive connector
settings-radio-transmit-connector = Transmit connector
settings-radio-follow-dial = Follow the dial
settings-radio-switch-filters-match-operating-frequency = Switch the filters to match the operating frequency, before any RF appears. Tuning within one band puts nothing on the control link.
settings-radio-relays = Relays
settings-radio-receive-attenuator = Receive attenuator
settings-radio-notch-filter = Notch filter
settings-radio-fan = Fan
settings-radio-worth-having-any-sustained-transmitting = Worth having on for any sustained transmitting.
settings-radio-automatic-board-receives-switched-transmit = On Automatic the board receives, and is switched to transmit at key-down and back at key-up — on either cabling. Its amateur channels have one filter with a transmit/receive switch either side of it, so a board asked for both at once puts that switch on the transmitter and stops hearing anything. The 30 MHz channel is reachable only through J5, which is one connector for both directions; above it, transmitting from J4 keeps the receive path off the connector the amplifier is driving.
settings-radio-limesdr-support-has-not-been = LimeSDR support has not been verified against hardware. This is the last session's trace, for an issue report.
settings-radio-gains-filters-corrections-antenna-sockets = Gains, filters, corrections, the antenna sockets and every LimeRFE control — its connectors, band, relays, attenuator, notch and fan — apply immediately. The board, the receive chain, the sample rate, arming transmit and the LimeRFE's connection take effect on Apply.

## Settings window, Radio tab: messages with values

settings-radio-fill-from = FILL FROM { $name }
settings-radio-decimation-auto = Automatic (nearest { $khz } kHz)
settings-radio-decimation-stage-server = Stage { $stage } — the server's rate ÷ { $divisor }
settings-radio-decimation-stage = Stage { $stage } — rate ÷ { $divisor }
settings-radio-kiwi-receive-only = Receive only — this is somebody else's antenna. The panadapter is the ~12 kHz the receiver sends as I/Q, which follows the dial; the band view above it is its own waterfall, and tuning across it retunes the receiver. About { $rate } kB/s while connected. Press "Apply / reconnect" to switch without a restart.
settings-radio-connected-detail = Connected: { $detail }
settings-radio-failed = Failed: { $error }
settings-radio-pluto-buffer = ≈ { $ms } ms, { $kib } KiB per transfer
settings-radio-driver-settings = { $driver } settings
settings-radio-rx888-coverage = 0–{ $mhz } MHz coverage, { $mbytes } MB/s over USB
settings-radio-rx888-digitised = of the { $mhz } MHz digitised
settings-radio-elad-16-bit = { $khz } kHz (16-bit samples)
settings-radio-elad-pad = { $db } dB pad in
settings-radio-hackrf-rx-preamp = { $db } dB preamp on receive
settings-radio-hackrf-tx-preamp = { $db } dB preamp on transmit
settings-radio-device-not-found = { $device } (not found)
settings-radio-lime-chain = Chain { $n } (RX{ $n }_*)
settings-radio-lime-chain-tx = Chain { $n } (RX{ $n }_* / TX{ $n }_*)
settings-radio-lime-second-chain = The board's other receive chain, on the { $sockets } sockets. It shares the synthesiser, so it hears the same span at the same instant as the first — which is what lets it carry a second aerial, or a sample of your own transmitter.
settings-radio-tx-blocked = Transmit is blocked: { $reason }
settings-radio-rfe-rx-fallback = Receiving on { $port }, these fall back to the unfiltered wideband path: { $bands }.
settings-radio-rfe-tx-fallback = Transmitting on { $port }, these fall back to the wideband path — no band amplifier and no filtering: { $bands }.

## Top bar and control strip (src/app/top_bar.rs)

topbar-how-far-one-press-moves = How far one press moves the dial. Tap to take the next step: 10 Hz, 100 Hz, 500 Hz, 1, 2.5, 5, 9, 10 and 25 kHz. Turn the row off in Settings › UI.
topbar-volume-gain-agc-squelch-noise = Volume, gain, AGC, squelch, the noise controls, and the RDS readout on FM broadcast
topbar-receiver = RECEIVER
topbar-vfo-b-split-rit-xit = VFO A/B, split, and the RIT/XIT offsets
topbar-tuning = TUNING
topbar-diversity-filter-which-way-combines = The diversity filter: which way it combines the two aerials, how fast it adapts, and holding it where it is
topbar-diversity = DIVERSITY
topbar-second-receiver-s-frequency-mode = The second receiver's frequency, mode, filter and level
topbar-sub-receiver = SUB RECEIVER
topbar-transceiver-s-own-aerial-socket = The transceiver's own aerial socket, its antenna tuner and SWR, and its power switch
topbar-radio = RADIO
topbar-tune-voice-keyer-drive-mic = Tune, the voice keyer, and the drive and mic levels
topbar-transmit = TRANSMIT
topbar-tune = {" "}TUNE{" "}
topbar-panadapter-two-layers-their-speeds = The panadapter — its two layers, their speeds and detail, peak hold — plus waterfall contrast, FFT size and the skimmers
topbar-display = DISPLAY
topbar-logbook-spots-awards-memories-scanner = Logbook, spots, awards, memories, the scanner, settings and the manual
topbar-system = SYSTEM
topbar-linked = Linked
topbar-not-linked = Not linked
topbar-bar-trace = bar / trace
topbar-needle-bar-trace = needle / bar / trace
topbar-swap-vfos = Swap VFOs
topbar-copy-b = Copy A to B
topbar-second-receiver-right-ear-tunes = Second receiver, in the right ear. It tunes independently of A/B — its controls appear in the SUB module, and its passband on the waterfall.
topbar-band-plan = , from the band plan
topbar-repeater-shift-following-band-plan = Repeater shift: following the band plan, which has no shift for this frequency — simplex
topbar-repeater-shift-transmit-above-below = Repeater shift: transmit above or below the dial, to work a repeater. Simplex now
topbar-repeater-shift = REPEATER SHIFT
topbar-auto = AUTO
topbar-take-shift-band-plan-dial = Take the shift from the band plan as the dial moves. It only speaks inside a repeater output sub-band — everywhere else it leaves the radio simplex, so the calling channels stay simplex.
topbar-offset = Offset
topbar-repeater-tone-ctcss-dcs-goes = Repeater tone: the CTCSS/DCS that goes out under the voice, the 1750 Hz burst, and the receive tone squelch
topbar-transmit-tone = TRANSMIT TONE
topbar-set-here-sent-nfm-sub = set here, sent on NFM — sub-audible signalling is an FM channel's
topbar-normal = NORMAL
topbar-invert = INVERT
topbar-bit-order-dcs-encoded-here = the bit order DCS is encoded with here is transcribed from the standard and has never been checked against a repeater — if it will not open, try CTCSS
topbar-t-1750-hz-burst = 1750 HZ BURST
topbar-every-over = EVERY OVER
topbar-open-burst-every-time-transmitter = Open with the burst every time the transmitter keys
topbar-send = SEND
topbar-send-burst-now-keying-transmitter = Send the burst now — keying the transmitter for its length if it is not already keyed
topbar-nfm-only-burst-fm-repeater = NFM only: the burst is an FM repeater's door-opener
topbar-receive-tone-squelch = RECEIVE TONE SQUELCH
topbar-require-receive-what-station-transmits = Require on receive what this station transmits
topbar-hide-docked-band-selector-click = Hide the docked band selector — click again to bring it back
topbar-show-docked-band-selector = Show the docked band selector
topbar-dock = DOCK
topbar-keep-band-mode-selector-open = Keep the band and mode selector open beside the waterfall instead of closing this popup every time
topbar-band-mode = BAND & MODE
topbar-hide-band-chip-brings-back = Hide — the band chip brings it back
topbar-undock = UNDOCK
topbar-return-selector-top-bar-popup = Return the selector to the top-bar popup
topbar-dec-off = DEC off
topbar-vol = Vol
topbar-counted-steps-not-decibels-what =
    {""}

    Counted in steps, not decibels: what one step is worth is the receiver's own business, and on an SDRplay it depends on the band as well. Right is still more gain.
topbar-gain = Gain
topbar-plutosdr-s-own-agc-setting =
    {""}

    The PlutoSDR's own AGC is setting this gain, so moving the slider does nothing. Set Settings → Radio → AGC to Manual to control it here.
topbar-agc-hang-time-click-cycle = AGC hang time — click to cycle: Off / Slow / Med / Fast
topbar-radio-s-own-agc-click = The radio's own AGC — click to cycle: Slow / Med / Fast. It cannot be switched off from here.
topbar-man = Man
topbar-manual-audio-gain-used-while = Manual audio gain, used while the AGC is off. Seeded from the level the AGC was holding when it was switched off.
topbar-radio-s-own-squelch-sent = The radio's own squelch, sent over the control link. This is the gate the audio actually passes through — the software one would only close further on what the rig already let by.
topbar-max-gain = max gain
topbar-manual-gain = manual gain
topbar-noise-reduction = noise reduction
topbar-impulse-noise-blanker = Impulse noise blanker
topbar-auto-notch-cancel-constant-tone = Auto-notch: cancel constant tone elements (heterodynes)
topbar-noise-reduction-off-click-switch = Noise reduction, off — click to switch it on, or pick an engine in the rows below
topbar-binaural-audio-not-while-sub = Binaural audio — not while the sub receiver has the right ear
topbar-binaural-audio-passband-spread-across = Binaural audio: the passband is spread across the two ears, so signals at different pitches come from different directions and the one you tune floats across. Click to go back to mono
topbar-binaural-cw-spread-passband-across = Binaural CW: spread the passband across the two ears, so that signals at different pitches come from different directions — a pile-up becomes several places instead of one crowded note, and tuning a station floats it across. Best on headphones
topbar-binaural-audio-spread-passband-across = Binaural audio: spread the passband across the two ears. On voice the noise spreads over the whole image while the station stays in the middle of it, which is easier to listen to for an hour — at the cost of the voice itself being spread out. Best on headphones
topbar-mute = MUTE
topbar-record-audio-raw-i-q = Record the audio, the raw I/Q, or both
topbar-wfm-stereo-forced-off-click = WFM stereo forced off — click for automatic stereo
topbar-wfm-stereo-pilot-locked-click = WFM stereo: pilot locked. Click to force mono
topbar-wfm-stereo-automatic-no-pilot = WFM stereo: automatic, no pilot on this station
topbar-rds-data-arriving-click-see = RDS data is arriving — click to see it
topbar-rds-no-data-station-click = RDS — no data on this station. Click anyway for the decoder's diagnostics
topbar-drm-decoding-click-details = DRM is decoding — click for the details
topbar-drm-click-decoder-s-state = DRM — click for the decoder's state
topbar-hd-radio-decoding-click-details = HD Radio is decoding — click for the details
topbar-hd-radio-click-decoder-s = HD Radio — click for the decoder's state
topbar-presets = PRESETS
topbar-preset-right-click = Right-click to change this preset
topbar-preset-edit = EDIT PRESET
topbar-preset-label = Label
topbar-preset-label-hint = What the chip says. Up to 8 characters.
topbar-preset-lo = Lo
topbar-preset-hi = Hi
topbar-preset-use-current = Use current
topbar-preset-use-current-hint = Take the edges of the filter the receiver has now
topbar-preset-save = Save
topbar-preset-reset = Reset
topbar-preset-reset-hint = Put back this mode's built-in preset
topbar-preset-cancel = Cancel
topbar-width-per-sideband = WIDTH PER SIDEBAND
topbar-width = WIDTH
topbar-passband-width-hertz-type-drag = Passband width in hertz — type it, or drag. Click into the field to enter an exact figure.
topbar-edges = EDGES
topbar-low-edge-hz-carrier = Low edge, in Hz from the carrier
topbar-high-edge-hz-carrier = High edge, in Hz from the carrier
topbar-noise-reduction-voice-off-click = Noise reduction (voice), off — click to pick an engine
topbar-record-audio = RECORD AUDIO
topbar-mp3 = ● MP3
topbar-what-you-hear-mp3-receiver = What you hear, as an MP3: the receiver in one channel and your own transmit in the other. This is the recording of a QSO.
topbar-mono = MONO
topbar-mixed-one-channel-click-two = Mixed to one channel — click for two
topbar-two-channels-rx-left-tx = Two channels: RX left, TX right — click for one mixed channel
topbar-quick-clip = QUICK CLIP
topbar-stop-after = STOP AFTER
topbar-no-stop = no stop
topbar-auto-record = AUTO-RECORD
topbar-set-squelch-auto-record-needs = Set a squelch — auto-record needs one to know silence
topbar-auto-record-needs-sdr-front = Auto-record needs an SDR front end with a software squelch
topbar-stop-following-squelch = Stop following the squelch
topbar-waiting-signal = waiting for a signal
topbar-record-spectrum = RECORD SPECTRUM
topbar-i-q-wav = ● I/Q WAV
topbar-i-q-wav-2 = I/Q WAV
topbar-raw-spectrum-receiver-delivering-stereo = The raw spectrum the receiver is delivering, as a stereo 32-bit float WAV (RF64 past 4 GB) that SDR#, SDRuno, HDSDR and SDRangel open — and that sdroxide itself plays back with --file. Large: 8 bytes a sample.
topbar-radio-hands-over-demodulated-audio = This radio hands over demodulated audio, so there is no I/Q to record.
topbar-what-capture-costs-sample-rate = What a capture costs at this sample rate
topbar-engine = ENGINE
topbar-no-noise-reduction-decoders-never = No noise reduction — the decoders never saw it anyway
topbar-strength = STRENGTH
topbar-fm-decrackle = FM-DECRACKLE
topbar-fm-decrackle-chip = DECRACKLE
topbar-fm-decrackle-hint = NFM: phase slips (crackle on weak signals) found and repaired in a ring buffer; ~1 ms delay
topbar-fm-deemph = DE-EMPH
topbar-fm-deemph-hint = De-emphasis 6 dB/octave above 300 Hz; off for transmitters without pre-emphasis
topbar-fm-decrackle-sens-hint = Sensitivity: magnitude dip below this share of its mean; 0.2 default, higher repairs more
topbar-nr-filters = FILTERS
topbar-sk-filter-hint = SK post-filter refines neural or other noise removal: penalises unwanted noise
    (the filter resonates, leading to echo) · filter by HDZ
topbar-sk-filter-tau-hint = Time constant tau: long = filter curve, short (0.1 s) = syllable gate too
topbar-sk-filter-open = open
topbar-sk-filter-close = close
topbar-sk-filter-open-hint = Open: how fast a bin opens when speech arrives in it
topbar-sk-filter-close-hint = Close: how fast it shuts afterwards; short = less reverb
topbar-sk-filter-width = width
topbar-sk-filter-width-hint = Mask smoothing, ± bins of 47 Hz; wider = less ringing, coarser
topbar-sk-filter-after-nr = after NR
topbar-sk-filter-after-nr-hint = SK behind RNN/DFNR instead of ahead of the AGC (variant B)
topbar-sk-filter-caption = SK POST-FILTER
topbar-sk-filter-reset-hint = SK back to defaults: τ 0.1 s, open 0.01 s, close 1 s, width ±1, ahead of NR
topbar-sk-filter-before-nr = ahead of NR
topbar-sk-preset-hint = Click loads, right-click stores the current values
    { $values }
topbar-hum-filter-hint = High-pass 300 Hz: 50/60 Hz mains hum and harmonics
topbar-tone = TONE
topbar-ctcss-dcs-no-sub-audible = CTCSS / DCS — no sub-audible tone on this signal
topbar-tone-squelch = TONE SQUELCH
topbar-carrier-squelch-open-any-signal = Carrier squelch: open on any signal
topbar-require-tone-currently-being-received = Require the tone currently being received
topbar-ctcss = CTCSS
topbar-any-dcs = ANY DCS
topbar-open-any-dcs-coded-signal = Open on any DCS-coded signal. Which of the 104 codes it carries cannot be read reliably here, so there is nothing finer to choose
topbar-which-socket-back-radio-receiving =
    Which socket on the back the radio is receiving on — its own ANT command, the same setting as the ANT button on the front panel. Click to step to the next one.

    The choice is remembered per band, and put back the next time the dial crosses into that band.
topbar-radio-s-separate-receiving-antenna =
    The radio's separate receiving antenna, switched into the receive path or out of it — its own RX ANT setting. The aerial on the main socket stays on transmit either way.

    The radio remembers this per band itself, so sdroxide reads it back after every band change rather than putting back what it last saw: clicking here is the only thing that moves it.
topbar-radio-s-own-power-switch =
    The radio's own power switch, over the control link, and the one true on/off in the program — not sdroxide's LINK switch, which closes sdroxide's end and leaves the radio running.

    For ON to reach anything the radio's control end has to stay awake while it is off: Network Control over the LAN, or a CI-V port still fed from the mains on a set switched off at the front.
topbar-switch-radio = Switch the radio on
topbar-switch-radio-off-audio-meters = Switch the radio off. The audio and the meters stop with it.
topbar-what-second-aerial-click-swap =
    What the second aerial is for — click to swap.

    CANCEL subtracts it from the first, in the gain, phase and delay that make a local noise source line up on both: the DSP form of a noise-cancelling phaser. COMBINE adds the two in the phase that reinforces, weighted by how well each hears — diversity reception, which fills in fades.
topbar-hold = HOLD
topbar-stop-filter-moving-reach-moment = Stop the filter moving. Reach for this the moment a null appears: a filter left adapting will re-aim itself at whatever becomes loudest, which on a quiet band is the station you are listening to.
topbar-restart = RESTART
topbar-zero-filter-find-null-again = Zero the filter and find the null again.
topbar-adapt = Adapt
topbar-how-fast-filter-chases-slow = How fast the filter chases: slow and steady at the left, converging inside a fraction of a second and visibly hunting at the right. Start fast to find the null, then HOLD it.
topbar-where-sub-receiver-listens-shift = Where the sub receiver listens. Shift-click the waterfall, or drag inside the sub's passband, to move it.
topbar-dial = ←DIAL
topbar-move-sub-receiver-main-dial = Move the sub receiver to the main dial
topbar-dial-2 = DIAL←
topbar-move-main-dial-sub-receiver = Move the main dial to the sub receiver
topbar-filter = Filter
topbar-sub-receiver-passband-edges-hz = Sub receiver passband edges, in Hz
topbar-sub-receiver-level-plays-right = Sub receiver level (it plays in the right ear)
topbar-ptt = {" "}PTT{" "}
topbar-voice-keyer-record-transmit-stored = Voice keyer: record and transmit stored messages
topbar-drive = Drive
topbar-tune-2 = Tune
topbar-tx-audio-processing-microphone-click = TX Audio is processing the microphone — click to open it
topbar-click-open-tx-audio-gate = Click to open TX Audio: gate, EQ, compressor and limiter for the microphone
topbar-mic = Mic
topbar-cessb = CESSB
topbar-microphone-gain = Microphone gain
topbar-deviation-how-far-mode-s =
    Deviation: how far this mode's burst swings a radio that modulates it itself. An FM transmitter turns audio level into frequency swing and has no ALC to catch it, so full scale into a data input set for voice over-deviates — which sounds completely normal to a listener and decodes for nobody.

    Kept per mode, so a deviation set for 1200 baud packet never lands on FT8.
topbar-transmit-audio-how-hard-mode =
    Transmit audio: how hard this mode drives the modulator of a radio that modulates what we send it — a CAT rig on its sound card, a FLEX, an Icom on its network port. Bring it down until the rig's ALC is barely moving and set the power at the radio; ALC riding on a constant-envelope digital mode is what splatters.

    Drive is not this control: on these radios Drive reaches the rig's power register and never touches its audio.

    Kept per mode — FT8, RTTY, PSK and MCW each keep their own.
topbar-tx-audio = TX audio
topbar-cw-psk-rtty-skimmers-decode = CW / PSK / RTTY skimmers — decode signals across the band and mark them on the waterfall
topbar-skimmers = SKIMMERS
topbar-run-skimmer = Run this skimmer
topbar-minimum-snr-decoded-signal-needs = Minimum SNR a decoded signal needs to be spotted
topbar-needs-wideband-iq-source = needs a wideband IQ source
topbar-cw-decoder = CW DECODER
topbar-how-many-signals-model-reads = How many signals the model reads at once. The rest keep their marker but carry no text.
topbar-spots = SPOTS
topbar-save-skimmer-s-spot-list = Save the skimmer's spot list to a file
topbar-show-hide-full-band-waterfall = Show/hide the full-band waterfall strip above the panadapter — everything this receiver can see at once
topbar-spectrum-waterfall-either-layer-both = Spectrum: the line, peak hold, S/N, 3D and how fast it follows. Lit while the spectrum is shown.
topbar-spectrum = SPECTRUM
topbar-show-spectrum = SHOW SPECTRUM
topbar-draw-spectrum-line-across-top = Draw the spectrum line across the top of the panadapter. Switched off, the waterfall takes the whole height.
topbar-peak-hold = PEAK HOLD
topbar-snr-guide = S/N
topbar-snr-guide-tip = S/N per frequency over noise floor, running mean (τ 10 s up, 6 s down)
    Full height = S/N TOP
topbar-sk-guide = SK
topbar-sk-guide-tip = Spectral kurtosis per frequency: noise 1, steady carrier 0
    Modulation and speech above 1 (rolling)
topbar-caption-sk-top = SK TOP
topbar-sk-top-tip = SK at the full height of the SK line, dB (noise 0 dB)
topbar-caption-sk-tau = SK τ
topbar-sk-tau-tip = Memory of the SK averaging (rolling, like S/N)
topbar-caption-snr-top = S/N TOP
topbar-snr-top-tip = S/N at full height of S/N line, dB over noise floor
topbar-caption-snr-min = S/N MIN
topbar-snr-min-tip = Minimum S/N for labels and candidates, dB over noise floor
spots-snr-only-tip = In view only spots with S/N ≥ S/N MIN (±2 kHz); needs IN VIEW
topbar-spot-labels-chip = LABELS
topbar-spot-labels = Master switch: off, no labels at all (spots and decoded calls); the switches beside it grey out. Which kinds: SPOTS
topbar-caption-tempo = Tempo
topbar-snr-label-filter = BY S/N
topbar-snr-label-filter-tip = Spot labels only at S/N ≥ S/N MIN (±2 kHz), every 2 s, faded
topbar-snr-label-peak = WITH PEAKS
topbar-snr-label-peak-tip = Labels of AM and CW stations only at a peak 3 dB over its surroundings (0.6–2 kHz): none on interference or sidebands
topbar-snr-label-flanks = IGNORE FLANKS
topbar-snr-label-flanks-tip = No labels within ±10 kHz of a station 15 dB over noise floor; its own label stays
topbar-trace-highest-level-each-column = Trace the highest level each column has reached over the live line, decaying back down — what the band did while you were looking elsewhere
topbar-draw-spectrum-receding-surface-instead = Draw the spectrum as a receding surface instead of a flat line: the newest spectrum across the front, the ones before it flowing away from you. The last couple of seconds of the band as a landscape — a carrier that comes and goes is a ridge rather than a line that twitches. The grid and the peak hold belong to the flat line and are not drawn on it.
topbar-how-quickly-spectrum-line-follows = How quickly the spectrum line follows the band. Slower averages more frames into each other: a steadier line, and a weak carrier that stands still long enough to read. The waterfall is not touched by it — those rows get every frame either way.
topbar-waterfall = WATERFALL
topbar-resolution = RESOLUTION
topbar-show-waterfall = SHOW WATERFALL
topbar-draw-scrolling-waterfall-below-spectrum = Draw the scrolling waterfall below the spectrum. Switched off, the spectrum line takes the whole height.
topbar-how-fast-waterfall-scrolls-lines = How fast the waterfall scrolls, in lines a second: Slow 5, Medium 28, Fast 56, Faster 112, Fastest 224. The engine clocks these itself, so the two fastest are real detail rather than the same line drawn twice — as far as the receiver can feed them: a line can never show more than one transform, and a narrow front end makes only a few dozen a second. They cost history, since the waterfall keeps a fixed number of lines — 73 seconds at Medium, 9 at Fastest.
topbar-decode-labels = DECODE LABELS
topbar-mark-every-decoded-station-waterfall = Mark every decoded station on the waterfall with its callsign, at the frequency it was heard on — FT8, FT4 and the other slotted modes. A good opening puts thirty of them across the span twice a minute, over the traces you are reading; switch them off and the waterfall is just the band. The decode list beside it still has every callsign.
topbar-switch-3d-set = Switch 3D on to set this
topbar-how-3d-spectrum-drawn-only = How the 3D spectrum is drawn. Only the shape differs — both renderings show the same spectra, and both hide what is behind them, so a strong signal in front stands over the band it is covering.
topbar-lines = LINES
topbar-one-trace-per-remembered-spectrum = One trace per remembered spectrum, in the flat line's own colour, each hiding the ones behind it. The shape without the levels, and the reading to pick when the shape is what you are after — it costs a little more than the solid one rather than less, because it draws the same surface and then strokes every crest on top of it.
topbar-solid = SOLID
topbar-filled-surface-coloured-waterfall-s = A filled surface coloured by the waterfall's palette, so the level is in the colour as well as in the height and the two halves of the panadapter agree about what a strong signal looks like. Change the palette in Settings › Display.
topbar-how-fast-3d-spectrum-flows = How fast the 3D spectrum flows away from you, in rows a second: Slow 6, Medium 12, Fast 24, Faster 48. The surface is a fixed number of rows deep, so this is also how much time it holds. Slower is a longer memory and a surface that crawls; faster is a shorter one that moves. The waterfall keeps its own scroll rate, below.
topbar-how-many-columns-panadapter-waterfall = How many columns the panadapter and its waterfall are drawn with. Auto reads this machine's renderer and the size of the panadapter and picks the most it can carry — a 4K screen wants 4096. Every column is a byte in every frame, so a client connected to a remote station pays for the detail on its link: 4096 columns at 60 fps is about a quarter of a megabyte a second, twice the standard width.
topbar-read-machine-screen-take-most = Read this machine and this screen, take the most they can carry, and follow them if they change
topbar-width-force-right-now-whatever = The width in force right now, whatever the row above asks for
topbar-keep-floor-ceiling-set-best = Keep the floor/ceiling set for best waterfall contrast — eased back into place on a band change, after a pan or zoom, and when the levels drift. Switch it on to fit at once; switch it off to keep the levels where you set them.
topbar-keep-tuned-frequency-middle-panadapter = Keep the tuned frequency in the middle of the panadapter: the window slides under the dial instead of the picture jumping a whole span when you tune off the edge. Switch it on to centre at once; switch it off to pan and zoom wherever you like.
topbar-spectrum-floor-ceiling-fft-size = Waterfall: show, flip, scroll, contrast (floor / ceiling), labels, and the resolution (FFT size, columns). Lit while the waterfall is shown.
topbar-level-drawn-bottom-spectrum-darkest = The level drawn at the bottom of the spectrum and as the darkest waterfall colour. Bring it up until the noise floor just darkens. Setting it switches FIT off.
topbar-ceil = ceil{" "}
topbar-level-drawn-top-bring-down = The level drawn at the top. Bring it down until the strongest signal you care about reaches full colour. Setting it switches FIT off.
topbar-fft-size = FFT SIZE
topbar-flip = FLIP
topbar-scroll-waterfall-upwards-newest-row = Scroll the waterfall upwards — newest row at the bottom (V)
topbar-logbook-all-qsos-digital-manual = Logbook — all QSOs (digital + manual)
topbar-live-spots-dx-cluster-pota = Live spots — DX cluster, POTA, SOTA, PSK Reporter
topbar-award-tracking-dxcc-was-waz = Award tracking — DXCC / WAS / WAZ / grids
topbar-band-conditions-published-forecast-beside = Band conditions — the published forecast beside what has actually been heard on each band
topbar-satellite-qo-100-beacon-calibration = Satellite — the QO-100 beacon calibration is running
topbar-satellite-doppler-tracking-qo-100 = Satellite — Doppler tracking, and the QO-100 beacon calibration
topbar-ism-band-devices-decoding-now = ISM-band devices — decoding now, whether or not this window is open. Switch it off with DECODING inside the window.
topbar-ism-band-devices-weather-sensors = ISM-band devices — weather sensors, meters and home automation heard around you
topbar-public-sdrs-internet-browse-kiwisdr = Public SDRs on the internet — browse the KiwiSDR and SpyServer directories and open one as a radio
topbar-winlink-radio-email = Winlink radio email
topbar-memory-channels = Memory channels
topbar-scanner-stopped-signal = Scanner — stopped on a signal
topbar-scanner-running = Scanner — running
topbar-scan-memory-channels-frequency-range = Scan memory channels or a frequency range
topbar-hfdl-ground-network-decode-log = HFDL ground network — the decode log and aircraft map, below the waterfall. Switch the decoder on with LISTEN inside the panel.
topbar-hfdl-ground-network-decoding-now = HFDL ground network — decoding now. Open the panel.
topbar-hfdl-ground-network-aircraft-shortwave = HFDL ground network — the aircraft shortwave data link, one listening channel at a time
topbar-grid-tracker-worked-maidenhead-squares = Grid tracker — worked Maidenhead squares on a map, with what is heard
topbar-amplifier-set-up-hardrock-under = Amplifier — set up a Hardrock under Settings → Amplifier
topbar-settings-device-gains-antennas-audio = Settings — device gains, antennas, audio devices
topbar-user-manual-f1 = User manual (F1)
topbar-range-was-typed-settings-radio = That range was typed in Settings ▸ Radio ▸ RX range, not reported by the radio. Widen it, or empty the box to use whatever the device says about itself.
topbar-what-device-reports-about-itself = That is what the device reports about itself. If the radio does cover this band, state its real range in Settings ▸ Radio ▸ RX range.
topbar-band = BAND
topbar-recent = RECENT
topbar-recent-empty =
    Fills after 3 s on a place
topbar-mode = MODE
topbar-digital = DIGITAL

## Top bar: the System box's window chips.

topbar-chip-log = LOG
topbar-chip-spots = SPOTS
topbar-chip-awards = AWARDS
topbar-chip-bands = BANDS
topbar-chip-sat = SAT
topbar-chip-ism = ISM
topbar-chip-public-sdr = PUBLIC SDR
topbar-chip-mail = MAIL
topbar-chip-mem = MEM
topbar-chip-scan = SCAN
topbar-chip-hfdl = HFDL
topbar-chip-grid = GRID
topbar-chip-amp = AMP
topbar-chip-settings = ⚙ SETTINGS
topbar-chip-help = ? HELP

## Top bar: the mode-defaults chip.

topbar-changed-agc = AGC
topbar-changed-squelch = squelch
topbar-changed-auto-notch = auto-notch
topbar-changed-stereo = stereo
topbar-changed-binaural = binaural
# $mode is a mode name (USB, CW…); $changed lists what differs from its defaults.
topbar-mode-defaults =
    Back to { $mode }'s defaults. Changed from them: { $changed }.

    Click to put them back. { $mode }'s own values return, and what you set here is forgotten.

## Settings window, Radio tab: longer messages with values

settings-radio-no-such-baud = the radio has no { $baud } baud setting — { $default } will be used instead
settings-radio-icom-scope-hint =
    Streams the radio's own scope sweep over the CI-V link and draws it as the panadapter — the only picture of the band a demod-audio rig can give, since the audio it sends is what already came through its filter.

    Needs the radio set up for it: CI-V USB Baud Rate to 115200 and CI-V USB Port to "Unlink from {"["}REMOTE]" (both under SET > Connectors > CI-V), and the baud rate above set to match — below { $min_baud } the sweeps do not fit down the link and the scope stays off.

    The sweeps share the radio's internal USB bus with its own sound card. If received audio starts to drop out with the scope on, this box is the first thing to try turning off.
settings-radio-rx-tx-words = { $rx } / TX { $tx }
settings-radio-soapy-native = Hardware above is supported directly by sdroxide: { $names }. Selecting that interface above gives you its own settings — gain stages, filters and notches SoapySDR cannot express — and needs no SoapySDR module.
settings-radio-rx888-width-hint = How much of the digitised spectrum the panadapter shows at once. The whole DSP chain runs at this width, so wider costs proportionally more CPU — 1/2 is the entire band in the waterfall, and a serious amount of arithmetic. Above the VHF crossover ({ $crossover } MHz) the panadapter is not looking at the antenna but at the tuner's IF, which is 8 MHz wide and parked at { $if_centre } MHz — so anything wider than { $widest } cannot be centred on it, and the extra width is dead spectrum beside the signal rather than more of it.
settings-radio-sdrplay-serial-missing = Serial { $serial } is not among the receivers the SDRplay service reports. Pick one of the listed receivers, or replug this one and press Rescan.
settings-radio-hdr-filter-hint =
    The analog filter in front of the HDR path — a different control from the receiver's own bandwidth. Each setting is built at a fixed set of centres and can do nothing elsewhere; this one is built at { $centres }. Applies immediately.

    The path itself does not work yet — see the switch above — so this chooses a filter for a mode that is currently silent.
settings-radio-rspduo-only = Only an RSPduo has a second tuner, and the receiver selected here is an { $model }, so this will do nothing.
settings-radio-lime-tx-gain-low = Transmit gain is { $db } dB, at the bottom of its range — a few microwatts out of the board, which will read as nothing on a power meter whatever is downstream of it. That is the default so that an armed transmitter cannot surprise you; raise it before you key.

## Settings window, General: SWR

settings-general-swr-tune-limit = (tuning: { $limit }:1)

## Top bar and control strip: messages with values

# $reading is the meter's own hover text; $faces lists the faces a click cycles through.
topbar-smeter-hover =
    { $reading }

    Click to cycle meter face: { $faces }
topbar-step-down = Down { $step }
topbar-step-up = Up { $step }
# $auto is topbar-band-plan or empty.
topbar-repeater-shift-hover = Repeater shift { $shift } — receiving on { $rx }, transmitting on { $tx } MHz{ $auto }
topbar-tone-and-burst = Transmitting { $tone } under the voice, and a { $ms } ms 1750 Hz burst at the start of every over
topbar-tone-under-voice = Transmitting { $tone } under the voice
topbar-burst-every-over = A { $ms } ms 1750 Hz burst at the start of every over
topbar-match-tx = MATCH TX ({ $tone })
topbar-decimation-on = Front-end decimation: the receiver is being given the middle 1/{ $n } of the { $msps } Msps this radio streams — { $span } — and the rest is thrown away before any of it reaches the receiver.
topbar-decimation-off = Front-end decimation, off: the receiver sees the whole { $span } this radio streams.
topbar-decimation-hint =
    { $what }

    A narrower span means finer waterfall resolution, a quieter noise floor (3 dB per halving) and less CPU — at the cost of the band either side of it. The dial still tunes anywhere; the radio moves its LO to follow.

    Click to cycle: off / 2 / 4 … { $max }.
topbar-rx-gain-hint = Front-end RX gain ({ $stage }). Too much clips the receiver's ADC and smears spurious signals across the band; too little and it goes deaf.
topbar-rx-gain-stages =
    {""}

    This rig has { $n } RX gain stages — the rest are in Settings → Device.
topbar-passband-now =
    {""}

    The passband is at { $dbfs } dBFS now.
# $level is topbar-passband-now or empty.
topbar-squelch-hint =
    Gate the audio below this power in the receive passband, in dBFS. Left is open.{ $level } Set it above the noise and below the signal.

    A stream that arrives with the radio's own AGC already in it — an Icom's 12 kHz IF, for one — sits far higher on this scale than an SDR's raw baseband does, which is why the rail reaches full scale.
topbar-nr-cycle-hint =
    Noise reduction: { $strength } — { $engine }

    Click to cycle the strength: Low / Med / High / Off. The engine is in the rows below.
topbar-recording-both = Recording { $audio } and { $iq }
topbar-recording-audio = Recording audio to { $audio }
topbar-recording-iq = Recording I/Q to { $iq }
topbar-auto-record-armed = Auto-record armed: a file per transmission, closed after { $secs } s of silence
topbar-rds-hover = RDS: { $ps }. Click for the station's data
topbar-drm-hover = DRM: { $service }. Click for the broadcast's details
topbar-drm-no-codec = DRM: { $summary } — locked, but its { $codec } audio cannot be decoded here. Click for what is missing
topbar-drm-state = DRM: { $summary } — click for the decoder's state
topbar-hd-hover = HD Radio: { $station }. Click for the broadcast's details
topbar-hd-state = HD Radio: { $summary } — click for the decoder's state
topbar-binaural-preset = { $preset } in each ear — { $lo } … { $hi } Hz, both sidebands
topbar-nr-hover =
    Noise reduction: { $strength } — { $engine }

    Click to change engine or strength
topbar-clipping = Clipping { $span } — press again to start it over
topbar-record-clip = Record { $span } and stop — a clip to attach to a reception report
topbar-starting-clip = starting a { $span } clip…
topbar-stop-recording-after = Stop the MP3 recording after { $minutes } minutes
topbar-stops-in = stops in { $time }
topbar-auto-record-hint = Record each transmission to its own file, closed after { $secs } seconds of silence
topbar-recording-silence = recording · { $secs } s of silence (closes at { $hold })
topbar-tone-open = Receiving { $tone }, which is the tone squelch — audio open
topbar-tone-closed = Receiving { $heard }, but the tone squelch wants { $wanted } — audio stays closed
topbar-tone-heard = Receiving CTCSS/DCS { $tone }
topbar-tone-nothing = Tone squelch { $tone }: nothing matching it is being received
topbar-tone-use = USE { $tone }
topbar-sockets = Sockets: { $sockets }
topbar-voice-keyer-tx = Transmitting { $slot } — click to open the voice keyer
topbar-drive-uncal-gen = This frequency, which is on no amateur band, is not calibrated, so the setting reaches the transmitter whole.
topbar-drive-uncal-band = { $band } is not calibrated, so the setting reaches the transmitter whole.
# $trim is signed (+1.5); $diff is its size; $more is "less" or "more".
topbar-drive-cal-gen = This frequency, which is on no amateur band, is calibrated { $trim } dB, so the same setting puts { $diff } dB { $more ->
        [less] less
       *[more] more
    } on the air here than on an uncalibrated band.
topbar-drive-cal-band = { $band } is calibrated { $trim } dB, so the same setting puts { $diff } dB { $more ->
        [less] less
       *[more] more
    } on the air here than on an uncalibrated band.
topbar-drive-hover =
    How hard the transmitter is driven. On a radio sdroxide modulates itself it scales the modulated samples; on a rig with its own power control it is the fraction of rated power the rig is asked for.

    { $trim }

    Settings → Radio → Transmit drive by band.
topbar-cessb-hover =
    Controlled-envelope SSB: more average power for the same peak, without splatter — three or four decibels of apparent loudness at the far end. How many decibels the voice is driven into the processor; 0 is off, 6 is a sensible first try. It does not touch Drive.

    Now: { $now }
topbar-waterfall-rate = { $rows } rows a second — { $seconds } seconds of band
topbar-detail-auto-columns = { $n } columns, whatever Auto would have picked here
topbar-n-columns = { $n } columns
topbar-fft-bins-whole = { $n } bins across the whole stream — one per column, or finer
topbar-fft-bins-pooled = { $n } bins pooled into the panadapter's { $cols } columns: sharper signals, not more of them
topbar-sat-locked = Satellite — locked on { $name }
topbar-amp-in-line = { $model } — in line (PTT)
topbar-amp-bypassed = { $model } — bypassed (OFF)
topbar-amp-not-answering = { $model } — not answering
topbar-filter-edges = { $lo } … { $hi } Hz from the carrier
topbar-filter-isb = Receive filter: { $width } Hz in each ear ({ $edges }). ISB's two sidebands are separate transmissions, so the width is the width of one of them.
topbar-filter-width = Receive filter: { $width } Hz wide ({ $edges }).
topbar-filter-hint =
    { $what }

    Click for the width, the two edges and this mode's presets, as numbers — the panadapter's grips place a passband by eye, this is where an exact figure is typed.
topbar-band-outside-range =
    { $band } is outside what this radio receives ({ $ranges } MHz).

    { $why }
# $time is "day" or "night".
topbar-band-forecast = { $band }: { $verdict } ({ $time ->
        [day] daytime
       *[night] night
    }) — forecast by HAMQSL.com from the solar indices, not a measurement of your own path.

## Top bar: short words and captions

topbar-squelch-open = open
topbar-off = off
topbar-recording = recording
topbar-sql = sql
topbar-stations = stations
topbar-caption-reaction = reaction
topbar-caption-flow = flow
topbar-caption-detail = Columns
topbar-floor = floor

## Digital-mode panel tabs (src/app/panels/mod.rs, pane_label).

pane-aircraft = AIRCRAFT
pane-chart = CHART
pane-chat = CHAT
pane-decodes = DECODES
pane-files = FILES
pane-heard = HEARD
pane-image = IMAGE
pane-map = MAP
pane-messages = MESSAGES
pane-monitor = MONITOR
pane-panel = PANEL
pane-qso = QSO
pane-reading = READING
pane-receive = RECEIVE
pane-saved = SAVED
pane-send = SEND
pane-spots = SPOTS
pane-stations = STATIONS
pane-status = STATUS
pane-terminal = TERMINAL
pane-text = TEXT
pane-traffic = TRAFFIC
pane-vessels = VESSELS
pane-wfall = WFALL

## Digital-mode panels, shared (src/app/panels/mod.rs)

panels-save = {" "}SAVE{" "}
panels-nothing-decoded-save = Nothing decoded to save
panels-clear-rx = {" "}CLEAR RX{" "}
panels-empty-receive-window-nothing-air = Empty the receive window. Nothing that is on the air stops.
panels-nothing-received-clear = Nothing received to clear
panels-save-what-panel-has-decoded = Save what this panel has decoded to a file
panels-prop = PROP
panels-shade-map-where-signals-are = Shade the map by where signals are actually getting through, on each band. Every reception is placed at the midpoint of its path — the patch of ionosphere that bent it — not at the far station, so this is a map of the sky rather than of where radio amateurs live. Built from paths this station has taken part in, plus — when the Reverse Beacon Network is switched on under Settings → Spots — what the world's skimmers are hearing, which covers the bands this radio is not on.
panels-night = NIGHT
panels-shade-where-sun-down-twilight = Shade where the Sun is down, and the twilight between, so the grey line shows on the map. Low bands go long and high bands close on the night side of it, and the terminator itself is where the DX is.
panels-all-bands = ALL BANDS
panels-every-band-once-one-hue = Every band at once, one hue each — overall conditions.
panels-one-band = ONE BAND
panels-one-band-cold-hot-blue = One band, cold to hot: blue, green, yellow, red.
panels-frequency-you-are-actually-working = The frequency you are actually working: the dial plus the tone offset beside it. This is what goes in the log — the dial alone is that much low.
panels-send-return = SEND ON RETURN
panels-freq = ⇵ FREQ
panels-nothing-agreed-mode-save-your = Nothing agreed for this mode — save your own below.
panels-forget-saved-frequency = Forget this saved frequency
panels-dial-already-one-your-saved = This dial is already one of your saved frequencies.
panels-dial-already-agreed-list-mode = This dial is already in the agreed list for this mode.
panels-decode-squelch-raise-stop-decoding = Decode squelch — raise to stop decoding noise
panels-message-repeats-long-transmit-held = The message repeats for as long as transmit is held — a meteor catches whatever part of it is passing.
panels-w1abc-w9xyz-fn42 = W1ABC W9XYZ FN42
panels-tx = {"  "}TX ON{"  "}
panels-tx-2 = {"   "}TX{"   "}
panels-call-cq = {" "}CALL CQ{" "}
panels-nocall = NOCALL

## TX Audio window (src/app/tx_audio.rs)

txaudio-heard-too-little-speech-talk = Heard too little speech — talk normally for the whole count and try again.
txaudio-tx-audio = TX Audio
txaudio-mode-not-voice-mode-so = This mode is not a voice mode, so nothing it sends passes through the chain. Settings made here take effect in SSB, AM and FM.
txaudio-bypass = BYPASS
txaudio-switch-every-stage-out-once = Switch every stage out at once, denoiser included, so the microphone goes out unprocessed. Click again to restore exactly the stages that were on — the quickest A/B of what the chain is doing.
txaudio-denoise = DENOISE
txaudio-rnnoise-microphone-ahead-every-other = RNNoise on the microphone, ahead of every other stage, so room noise is removed before the gate or the compressor can bring it up. Adds about 10 ms of delay.
txaudio-digital-voice = DIGITAL VOICE
txaudio-also-run-freedv-rade-digital = Also run FreeDV / RADE digital voice through the chain. Off by default: a vocoder is trained on natural speech and may decode worse from a heavily processed voice, so try it on the air before relying on it. TCI audio and the digital modes never pass through the chain.
txaudio-test = Test
txaudio-rec = {" "}⏺ REC{" "}
txaudio-play = {" "}▶ PLAY{" "}
txaudio-play-test-phrase-through-chain = Play the test phrase through the chain as it is set now, to your speakers. Adjust while it plays and hear the change. Receive audio is replaced while it plays.
txaudio-recording = recording…
txaudio-no-phrase-yet = no phrase yet
txaudio-live = Live
txaudio-live-2 = {" "}🎧 LIVE{" "}
txaudio-hear-your-microphone-through-chain = Hear your microphone through the chain as you talk, without keying the transmitter. Use headphones: through speakers it feeds back. You hear yourself a few tens of milliseconds late, which makes talking awkward — set levels on the meters, and judge the sound with REC / PLAY. Receive audio is replaced while it runs; pressing PTT stops it.
txaudio-meters = {" "}METERS{" "}
txaudio-run-your-microphone-through-chain = Run your microphone through the chain for the meters alone: nothing on the speakers, so no delay to talk against and nothing to feed back. The way to set mic and input gain, the gate threshold and how hard the compressor works. Receive audio carries on; pressing PTT stops it.
txaudio-monitoring-live-use-headphones-speakers = Monitoring live — use headphones, or the speakers will feed back into the microphone.
txaudio-voice-modes-only-tci-clients = Voice modes only. TCI clients and the digital modes bypass the chain; when it is in use, keep the radio's own TX EQ and processor flat.
txaudio-auto-set-stopped-microphone-was = Auto-set stopped: the microphone was taken.
txaudio-microphone-may-clipping-before-gets = {" "}The microphone may be clipping before it gets here — turn its own gain down if it sounds harsh.
txaudio-level = Level
txaudio-mic-control-same-one-transmit = The Mic control — the same one as on the transmit bar, shared by every profile. 0 dB at its middle, +6 dB at the top.
txaudio-mic = Mic
txaudio-gain-top-mic-microphone-stored = Gain on top of Mic for this microphone, stored with the profile. The presets' thresholds assume a voice peaking around -10 dBFS on the IN meter; Auto-set puts it there. Stays in under BYPASS, and applies to digital voice too: it is level, not processing.
txaudio-input = Input
txaudio-auto-set = Auto-set
txaudio-auto-set-cancelled = Auto-set cancelled.
txaudio-talk-now-you-would-air = Talk now, as you would on the air…
txaudio-db = -∞ dB
txaudio-chain = Chain
txaudio-order-audio-meets-them = In order the audio meets them
txaudio-switch-stage = Switch this stage on
txaudio-earlier-chain = Earlier in the chain
txaudio-later-chain = Later in the chain
txaudio-output-limiter = The output limiter
txaudio-output = Output
txaudio-trim-limiter-always-last = Trim and limiter — always last
txaudio-profiles = Profiles…
txaudio-gate-expander = Gate / expander
txaudio-pushes-room-noise-fan-rumble = Pushes room noise, fan rumble and hiss down in the pauses. Set the threshold just above your noise floor and below your quietest syllables.
txaudio-gate = GATE
txaudio-hard-gate-10-1-down = Hard gate: 10:1, down to -40 dB
txaudio-expander = EXPANDER
txaudio-gentle-expander-2-1-never = Gentle expander: 2:1, never more than -15 dB — the room never goes dead
txaudio-reset = RESET
txaudio-back-defaults = Back to the defaults
txaudio-threshold = Threshold
txaudio-below-gate-starts-close = Below this the gate starts to close.
txaudio-return = Return
txaudio-hysteresis-once-open-closes-only = Hysteresis: once open, it closes only this far below the threshold, so a voice hovering at it does not make the gate flutter.
txaudio-hold = Hold
txaudio-how-long-stays-open-after = How long it stays open after the voice drops, so word endings are not clipped.
txaudio-release = Release
txaudio-how-fast-closes-once-hold = How fast it closes once the hold is over. Lengthen it if the gate pumps.
txaudio-floor = Floor
txaudio-most-ever-turns-pauses-down = The most it ever turns the pauses down.
txaudio-ratio = Ratio
txaudio-how-steeply-closes-below-threshold = How steeply it closes below the threshold.
txaudio-look-ahead = Look-ahead
txaudio-opens-much-before-word-starts = Opens this much before a word starts, so its first consonant is not chopped. Adds the same delay to everything transmitted.
txaudio-high-pass-low-shelf-two = High-pass, low shelf, two peaks, high shelf and low-pass. Drag a point on the curve, or set it below. Set the low-pass to your transmit filter's width.
txaudio-flat = FLAT
txaudio-every-band-back-0-db = Every band back to 0 dB, both edges off
txaudio-freq = Freq
txaudio-gain = Gain
txaudio-q-slope = Q / Slope
txaudio-high-pass = High-pass
txaudio-removes-handling-noise-breath-pops = Removes handling noise, breath pops and hum below the corner.
txaudio-low-shelf = Low shelf
txaudio-body-mud-cut-little-here = Body, or mud: cut a little here for a voice that sounds boxy.
txaudio-peak-1 = Peak 1
txaudio-bump-dip-voice-band-often = A bump or dip in the voice band — often a cut around 300–500 Hz.
txaudio-peak-2 = Peak 2
txaudio-presence-lift-around-2-3 = Presence: a lift around 2–3 kHz is what cuts through on SSB.
txaudio-high-shelf = High shelf
txaudio-brightness = Brightness.
txaudio-low-pass = Low-pass
txaudio-stops-energy-above-transmit-filter = Stops energy above the transmit filter from being spent in the modulator.
txaudio-t-24-db-oct = 24 dB/oct
txaudio-t-12-db-oct = 12 dB/oct
txaudio-click-steeper-gentler-slope = Click for a steeper or gentler slope
txaudio-de-esser = De-esser
txaudio-turns-down-only-sibilant-band = Turns down only the sibilant band when "s" and "sh" cross the threshold, so the rest of the voice keeps its level. Find the frequency by sweeping it while saying "sister silver cellophane" and parking where the hiss is loudest.
txaudio-frequency = Frequency
txaudio-centre-sibilant-band = Centre of the sibilant band.
txaudio-width-band-higher-narrower = Width of the band — higher is narrower.
txaudio-band-level-where-reduction-starts = Band level where reduction starts. Too low and it works all the time, dulling the voice.
txaudio-amount = Amount
txaudio-most-band-ever-turned-down = The most the band is ever turned down. Start around -6 dB.
txaudio-attack = Attack
txaudio-how-fast-reacts = How fast it reacts.
txaudio-how-fast-lets-go = How fast it lets go.
txaudio-compressor = Compressor
txaudio-brings-quiet-syllables-up-holds = Brings quiet syllables up and holds loud ones back. Aim for 3–6 dB of reduction on peaks; more than 10 dB is obvious compression, better suited to contests.
txaudio-level-above-which-starts-compress = Level above which it starts to compress.
txaudio-how-hard-compresses-above-threshold = How hard it compresses above the threshold.
txaudio-how-fast-reacts-peak-10 = How fast it reacts to a peak. 10–25 ms keeps consonants crisp.
txaudio-how-fast-recovers = How fast it recovers.
txaudio-knee = Knee
txaudio-soft-knee-width-0-hard = Soft-knee width; 0 is a hard knee.
txaudio-makeup = Makeup
txaudio-gain-after-compression-bring-level = Gain after compression, to bring the level back up.
txaudio-drive = Drive
txaudio-gain-ahead-compressor-so-more = Gain ahead of the compressor, so more of the voice crosses the threshold — raises the average power. Pair with the output limiter.
txaudio-phase = Phase
txaudio-all-pass-phase-rotator-ahead = All-pass phase rotator ahead of the detector. Evens out the lopsided peaks a voice has, so the same peak level carries more average power. 4 is a common broadcast setting; 0 is off.
txaudio-last-stage-trim-look-ahead = The last stage: a trim, and a look-ahead limiter that holds every peak under the ceiling without the splatter hard clipping makes. Off by default, so switching the chain on does not change a drive level you have already set.
txaudio-limiter = Limiter
txaudio-on = On
txaudio-ceiling = Ceiling
txaudio-no-peak-leaves-chain-above = No peak leaves the chain above this.
txaudio-trim = Trim
txaudio-output-gain-ahead-limiter = Output gain, ahead of the limiter.
txaudio-off = {"  "}off
txaudio-meters-move-while-you-transmit = Meters move while you transmit a voice over, play the test phrase, or run LIVE or METERS.
txaudio-profiles-2 = Profiles
txaudio-profile-whole-chain-every-stage = A profile is the whole chain: every stage's settings, which are on, their order, and the input gain for your microphone. Loading one replaces the chain; the Mic control and drive stay as they are.
txaudio-name = Name
txaudio-save = Save
txaudio-store-chain-now-under-name = Store the chain as it is now under this name, replacing a profile of the same name
txaudio-import = Import…
txaudio-read-profile-json-file = Read a profile from a .json file
txaudio-tx-audio-profile = TX audio profile
txaudio-no-profiles-yet-save-stores = No profiles yet. Save stores the chain you have set up now.
txaudio-load = Load
txaudio-export = Export…
txaudio-delete = Delete
txaudio-starting-points = Starting points
txaudio-set-your-level-auto-set = Set your level with Auto-set first — the presets' thresholds assume it — then tune from here rather than transmitting as loaded: record a phrase and listen after each change. A preset keeps your input gain.
txaudio-profile-loaded = Profile loaded.

## Solar-system 3D view overlay (src/solar3d/overlay.rs)

solar-prop = PROP
solar-where-signals-are-actually-getting =
    Where signals are actually getting through, band by band. Every reception this station takes part in — WSPR, FT8/FT4, JS8, and the logbook — is placed at the midpoint of its path, which is the patch of ionosphere that bent it, rather than at the far station. So this is a map of the sky, not of where radio amateurs live.

    It can only show paths this station has been one end of. Oceans light up because the midpoints of long paths fall there; Antarctica stays dark because nobody is transmitting from it. Off by default — it paints the whole planet.
solar-orbits = ORBITS
solar-orbital-paths = Orbital paths
solar-clouds = CLOUDS
solar-cloud-cover-noaa-s-global = Cloud cover from NOAA's global geostationary mosaic, drawn as a depth of air rather than a picture stuck on the surface: infrared says how cold each cloud top is, and that is how high it stands. Thunderstorms flash inside the towers they build.
solar-planets = PLANETS
solar-other-seven-planets-eighteen-their = The other seven planets, eighteen of their moons, and Saturn's and Uranus's rings. Planet positions come from JPL's Keplerian element set; the moons are circular orbits fitted to JPL Horizons.
solar-coronal-mass-ejection-trajectory-cones = Coronal mass ejection trajectory cones
solar-sun-obs = SUN OBS
solar-solar-observations-sun-s-disk = Solar observations on the Sun's disk: sunspot active regions, and where the flares came from
solar-labels = LABELS
solar-body-region-labels = Body and region labels
solar-small-bodies = SMALL BODIES
solar-names-asteroids-comets-they-are = Names on the asteroids and comets. They are drawn either way — this is only whether thirty-five designations are drawn with them. Whatever it is set to, the body the camera is on and anything the find box has matched stay named.
solar-decoded-ft8-ft4-stations-path = Decoded FT8/FT4 stations and the path to the station being worked
solar-sats = SATS
solar-amateur-radio-satellites-their-orbits = Amateur-radio satellites, their orbits and elevation from your QTH
solar-aurora = AURORA
solar-auroral-oval-noaa-s-ovation = The auroral oval from NOAA's OVATION model, drawn as emission shells at their real altitudes, with the equatorward edge of the 10 % contour marked on the surface
solar-awards = AWARDS
solar-award-coverage-every-dxcc-entity = Award coverage: every DXCC entity painted by what your logbook is still missing. Orange burns where you have never worked the entity, amber where you have but it is unconfirmed, dim green where a QSL has come back. Follows the band filter in the AWARDS window.
solar-view = VIEW
solar-which-body-camera-orbits-animated = Which body the camera orbits, and the animated tour
solar-layers = LAYERS
solar-what-drawn-orbits-clouds-cmes = What is drawn: orbits, clouds, CMEs, labels, QSOs, aurora
solar-which-sdo-channel-wraps-sun = Which SDO channel wraps the Sun, and how old it is
solar-weather = WEATHER
solar-aurora-propagation-numbers-which-bands = The aurora, the propagation numbers and which bands are open
solar-space-weather = SPACE WEATHER
solar-nothing-measured-yet = nothing measured yet
solar-scale = SCALE
solar-body-moon-orbit-exaggeration = Body and Moon-orbit exaggeration
solar-time = TIME
solar-scrub-whole-scene-forward-back = Scrub the whole scene forward and back
solar-activity = ACTIVITY
solar-replay-last-hour-decodes-globe = Replay the last hour of decodes on the globe
solar-what-propagation-heat-map-showing = What the propagation heat map is showing
solar-propagation = PROPAGATION
solar-auto = ▶ AUTO
solar-fly-spline-through-set-framed = Fly a spline through a set of framed viewpoints. Any mouse input cancels it.
solar-moons-ride-circular-orbits-fitted = Moons ride circular orbits fitted to JPL Horizons: within a degree or two of where they really are, and up to six for Miranda, whose orbit plane swings too fast for a circle to follow.
solar-fetch-everything-again-now = Fetch everything again now
solar-all-sats = ALL SATS
solar-show-every-satellite-element-set = Show every satellite in the element set, not just the popular ones. Orbit rings stay on the curated few — ninety at once is unreadable.
solar-starting = starting…
solar-body = body{" "}
solar-moon-orbit = moon orbit{" "}
solar-stretch-earth-moon-distance-so = Stretch the Earth→Moon distance so the pair can be seen apart
solar-sun = sun{" "}
solar-sun-radius-exaggeration-leave-1 = Sun radius exaggeration. Leave at 1× to keep the CME geometry readable — a swollen Sun swallows the base of every cone.
solar-show = SHOW
solar-paint-globe-what-getting-through = Paint the globe by what is getting through
solar-paints-globe-what-getting-through = Paints the globe by what is getting through, on each band. Every reception is placed at the midpoint of its path — where the ionosphere did the work — rather than at the far station. Built from what this station hears, plus the world's skimmers when the Reverse Beacon Network is switched on.
solar-display = DISPLAY
solar-all-bands = ALL BANDS
solar-every-band-once-one-hue = Every band at once, one hue each, mixing where two overlap. The view for 'what are conditions like' rather than 'is twenty open'.
solar-one-band = ONE BAND
solar-one-band-cold-hot-blue = One band, cold to hot: blue, green, yellow, red.
solar-band = BAND
solar-nothing-heard-yet = nothing heard yet
solar-bands = BANDS
solar-sources = SOURCES
solar-contacts-logbook-path-was-open = Contacts in the logbook. A path that was open, with no signal report worth the name — an RST is not an SNR, so these count towards how busy a cell is and never towards its margin.
solar-stations-reported-hearing-one-downloaded = Stations that reported hearing this one, downloaded from WSPRnet. The only feedback a transmitting beacon ever gets.
solar-reverse-beacon-network-skimmers-what = Reverse Beacon Network skimmers: what everyone else is hearing, on every band, including the ones this radio is not on. Coarser than the rest — an RBN line carries no locators, so both ends are placed at their country's centre, which is close for a small country and badly out for a large one. Switch on under Settings → Spots.
solar-decodes-station-made-each-measured = Decodes this station made. Each is measured against its own mode's floor, so a WSPR report and an FT4 one are comparable.
solar-memory = MEMORY
solar-half-life = half-life{" "}
solar-how-long-reception-takes-count = How long a reception takes to count for half as much. The ionosphere's own memory is short — an opening two hours old should not be arguing with one from two minutes ago.
solar-live = LIVE
solar-follow-band-happens = Follow the band as it happens
solar-replay = ⏸ REPLAY
solar-replay-2 = ▶ REPLAY
solar-replay-last-hour-decodes-over = Replay the last hour of decodes, over and over
solar-min-ago = {" "}min ago
solar-where-last-hour-globe-showing = Where in the last hour the globe is showing
solar-trail = trail{" "}
solar-how-long-decode-s-arc = How long a decode's arc stays on the globe behind the head
solar-speed = speed{" "}
solar-how-much-faster-than-real = How much faster than real time the replay runs
solar-qso-layer-off = QSO layer off
solar-no-decodes-yet = no decodes yet
solar-qth-not-set-enter-your = QTH not set — enter your grid square in Settings
solar-mode = MODE
solar-path = PATH
solar-sent = SENT
solar-rcvd = RCVD
solar-grid = GRID
solar-dxcc = DXCC
solar-geostationary-always-above-your-horizon = Geostationary — always above your horizon.
solar-no-passes-next-48-hours = No passes in the next 48 hours from your QTH.
solar-start = START
solar-max-el = MAX EL
solar-aos-los-are-azimuths-horizon = AOS/LOS are azimuths at the horizon. Times are UTC.
solar-locked-unlock = {" "}● LOCKED — UNLOCK{" "}
solar-release-satellite-lock = Release the satellite lock
solar-lock = {" "}LOCK ON{" "}
solar-track-satellite-main-window-doppler = Track this satellite in the main window: Doppler-corrected RX and TX, transponder-mapped uplink, rotator steering
solar-no-frequencies-file-one-add = No frequencies on file for this one — add them in Settings ▸ TLE.
solar-frequencies-yours = FREQUENCIES · YOURS
solar-frequencies = FREQUENCIES
solar-link = LINK
solar-downlink-mhz = DOWNLINK (MHz)
solar-uplink-mhz = UPLINK (MHz)
solar-doppler-shifts-these-few-khz = Doppler shifts these by a few kHz across a LEO pass.
solar-sim = -- SIM
solar-satellite-planet-comet = satellite, planet or comet
solar-planet-asteroid-comet = planet, asteroid or comet
solar-clear-search = Clear the search
solar-no-match = no match
solar-no-sounder = no sounder
solar-heard = HEARD ≥
solar-kp = Kp / A
solar-x-ray = X-ray
solar-observed-above-sounder-better-than = observed above the sounder — better than modelled
solar-power-n-s = power N/S
solar-edge-n-s = edge N/S
solar-kp-peak-24-h = Kp peak 24 h
solar-observed-forecast = observed | forecast
solar-bands-open = BANDS OPEN
solar-dxcc-coverage = DXCC COVERAGE
solar-ir-vis = IR+VIS
solar-ir-only = IR only
solar-no-deep-convection = no deep convection
solar-t-1-storm = 1 storm
solar-glancing = {" "}· glancing
solar-direction-estimated = {" "}· direction estimated

## Digital-mode setup dialogs (src/app/panels/setup.rs)

setup-aprs-setup = APRS Setup
setup-packet-setup = Packet Setup
setup-ft8-ft4-ft2-setup = FT8 / FT4 / FT2 Setup
setup-my-callsign = My callsign
setup-my-grid = My grid
setup-station-call = Station call
setup-ssid-oe3jjs-10-packet-station = With an SSID — OE3JJS-10. A packet station is conventionally a different SSID from the operator, and nothing transmits until this is set.
setup-speed = Speed
setup-t-9600-needs-radio-s-data = 9600 needs the radio's data port. A microphone and speaker path destroys it at both ends — that is the radio, not sdroxide.
setup-tx-delay = TX delay
setup-flags-sent-before-frame-so = Flags sent before a frame, so the far end can hear us and lock its clock. On a CAT rig sdroxide alone spends 165–240 ms getting on the air, and the rig's own transmit-ready time is on top of that.
setup-packet-length = Packet length
setup-bytes = {" "}bytes
setup-most-single-frame-carries-shorter = The most a single frame carries. Shorter frames survive a marginal path — only the frame that was hit is resent — and cost more overhead. 128 on HF, 256 where the path is solid.
setup-frames-sent-before-waiting-acknowledgement = Frames sent before waiting for an acknowledgement. More fills a good path; on a bad one it is more to resend.
setup-extended = Extended
setup-ask-mod-128 = Ask for mod-128
setup-extended-sequence-numbers-window-bigger = Extended sequence numbers, for a window bigger than seven. Many nodes refuse the request with a DM, which looks exactly like a station that would not talk to you — so leave it off unless you know the far end wants it.
setup-answer-calls = Answer calls
setup-accept-connections = Accept connections
setup-off-winlink-client-which-dials = Off for a Winlink client, which dials out and has no reason to answer. On to be reachable as a peer: a call arrives in the terminal pane and you talk to whoever made it. A station whose link is already busy with a Winlink session refuses calls until that finishes.
setup-connect-text = Connect text
setup-what-caller-greeted = what a caller is greeted with
setup-sent-station-connects-us-once = Sent to a station that connects to us, once the link is up. Empty sends nothing — which looks broken to whoever called.
setup-default-via = Default via
setup-oe3xlr-1-oe3xms-1 = OE3XLR-1,OE3XMS-1
setup-digipeater-path-terminal-starts-path = The digipeater path the terminal starts with. The path to your local node is the same every time, and retyping it is how a hop gets left off.
setup-beacon = Beacon
setup-t-0-disables-timer = 0 disables the timer
setup-beacon-text = beacon text
setup-kiss-server = KISS server
setup-serve = Serve
setup-offer-modem-kiss-tnc-socket = Offer this modem as a KISS TNC on a socket, so Pat, an APRS client or the Linux AX.25 stack can use the radio.
setup-aprs-call = APRS call
setup-callsign-station-beacons-under-ssid = The callsign this station beacons under, with its SSID — OE3JJS-9 for a car, -10 for an I-gate, -5 for a phone. Leave it empty to use the callsign above.
setup-no-callsign-station-will-not = No callsign: this station will not transmit at all — not a beacon, not a message, not an acknowledgement.
setup-symbol = Symbol
setup-two-characters-protocol-carries-them = The two characters as the protocol carries them: a table (`/` or `\`) and a symbol code. A digit or a letter in the first position is an overlay, drawn on top of the alternate table's icon.
setup-path = Path
setup-wide1-1-wide2-1 = WIDE1-1,WIDE2-1
setup-how-far-you-ask-repeated = How far you ask to be repeated. This is the single most consequential setting on the channel: every hop multiplies the transmissions the whole network makes on one shared frequency.
setup-position = Position
setup-my-grid-2 = From my grid
setup-beacon-centre-locator-above-reported = Beacon the centre of the locator above, reported with the ambiguity a locator actually has — a six-character one is a couple of kilometres across, and saying so is honest. Untick to give exact coordinates instead.
setup-latitude = Latitude
setup-longitude = Longitude
setup-locator = from the locator
setup-not-coordinate = not a coordinate
setup-comment = Comment
setup-sent-every-beacon = sent with every beacon
setup-t-0-default-never-beacons-timer = 0 — the default — never beacons on a timer. Thirty minutes is the convention for a fixed station; a moving one beacons oftener, but every beacon is somebody else's channel time.
setup-compressed = Compressed
setup-compressed-position-format-third-air = The compressed position format: a third of the air time and more precise. Every receiver since the 1990s reads it.
setup-first-goes-out-one-interval = The first goes out one interval from now, not immediately.
setup-messages = Messages
setup-acknowledge = Acknowledge
setup-answer-messages-addressed-you-acknowledgement = Answer messages addressed to you. An acknowledgement is a transmission this station makes without being asked, so a receive-only setup should turn it off — and the beacon then stops claiming to be reachable too.
setup-flags-sent-ahead-every-frame = Flags sent ahead of every frame, so the far end's modem hears the carrier and locks its clock before the data starts. It has to outlast everything between pressing transmit and radiating: sdroxide alone spends 165–240 ms of it on a CAT rig, and a radio taking audio over a network buffers more on top. Too little and the far end never locks — the transmission is there and nothing decodes it.
setup-shared-packet-mode = shared with the packet mode
setup-keep-stations = Keep stations
setup-how-long-station-stays-map = How long a station stays on the map after it was last heard. Also what the map's fade is measured against, so a short window shows only what is live.
setup-auto-reply = Auto-reply
setup-answer-snr-grid-status = Answer SNR? / GRID? / STATUS? / INFO?…
setup-answer-direct-question-addressed-you = Answer a direct question addressed to you, to @ALLCALL or to a group you are in: SNR?, GRID?, STATUS?, INFO?, HEARING? and AGN? (which repeats your last message to that station, if it was within ten minutes). Never answers another station's traffic, and never answers itself.
setup-heartbeat = Heartbeat
setup-off = Off
setup-turbo-does-not-beacon-local = Turbo does not beacon — it is the local and VHF speed.
setup-sends-your-callsign-grid-so = Sends your callsign and grid so others know you are receivable. The first goes out one interval from now, not immediately.
setup-beacon-frequency = Beacon frequency
setup-t-500-1000-hz = 500–1000 Hz
setup-move-each-beacon-free-slot = Move each beacon to a free slot in the heartbeat sub-band, the way JS8Call does: it is where stations watching for beacons look, and it keeps an unattended transmitter off somebody else's QSO. The slot is chosen when the beacon actually goes out, clear of everything being decoded.
setup-working-freq = Working freq
setup-beacon-where-you-are-working = Beacon where you are working instead. Against the band convention, but it keeps everything you transmit in one place.
setup-heartbeat-reply = Heartbeat reply
setup-answer-heartbeats-signal-report = Answer heartbeats with a signal report
setup-tell-station-beaconed-how-well = Tell a station that beaconed how well you copied them. Off by default: a busy band carries a heartbeat every slot, and answering all of them would flood exactly the band heartbeats exist to keep quiet. Rate-limited to one answer per station every 15 minutes, and never while a message is still arriving or while you have something queued to send.
setup-status-message = Status message
setup-js8-station-info = Station info
setup-js8-station-info-example = e.g. IC-7300 50W END-FED
setup-js8-station-info-hint = Sent in reply to INFO?
setup-js8-callsign = JS8 callsign
setup-js8-callsign-hint = Only if JS8 should use a different callsign, such as KN4CRD/P or KN4CRD/QRP. Leave empty to use My callsign.
setup-js8-groups = Groups
setup-js8-groups-hint = Groups you are a member of, separated by spaces or commas. Messages to them count as addressed to you: they are highlighted, and the questions in them (SNR?, GRID?…) are answered.
setup-js8-auto-response = Auto-response
setup-js8-auto-response-on = Answer group messages with a canned reply
setup-js8-auto-response-on-hint = When a message to a group matches a rule below, send that rule's reply, repeated if the rule says so. Off by default: this transmits in answer to traffic nobody addressed to you personally. ABORT, or sending anything yourself, cancels the repeats still waiting. Never answers your own messages, and never sends more than 20 replies an hour in all.
setup-js8-rule-on = Rule on
setup-js8-rule-name = Rule name
setup-js8-rule-move-up = Move up. The first rule that matches a message is the one that answers.
setup-js8-rule-delete = Delete this rule
# "When [@GROUP] hears [phrase]": the two words around the group and phrase fields.
setup-js8-rule-when = When
setup-js8-rule-hears = hears
setup-js8-rule-group-hint = The group whose messages this rule answers
setup-js8-rule-any-message = any message
setup-js8-rule-phrase-hint = Text the message must contain, ignoring case and spacing. Leave empty to answer any message to the group.
setup-js8-rule-reply = Reply
setup-js8-rule-reply-hint = @SENDER, @GROUP, @MYCALL and @MYGRID are filled in when it is sent.
setup-js8-rule-to = To
setup-js8-reply-to-sender-hint = Directed to the station that wrote
setup-js8-reply-to-group-hint = To the whole group
setup-js8-reply-to-as-written-hint = Exactly as written: start the reply with its addressee
# "Send [N] ×, every [M min]": the words around the repeat count and the gap.
setup-js8-rule-send = Send
setup-js8-rule-times-every = ×, every
setup-js8-rule-min-suffix = {" "}min
setup-js8-rule-cooldown = Same station again after
setup-js8-rule-cooldown-hint = The cooldown: a station repeating its call inside it does not start the replies again.
setup-js8-rule-preview-empty = Sends nothing — the reply is empty
setup-js8-rule-no-group = Name the group this rule answers, e.g. @EMCOMM.
setup-js8-rule-self-retriggers = This reply goes to the group and contains the phrase it answers. Two stations with this rule would answer each other until the cooldown runs out.
setup-js8-add-rule = + Add rule
setup-tx-period = TX period
setup-even = Even
setup-odd = Odd
setup-auto-sequence = Auto-sequence
setup-decode-depth = Decode depth
setup-fast = Fast
setup-normal = Normal
setup-deep = Deep
setup-how-hard-ft8-decoder-works =
    How hard the FT8 decoder works for weak signals.

    Fast — one pass, no signal subtraction: quickest, fewest decodes.
    Normal — flat multi-pass subtraction: a little quicker than Deep, a little less thorough.
    Deep — the checkpointed multi-pass: the most decodes, and ~1.2 s on a busy slot.
setup-auto-tx-frequency = Auto TX frequency
setup-choose-transmit-frequency-automatically-quietest = Choose the transmit frequency automatically: the quietest spot in the period you transmit in, rather than the frequency of the station you are answering. Off does NOT hold it — it answers on the frequency of the station being called. To hold, use Hold TX.
setup-hold-tx-frequency = Hold TX frequency
setup-pin-transmit-tone-where-nothing = Pin the transmit tone where it is. Nothing moves it: not answering a station, not the call queue, not calling CQ, not a click on a decode or the waterfall. Overrides Auto TX frequency. Changing band is the one exception: the offset you last set on the new band comes back with it. For where your licence is narrower than the band plan — on a UK 60 m dial of 5357 kHz the allocation ends at 5358.0, so the tone must stay under 1000 Hz.
setup-tx-watchdog = TX watchdog
setup-stop-transmitting-after-long-no = Stop transmitting after this long with no reply and no action from you. 0 disables it.
setup-give-up-after = Give up after
setup-calls = {" "}calls
setup-unanswered-calls-one-station-before = Unanswered calls to one station before moving on. Calling CQ is exempt. 0 disables it.
setup-contest = Contest
setup-everyday-exchange-grid-report-rr73 = The everyday exchange: grid, report, RR73.
setup-european-vhf-contests-send-signal = European VHF contests: send a signal report, a serial number and your SIX-character locator, in the message layout WSJT-X calls EU VHF Contest. CQ goes out as CQ TEST.
setup-serial = Serial
setup-serial-number-next-exchange-will = The serial number the next exchange will carry. It advances by itself as each contact is logged and is remembered between sessions — set it by hand only to start a contest, or to pick up where a paper log got to.
setup-needs-6-character-grid = ⚠ needs a 6-character grid
setup-eu-vhf-contest-messages-carry = EU VHF Contest messages carry a six-character locator (JN88DD, not JN88). Until My grid has one, the ordinary exchange is sent instead.
setup-dxpedition = DXpedition
setup-ordinary-ft8-operation = Ordinary FT8 operation.
setup-calling-dxpedition-running-fox-mode = Calling a DXpedition running Fox mode: call from above 1000 Hz, move down onto the Fox when it answers, and log on its RR73 without sending 73.
setup-run-pile-up-several-signals = Run the pile-up: several signals at once, a queue of callers, worked strongest and rarest first. CALL CQ starts it, STOP QSO stands it down.
setup-fox-signals = Fox signals
setup-once = {" "}at once
setup-simultaneous-transmissions-spaced-60-hz = Simultaneous transmissions, spaced 60 Hz apart. They share the transmitter's power, so more signals means each is weaker.
setup-tx-audio = TX audio
setup-how-loud-mode-s-burst =
    How loud this mode's burst is handed to a radio that modulates it itself — and on FM that is the deviation, because an FM transmitter turns audio level into frequency swing and has no ALC to catch it. 1200 baud packet wants about 3 kHz where voice wants 5, so full scale into a data input set for voice over-deviates — which sounds completely normal to a listener and decodes for nobody. Turn it down until other stations report you, or set the level at the radio.

    This level belongs to this mode alone. A mode you have never set starts from the FM figure, and the sideband modes start from their own — so a deviation set for 1200 baud never lands on FT8.
setup-how-loud-mode-s-over =
    How loud this mode's over is handed to a radio that modulates it itself — a CAT rig on its sound card, a FLEX, an Icom on its network port. On sideband this is drive into the modulator: bring it down until the rig's ALC is barely moving and set the power at the radio, because ALC riding on a constant-envelope mode is what splatters. Drive reaches the rig's power register here, not its audio, so this is the level.

    This level belongs to this mode alone: FT8, RTTY, PSK and MCW each keep their own, because they do not load a modulator the same way. The same rail is on the transmit strip, where you can reach it while you are transmitting.
setup-deviation-fm = deviation, on FM
setup-drive-ssb = drive, on SSB
setup-grid = Grid
setup-report = Report
setup-r-report = R+Report

## AMP window (src/app/amp.rs)

amp-t-10-w-scale-rescaled = , 10 W scale, rescaled
amp-t-10-w-scale = , 10 W scale
amp-t-2-w-scale-rescaled = , 2 W scale, rescaled
amp-t-2-w-scale = , 2 W scale
amp-t-100-w-scale-rescaled = , 100 W scale, rescaled
amp-t-100-w-scale = , 100 W scale
amp-t-20-w-scale-rescaled = , 20 W scale, rescaled
amp-t-20-w-scale = , 20 W scale
amp-rescaled = , rescaled
amp-and = {" "}AND{" "}
amp-not-while-transmitting-switching-relay = Not while transmitting: switching a relay with RF on it can damage it. Unkey first.
amp-no-amplifier-set-up-choose = No amplifier is set up. Choose its port under Settings → Amplifier.
amp-settings = SETTINGS
amp-dock = DOCK
amp-put-amp-window-back-inside = Put the AMP window back inside the main window
amp-pop-out = POP OUT
amp-give-amp-window-window-own = Give the AMP window a window of its own, to move to another screen
amp-rig = , from the rig
amp-rig-flagged-reading = , the rig flagged its reading
amp-drv = , from DRV
amp-rig-s-swr = , from the rig's SWR
amp-ptt-keys-amplifier-radio-off = PTT keys the amplifier with the radio; OFF leaves it in bypass. Click to switch.
amp-switch-antenna-jack = Switch antenna jack
amp-band-down = Band down
amp-band-up = Band up
amp-tuner = TUNER
amp-tuned = Tuned
amp-tune = TUNE
amp-start-tune-then-key-steady = Start a tune — then key a steady carrier (CW, FM or RTTY). Press again to stop.
amp-put-tuner-line-bypass = Put the tuner in line, or bypass it
amp-tuning = Tuning…
amp-bottom = BOTTOM
amp-pick-any-reading-either-needle = Pick any reading for either needle. The top needle is white, the bottom blue.
amp-forward-power-measured-radio-amplifier = Forward power, measured by the radio: the amplifier reports none in OFF
amp-forward-power-taken-drv-amplifier = Forward power, taken from DRV: the amplifier reports none in OFF, and in bypass the radio's power goes straight through
amp-forward-power = Forward power
amp-reflected-power-measured-radio-amplifier = Reflected power, measured by the radio: the amplifier reports none in OFF
amp-reflected-power-worked-out-fwd = Reflected power, worked out from FWD and the SWR: the amplifier reports none in OFF
amp-reflected-power = Reflected power
amp-standing-wave-ratio-red-past = Standing wave ratio: red at or past the tune limit (Settings → General). Check the antenna
amp-standing-wave-ratio-measured-radio = Standing wave ratio, measured by the radio: the amplifier reports none in OFF
amp-standing-wave-ratio = Standing wave ratio
amp-drive-radio = Drive from the radio
amp-drain-supply = Drain supply
amp-drain-current = Drain current
amp-temp = TEMP
amp-heat-spreader-temperature-yellow-49 = Heat spreader temperature: yellow from 49 °C (120 °F), red above 65 °C (149 °F), off line above 70 °C (158 °F). Click for °C/°F.
amp-c = °C
amp-f = °F
amp-performance = PERFORMANCE
amp-numbers-hold-peaks-bars-are = numbers hold peaks · bars are live
amp-input-power = INPUT POWER
amp-supply-volts-drain-amps = Supply volts × drain amps
amp-efficiency = EFFICIENCY
amp-output-power-input-power = Output power ÷ input power
amp-gain = GAIN
amp-output-power-over-drive-decibels = Output power over drive, in decibels
amp-off-line-over-temperature = OFF LINE · over temperature
amp-connected-transmitting = CONNECTED · TRANSMITTING
amp-connected = CONNECTED
amp-connecting = Connecting…
amp-atu-bypassed = ATU BYPASSED
amp-no-inductance-needed = no inductance needed
amp-no-capacitance-needed = no capacitance needed
amp-c-out-antenna-side = C OUT · antenna side
amp-c-transmitter-side = C IN · transmitter side

## SSTV panel (src/app/panels/sstv.rs)

sstv-chunks-received-lit-still-missing = Chunks received (lit) and still missing (dark)
sstv-sstv = SSTV
sstv-auto = Auto
sstv-signal = Signal
sstv-station-sending-mode-sdroxide-does = A station is sending in a mode sdroxide does not have. The signal and the tuning are fine — there is simply no decoder for this one. Nothing to fix at your end.
sstv-listening = listening…
sstv-restart-rx = Restart RX
sstv-abandon-picture-being-received-listen = Abandon the picture being received and listen for the next header. For a transmission that started decoding in the wrong mode — the receiver is otherwise committed until that mode runs out.
sstv-callsign-last-station-sent-fsk = The callsign the last station sent as an FSK ID after its picture.
sstv-tx-slant = TX slant
sstv-transmit-clock-trim-ppm-remove = Transmit clock trim (ppm) to remove slant on the far-end decoder
sstv-reset-0-ppm = Reset to 0 ppm
sstv-fsk-id = FSK ID
sstv-send-your-callsign-tones-after = Send your callsign in tones after each picture — the identification SSTV repeaters and other programs read. Adds about 2.5 seconds, and sends nothing at all until you have set a callsign.
sstv-tx-lead = TX lead
sstv-silence-sent-after-keying-before = Silence sent after keying and before the picture's leader and VIS code. A decoder that misses any of that header shows no picture at all, so this covers the gap between asking a rig for PTT and it really being on the air. 0 for an SDR that keys instantly.
sstv-live = LIVE
sstv-waiting-picture = waiting for a picture…
sstv-waiting-signal = waiting for a signal…
sstv-no-low-audio = no / low audio
sstv-received = RECEIVED
sstv-reading-radio-s-pictures = Reading the radio's pictures…
sstv-decoded-pictures-collect-here = Decoded pictures collect here.
sstv-click-enlarge-right-click-delete = Click to enlarge · right-click to delete
sstv-delete-picture = Delete this picture
sstv-loading = loading…
sstv-transmit = TRANSMIT
sstv-click-edit-slot-s-message = Click to edit this slot's message · double-click to load an image
sstv-change-image = Change image…
sstv-load-image = Load image…
sstv-clear = Clear
sstv-empty-slot-s-picture-message = Empty this slot's picture (the message stays)
sstv-banner = Banner…
sstv-what-printed-across-top-every = What is printed across the top of every picture this station sends
sstv-fetching-slot-s-picture = Fetching this slot's picture…
sstv-load-image-into-slot = Load an image into this slot →
sstv-drawn-slot-s-image = Drawn on this slot's image
sstv-tx = {"   "}TX{"   "}
sstv-abort-tx = {" "}ABORT TX{" "}
sstv-delete-picture-store = Delete this picture from the store
sstv-received-image = Received image
sstv-no-longer-store = no longer in the store
sstv-loading-full-size = loading full size…
sstv-save-picture = Save picture…
sstv-save-copy-computer = Save a copy on this computer
sstv-delete-sure = Delete — sure?
sstv-click-again-delete-good = Click again to delete it for good
sstv-delete = Delete…
sstv-image-banner = Image banner
sstv-drawn-into-every-picture-station = Drawn into every picture this station transmits, over the slot's own message.
sstv-draw-banner = Draw the banner
sstv-substituted-when-picture-composed = Substituted when the picture is composed:
sstv-top-left = Top left
sstv-top-right = Top right
sstv-colours = Colours
sstv-strip-top-edge-fades-black = The strip, at its top edge — it fades to black, or to the gradient colour below it
sstv-both-texts = Both texts
sstv-gradient = Gradient
sstv-fade = fade to
sstv-fade-strip-top-colour-second = Fade the strip from its top colour to a second one instead of to black.
sstv-text-outline = Text outline
sstv-draw-outline-around-banner-text = Draw an outline around the banner text, so it stays readable over a busy or light strip.
sstv-text-gradient = Text gradient
sstv-fade-banner-text-colour-second = Fade the banner text from its colour to a second one across the width of the picture.
sstv-rainbow-text = Rainbow text
sstv-all-text-overriding-colours-above = all text, overriding the colours above
sstv-draw-every-text-picture-banner = Draw every text in the picture — banner and message — as a horizontal rainbow, ignoring the colours above.
sstv-message = Message
sstv-slot-message-s-text-colour = The slot message's text colour
sstv-outline = outline in
sstv-height = Height
sstv-how-tall-strip-pixels-transmitted = How tall the strip is, in pixels of the transmitted picture. The text is sized from it — an SSTV frame is only 320 pixels wide, so a taller banner is what makes it readable on the far end.
sstv-reset = Reset
sstv-back-callsign-left-version-right = Back to the callsign on the left and the version on the right
sstv-waiting-station-s-digital-mode = Waiting for the station's digital-mode settings to load…
sstv-rifp = RIFP
sstv-size = Size
sstv-calling-frequency-draft-names = The calling frequency the draft names
sstv-encode = Encode
sstv-how-picture-encoded-into-object = How the picture is encoded into the object RIFP carries. Auto tries each and sends the smallest.
sstv-try-every-encoding-send-smallest = Try every encoding, send the smallest (never lossy)
sstv-gray = Gray
sstv-grayscale-depth-rifp-s-raster = Grayscale depth. RIFP's raster is grayscale by definition — colour has no place in its manifest.
sstv-dither = Dither
sstv-diffuse-quantisation-error-worth-below = Diffuse quantisation error — worth it below 8 bits
sstv-caption = Caption
sstv-sent-manifest-content-hint-shown = Sent in the manifest as the content hint, and shown under the picture by receivers. Travels as text — it is not drawn into the image.
sstv-what-picture = What this picture is
sstv-repeat-data = Repeat data
sstv-send-every-data-frame-many = Send every data frame this many times. RIFP is one-way with no repair requests, so this is the only recovery a receiver gets.
sstv-chunk = Chunk
sstv-payload-octets-per-data-frame = Payload octets per data frame (the profile recommends 192)
sstv-valid-frames-frames-failed-crc = Valid frames, frames that failed CRC, and complete verified pictures
sstv-no-transfer-progress = no transfer in progress
sstv-forget-incomplete-transfer = Forget this incomplete transfer

## Memories and voice keyer windows (src/app/windows.rs)

windows-recall = Recall
windows-delete = Delete
windows-edit-name-frequency-mode-antenna = Edit the name, frequency, mode, antenna and repeater setup
windows-r = {" "}R
windows-repeater-shift-tone-1750-hz = The repeater shift, tone and 1750 Hz burst stored with this channel
windows-abandon-edit-escape = Abandon the edit (Escape)
windows-save = SAVE
windows-keep-changes-enter = Keep the changes (Enter)
windows-recall-channel-without-moving-antenna = Recall this channel without moving the antenna
windows-antenna-switch-when-channel-recalled = The antenna to switch to when this channel is recalled. — leaves whatever the radio is already on.
windows-shift = Shift
windows-tone = Tone
windows-invert = INVERT
windows-normal = NORMAL
windows-dcs-polarity = DCS polarity
windows-open-every-over-tone-burst = Open every over on the tone burst while this channel is recalled
windows-memories = Memories
windows-memory-name = memory name
windows-store = Store
windows-folder-name = folder name
windows-new-folder = New folder
windows-import = IMPORT
windows-read-channel-list-chirp-csv =
    Read a channel list from a CHIRP CSV file (.csv) — a repeater directory export, a marine or PMR channel table, or anything else CHIRP can write.

    Each channel brings its frequency, mode, repeater shift and CTCSS or DCS tone with it. Channels already on this list — same frequency, same mode — are skipped, so re-importing an updated directory adds what is new rather than doubling what is not.
windows-chirp-csv = CHIRP CSV
windows-export = EXPORT
windows-write-list-out-chirp-csv = Write this list out as a CHIRP CSV file, to load into a handheld or to keep as a backup.
windows-clear = CLEAR
windows-clear-sure = SURE?
windows-clear-hint = Delete every memory channel. Click once, then again within five seconds (it reads SURE?); folders stay.
windows-no-memories-yet = no memories yet
windows-drag-memory-onto-folder-file = drag a memory onto a folder to file it, below them to unfile it
windows-drop-here-unfile = drop here to unfile
windows-sort = Sort
windows-order-they-were-stored = In the order they were stored
windows-name-ignoring-case = By name, ignoring case
windows-frequency = By frequency
windows-band-then-frequency-inside-each = By band, then by frequency inside each
windows-descending-click-ascending = Descending — click for ascending
windows-ascending-click-descending = Ascending — click for descending
windows-delete-folder-memories-move-top = Delete folder — its memories move to the top level
windows-rename-folder = Rename folder
windows-empty-drop-memories-here = empty — drop memories here
windows-voice-keyer = Voice keyer
windows-rec-records-your-microphone-play = REC records from your microphone, PLAY lets you listen to what you recorded, TX puts it on the air — as does a numpad key, a MIDI pad, or rigctld's send_voice_mem.
windows-stop-store = Stop and store
windows-stop = STOP
windows-play = PLAY
windows-stop-listening = Stop listening
windows-nothing-recorded-slot = Nothing recorded in this slot
windows-not-while-transmitting = Not while transmitting
windows-listen-message-nothing-transmitted = Listen to this message — nothing is transmitted
windows-stop-transmitting = Stop transmitting
windows-tune-active-switch-off-first = TUNE is active — switch it off first
windows-transmit-message = Transmit this message
windows-erase-recording = Erase this recording
windows-rade-message-encoded-digital-voice = RADE: the message is encoded by the digital-voice codec, exactly as a live over would be.

## TX Audio window: messages with values

txaudio-rec-hint = Record a test phrase from the microphone — up to { $max } s, without keying the transmitter. Click again to stop. It is kept unprocessed, so PLAY can put the same phrase through every change you make.
txaudio-phrase-length = { $secs } s phrase
# $clip is txaudio-microphone-may-clipping-before-gets or empty.
txaudio-auto-set-done = Input gain set to { $gain } dB (voice peaks were { $loud } dBFS).{ $clip }
txaudio-listening = Listening… { $secs } s
txaudio-auto-set-hint = Talk normally for { $secs } s — the way you would on the air — and the input gain is set so your voice peaks around { $target } dBFS. Click again to cancel.
txaudio-eq-point = { $hz } Hz, { $db } dB — drag to move
txaudio-not-a-profile = Not a TX audio profile: { $error }
txaudio-imported = Imported "{ $name }".
txaudio-saved = Saved "{ $name }".
txaudio-deleted = Deleted "{ $name }".
txaudio-loaded-preset = Loaded the "{ $name }" starting point.

## Solar-system 3D view: messages with values

solar-age-offline = { $age } · offline
solar-radius-exaggeration = Earth/Moon radius exaggeration (max { $max }× at this moon-orbit scale)
solar-brightest-cell = brightest cell ≈ { $peak } paths · { $bands } { $bands ->
        [one] band
       *[other] bands
    }
solar-hits-in-hour = { $hits } in the hour
solar-passes-from = { $name } · PASSES FROM { $grid }
solar-elements-age = elements { $age } old · SGP4
solar-point-at = Point at { $az }° ({ $compass }), elevation { $el }°.
solar-published-as = published as { $name }
solar-fly-there = { $name } — ↵ to fly there
solar-sats-tracked = { $n } of { $total } tracked
solar-bodies-found = { $n } of { $total } bodies
solar-sats-and-bodies = { $n } of { $total } tracked · { $bodies } of { $bodies_total } bodies
solar-confidence-on-band = { $confidence } on { $band }
solar-kp-now = { $kp } now
solar-kp-in = { $kp } in { $hours } h
solar-valid-age = valid { $valid } · { $age } old
solar-full-bar = full bar = { $share } of the world
solar-own-paths-memory = where this station's own paths went · { $minutes } min memory
solar-n-storms = { $n } storms
solar-clouds-legend = CLOUDS { $time } · { $age } old · { $channels } · { $storms } · lightning simulated
solar-cme-eta = ETA { $eta } (+{ $hours } h)
solar-cme-arrived = arrival was { $eta } ({ $hours } h ago)
# $glancing and $estimated are solar-glancing / solar-direction-estimated or empty.
solar-cme-earth-directed = EARTH-DIRECTED CME { $time } · { $speed } km/s · { $when }{ $glancing }{ $estimated }

## Digital-mode setup dialogs: messages with values

setup-mode-setup = { $mode } Setup
setup-transmits-as = Transmits as { $call }
setup-decimal-degrees = Decimal degrees, negative for { $hemisphere }. Paste one straight off a map.
setup-js8-not-a-group-name = Not a group name: { $names }
setup-js8-callsign-same = same as My callsign ({ $call })
setup-js8-rule-preview = Sends: { $text }
setup-js8-rule-preview-hint = As it would go out to { $call }
setup-js8-rule-bad-group = { $group } is not a group name.

## AMP window: messages with values

amp-unreliable-at-swr = { $meters } UNRELIABLE AT THIS SWR
amp-lower-drive-first = Lower the drive first: { $drive } W would overdrive the amplifier in line (its limit is { $limit } W).
amp-live-value = live { $value } { $unit }
amp-tripped = TRIPPED{ $why }

## Digital-mode panels, shared: messages with values

panels-top-paths = top ≈ { $n } paths
panels-no-agreed-frequency = No frequency is agreed for { $mode } — open this to save the one you are on
panels-mode-frequencies-hint = The { $n } frequencies for { $mode } — picking one tunes the dial, and you can save your own
panels-mode-frequencies = { $mode } · frequencies
panels-global-convention = A global convention that the IARU Region { $region } band plan does not put narrow data on — check your own band plan before transmitting here.
panels-outside-data-segment = Amber: outside the Region { $region } data segment.
panels-save-dial = ＋ Save { $mhz } MHz
panels-save-dial-hint = Remember this dial under { $mode } and offer it here from now on. Saved on the station, so every screen attached to this radio has it.
panels-tr-period = { $period }-second T/R period
# $mark is panels-burst-mark while transmitting, otherwise empty.
panels-this-turn = This turn: { $into } s of { $slot }. Decodes land at the end of it, and that is when the next transmission may start.{ $mark }
panels-burst-mark = {" "}The mark is where the burst stops, { $secs } s in.
panels-tr-period-burst = { $slot }-second T/R period, { $burst } s burst

## Memories and voice keyer windows: messages with values

windows-n-channels = { $n } { $n ->
        [one] channel
       *[other] channels
    }
windows-slot-n = Slot { $n }
windows-record-from-mic = Record from the microphone (up to { $max } s)

## SSTV panel: messages with values

sstv-picture-too-big = That picture is { $size } MB — the limit is { $limit } MB.
sstv-auto-mode = Auto ({ $mode })
sstv-not-decoded = { $what } — not decoded
sstv-last = last: { $mode }
# $place is sstv-store-on-radio or sstv-store-in.
sstv-saved-where = Decoded pictures are saved { $place } and collect here.
sstv-older-in-store = { $n } older in the store
sstv-older-load-more = { $n } older — load more
sstv-delete-picture-from = Delete this picture { $place }
sstv-rifp-profile = { $profile } · ~{ $khz } kHz occupied · dial is the channel centre
sstv-rifp-too-wide = ⚠ { $profile } occupies ~{ $khz } kHz — too wide for a narrow-band segment
sstv-rifp-no-frequency = RIFP assigns no frequency, and sdroxide will transmit it wherever you tune. A { $khz } kHz channel only fits where wideband or FM operation is allowed — { $segments } — and not in a narrow-band segment, least of all on HF. Even inside those your own licence conditions may be narrower. You are the operator; check your own rules.
sstv-rifp-tx-frame = ● TX frame { $frame }/{ $frames } · { $secs } s left
sstv-rifp-sent-as = sent as { $encoding } · { $octets } octets
sstv-rifp-rx-stats = frames { $frames } · bad { $bad } · pictures { $pictures }
sstv-rifp-session-idle = session { $session } · idle { $secs } s
sstv-rifp-session-waiting = session { $session } · chunks held, still waiting for the manifest · idle { $secs } s
sstv-store-on-radio = on the radio, in { $dir }
sstv-store-in = in { $dir }

## decodes (src/app/panels/decodes.rs)

decodes-map-window = World map (FT8/FT4)
decodes-map-pop-out-tip = The world map: open it in a window of its own (as big as you drag it, on a compact layout too), and switch cities, who is talking to whom, and callsigns at the dots.
decodes-map-cities-tip = Cities on this map, on or off. The other maps keep to Settings → UI.
decodes-map-links-tip = Who is talking to whom: a dotted line between the two stations of every exchange heard in the last ten minutes, where both have sent a locator. Fades as the exchange ages.
decodes-map-labels-tip = Callsigns beside the station dots, fading with them.
decodes-map-pick-gone = No decode from this station in the list any more; it is still on the map from earlier.
decodes-map-menu = MAP ▼
decodes-map-open-window = Open in a window
decodes-map-item-cities = Cities
decodes-map-item-links = Who is talking to whom
decodes-map-item-labels = Callsigns at the dots
decodes-map-pick-new-entity = new entity
decodes-map-pick-new-grid = new grid
decodes-map-pick-new-call = new call
decodes-map-pick-worked = worked before
decodes-decodes = DECODES
decodes-adif = ADIF
decodes-sort = Sort
decodes-single-list = Single list
decodes-every-decode-one-list-newest = Every decode in one list, newest turn first, instead of odd/even turn blocks. The Sort chips then order the whole list rather than each turn, and each row carries its slot time — cyan for an even slot, gold for an odd one.
decodes-cq-only = CQ only
decodes-only-stations-calling-cq-only = Only stations calling CQ — and only the calls you may answer: a directed CQ (DX, EU, JA, POTA, TEST …) is listed when it names you and hidden when it names someone else.
decodes-new-only = New only
decodes-only-stations-would-new-entity = Only stations that would be new: entity, band-slot, grid, or call
decodes-auto-tx-frq = Auto TX FRQ
decodes-overridden-hold-tx-lift-hold = Overridden by Hold TX. Lift the hold to choose which way the transmit frequency moves.
decodes-pick-our-transmit-frequency-automatically = Pick our transmit frequency automatically: the quietest spot in the period we transmit in, rather than the frequency of whoever we are answering — they transmit in the other period, so theirs says nothing about who is there when we key. Off does NOT hold the frequency: it answers on the frequency of the station being called. To hold, use Hold TX.
decodes-hold-tx = Hold TX
decodes-pin-transmit-tone-where-nothing =
    Pin the transmit tone where it is. Nothing moves it: not answering a station, not the call queue, not calling CQ, not a click on a decode or on the waterfall. Turn it off to move, then on again.

    Changing band is the one exception, and it is your own act: the offset you last set on the new band comes back with it, so a 60 m figure is not carried onto 20 m and 20 m's is not carried onto 60 m.

    For where the licence is narrower than the band plan — on a UK 60 m dial of 5357 kHz the allocation ends at 5358.0, so the tone has to stay under 1000 Hz, and either automatic mover will walk out of the band between one over and the next.
decodes-type-transmit-offset-hz-then = Type the transmit offset in Hz, then press Enter. Where your licence is narrower than the band plan the whole range below the edge is yours to pick from, so nothing is suggested here.
decodes-clear-rx = {" "}CLEAR RX{" "}
decodes-empty-decode-list-nothing-air = Empty the decode list. Nothing that is on the air stops.
decodes-even = EVEN
decodes-text = TEXT
decodes-dxcc = DXCC
decodes-band = BAND
decodes-grid = GRID
decodes-dupe = DUPE
decodes-reply = REPLY
decodes-queued-click-remove = Queued — click to remove
decodes-work-station-after-current-one = Work this station after the current one
decodes-qsos-worked-since-sdroxide-was = QSOs worked since sdroxide was started
decodes-setup = ⚙ SETUP
decodes-qso-complete = ✓ QSO COMPLETE
decodes-contact-complete-log-held-here = The contact is complete and in the log. It is held here for a few minutes so the final message can be re-sent if the other station repeats theirs — nothing more is owed.
decodes-tx = ● TX
decodes-dxpedition-mode-transmit-frequency-held = DXpedition mode. The transmit frequency is held out of the Fox's half of the passband (below 1000 Hz) until the Fox answers.
decodes-watchdog = WATCHDOG
decodes-transmitting-stopped-no-reply-no = Transmitting stopped: no reply and no action for the watchdog period. Call CQ or pick a message to resume.
decodes-well-inside-tolerance = Well inside tolerance.
decodes-no-active-qso-pick-decode = no active QSO — pick a decode to reply, or Call CQ
decodes-ft8-engine-idle = FT8 engine idle
decodes-being-worked = being worked
decodes-clear = CLEAR
decodes-no-messages = — no messages —
decodes-call-cq = {"  "}CALL CQ{"  "}
decodes-stop-qso = {" "}STOP QSO{" "}
decodes-stop-tx = {" "}STOP TX{" "}
decodes-free-text-13-chars = free text (13 chars)
decodes-send = SEND

## sat (src/app/sat.rs)

sat-pass-profile = PASS PROFILE
sat-satellite = SATELLITE
sat-satellites = SATELLITES
sat-passes = PASSES
sat-set-your-grid-passes-settings = Set your grid for passes (Settings ▸ General).
sat-next = NEXT
sat-peak = PEAK ≥
sat-all-satellites = all satellites
sat-off-only-satellites-frequencies-file = Off: only satellites with frequencies on file — the ones you can work — and any you pasted in yourself. On: every satellite in the list, which can be thousands with a subscription and takes a while to search.
sat-no-passes-window-lower-peak = No passes in this window. Lower the peak, or widen the window.
sat-length = LENGTH
sat-peak-2 = PEAK
sat-from-to = FROM → TO
sat-up-now = ● up now
sat-unlock = UNLOCK
sat-release-lock-dial-stays-where = Release the lock; the dial stays where it is
sat-azimuth = AZIMUTH
sat-elevation = ELEVATION
sat-range = RANGE
sat-rate = RATE
sat-downlink = DOWNLINK
sat-doppler-rx = DOPPLER RX
sat-uplink = UPLINK
sat-doppler-tx = DOPPLER TX
sat-no-pass-inside-next-48 = No pass inside the next 48 hours from your QTH.
sat-elements-stale-corrections-suspended-until = ⚠ Elements stale — corrections suspended until a TLE refresh
sat-doppler-correction-needs-iq-front = Doppler correction needs an IQ front end — a CAT rig tracks only
sat-rotator-not-connected = Rotator: not connected
sat-doppler = DOPPLER
sat-apply-computed-correction-rx-tx = Apply the computed correction to RX and TX
sat-rotator = ROTATOR
sat-steer-antenna-through-rotctld-client = Steer the antenna through the rotctld client (Settings ▸ Servers)
sat-name-norad = name or NORAD
sat-set-your-grid-passes-settings-2 = set your grid for passes (Settings ▸ General)
sat-norad = NORAD
sat-pass = PASS
sat-always-visible = always visible
sat-not-here = not from here
sat-no-elements = no elements
sat-pick-satellite-see-links-lock = Pick a satellite to see its links and lock on.
sat-no-frequencies-file-one-add = No frequencies on file for this one — add them in Settings ▸ TLE.
sat-link = LINK
sat-downlink-mhz = DOWNLINK (MHz)
sat-uplink-mhz = UPLINK (MHz)
sat-mode = MODE
sat-tune = TUNE
sat-set-dial-mode-link-no = Set the dial and mode to this link — no lock, no correction
sat-locked = {" "}LOCKED{" "}
sat-lock = {" "}LOCK ON{" "}
sat-track-satellite-doppler-correction-rx = Track this satellite: Doppler correction on RX and TX, transponder-mapped uplink, and the 3D view follows it

## wefax (src/app/panels/wefax.rs)

wefax-wefax = WEFAX
wefax-stop = {" "}■ STOP{" "}
wefax-end-chart-now-save-what = End the chart now and save what has arrived
wefax-start = {" "}● START{" "}
wefax-start-chart-now-without-waiting = Start a chart now, without waiting for a start tone
wefax-phasing = phasing…
wefax-subcarrier-offset-1900-hz-tune = Subcarrier offset from 1900 Hz. Tune for roughly zero: the fax carrier is 1500 Hz black to 2300 Hz white, and a receiver a few hundred hertz off clips the picture to solid black or solid white.
wefax-auto-start = AUTO START
wefax-begin-chart-when-300-hz = Begin a chart when the 300 Hz (IOC 576) or 675 Hz start tone is heard — and end the one in progress and phase the new one when the next transmission arrives, whether or not a stop tone came first.
wefax-auto-stop = AUTO STOP
wefax-end-450-hz-stop-tone = End it on the 450 Hz stop tone. Turn off to keep recording through a station that sends several charts back to back.
wefax-phase = PHASE
wefax-slant = SLANT
wefax-sample-clock-trim-if-chart = Sample-clock trim. If the chart leans to the left, increase this; to the right, decrease it. A sound card a hundred ppm off walks a quarter-hour chart most of a line sideways.
wefax-view = VIEW
wefax-scale-chart-panel-width = Scale the chart to the panel width
wefax-whole = WHOLE
wefax-shrink-chart-until-all-view = Shrink the chart until all of it is in view at once
wefax-half-size = Half size
wefax-one-screen-pixel-per-fax = One screen pixel per fax pixel — scroll for detail
wefax-twice-size-scroll-move-around = Twice size; scroll to move around the chart
wefax-height = HEIGHT
wefax-stretch-picture-vertically-chart-came = Stretch the picture vertically. A chart that came out squashed or stretched is usually being decoded at the wrong line rate — this makes it readable, and the LPM chips fix it properly. Double-click to type a value.
wefax-reset = RESET
wefax-back-picture-s-own-proportions = Back to the picture's own proportions
wefax-follow = FOLLOW
wefax-keep-newest-lines-view-scrolling = Keep the newest lines in view. Scrolling up turns this off so you can read what has already arrived; scrolling back to the bottom turns it on again.
wefax-tune-fax-schedule-usb-wait = Tune a fax schedule in USB and wait for a start tone, or press START to begin mid-chart.
wefax-saved = SAVED
wefax-path = PATH
wefax-reading-radio-s-charts = Reading the radio's charts…
wefax-finished-charts-collect-here = Finished charts collect here.
wefax-delete-chart = Delete this chart
wefax-loading = loading…
wefax-stations = STATIONS
wefax-broadcast-radiofax-schedules-picking-one = Broadcast radiofax schedules — picking one tunes the dial
wefax-frequencies-are-published-carrier-dial = Frequencies are the published carrier; the dial goes 1.9 kHz below it, which is done for you. Schedules change and stations close — treat this as where to start looking, not a timetable.
wefax-delete-chart-store = Delete this chart from the store
wefax-newer = ◀ NEWER
wefax-chart-received-after-one = The chart received after this one
wefax-older = OLDER ▶
wefax-chart-received-before-one = The chart received before this one
wefax-no-longer-store = no longer in the store
wefax-loading-full-size = loading full size…
wefax-save = SAVE
wefax-save-copy-chart-computer = Save a copy of this chart on this computer
wefax-sure = SURE?
wefax-click-again-delete-chart-good = Click again to delete this chart for good
wefax-delete = DELETE
wefax-file-chart-was-saved = The file this chart was saved as

## aprs (src/app/panels/aprs.rs)

aprs-starting-aprs-modem = starting the APRS modem…
aprs-aprs = APRS
aprs-no-callsign = no callsign
aprs-nothing-will-transmitted-no-beacon = Nothing will be transmitted — no beacon, no message, not even an acknowledgement — until this station has a callsign. Set one under Settings → General, or an APRS-specific one with its SSID under SETUP.
aprs-no-position = no position
aprs-beacon-needs-somewhere-report-fill = A beacon needs somewhere to report. Fill in your locator under Settings → General, or give coordinates under SETUP.
aprs-setup = SETUP
aprs-callsign-symbol-digipeater-path-position = Callsign, symbol, digipeater path, position and beacon
aprs-how-often-beacon-your-position = How often to beacon your position, unattended. `off` — the default — never beacons: selecting a mode must not put a station on the air. Thirty minutes is the convention for a fixed station, oftener for a moving one, and every beacon is somebody else's channel time.
aprs-beacon = {" "}BEACON{" "}
aprs-station-has-no-callsign-see = This station has no callsign — see the warning to the left.
aprs-station-has-no-position-report = This station has no position to report — see the warning to the left.
aprs-send-one-position-report-now = Send one position report now, without waiting for the interval. It waits for the channel to be clear like everything else.
aprs-stations = STATIONS
aprs-nothing-heard-yet = nothing heard yet
aprs-heard-direct = heard direct
aprs-two-characters-station-sent-say = The two characters the station sent to say what it is
aprs-killed = KILLED
aprs-station-put-object-here-has = The station that put this object here has cancelled it
aprs-sender-deliberately-reported-square-rather = The sender deliberately reported a square rather than a point. The map draws the square.
aprs-digipeaters-last-frame-came-through = The digipeaters the last frame came through. A `*` marks one that actually repeated it.
aprs-message = {" "}MESSAGE{" "}
aprs-address-message-box-station = Address the message box to this station
aprs-center = {" "}CENTER{" "}
aprs-put-station-middle-map-double = Put this station in the middle of the map. Double-click the map to hand the view back to the automatic fit.
aprs-channel = CHANNEL
aprs-messages = MESSAGES
aprs-frames-arrived-failed-their-check = Frames that arrived and failed their check sequence — a collision, a fade, or a signal too weak to read. A count rising with nothing decoding means the modem *is* hearing the channel: check the level meter, and open the radio's squelch.
aprs-frames-read-cleanly-off-channel = Frames read cleanly off the channel that were not APRS — somebody else's packet session, or a format this build does not decode.
aprs-every-frame-channel-arrived-other = Every frame on the channel, as it arrived — other people's traffic included, and anything the decoder could not read.
aprs-nothing-channel-yet = nothing on the channel yet
aprs-no-messages = No messages.
aprs-waiting-channel = waiting for the channel
aprs-sent-retried-still-waiting-acknowledged = sent and retried, still waiting to be acknowledged
aprs-air-waiting-acknowledged = on the air, waiting to be acknowledged
aprs-acknowledged-far-end = acknowledged by the far end
aprs-refused-far-end-not-retried = refused by the far end — not retried
aprs-no-ack = no ack
aprs-no-answer-after-every-retry = no answer after every retry
aprs-send = {" "}SEND{" "}
aprs-station-has-no-callsign-so = This station has no callsign, so nothing can be transmitted. Set one under Settings → General, or an APRS-specific one with its SSID under SETUP.
aprs-needs-station-address-something-say = Needs a station to address and something to say.
aprs-send-keep-retrying-until-far = Send it, and keep retrying until the far end acknowledges. Messages are the one thing on this channel that is answered.
aprs-busy = BUSY
aprs-stations-heard-still-inside-window = Stations heard and still inside the window set in APRS Setup, and whether another station is on the channel right now. Nothing will key while it is busy.
aprs-tx = ● TX
aprs-over-progress-frames-waiting-channel = The over in progress, the frames waiting for the channel to clear, or the time until the next scheduled beacon.
aprs-audio-reaching-modem-flat-means = Audio reaching the modem. Flat means nothing is arriving at all — check that the radio's data output is the one sdroxide is listening to. It lights up green while the modem hears a carrier.

## vdl2 (src/app/panels/vdl2.rs)

vdl2-starting-vdl2-decoder = starting the VDL2 decoder…
vdl2-aircraft-datalink = aircraft datalink
vdl2-tune-middle-vdl2-group-decoder = Tune to the middle of the VDL2 group. The decoder's own window slides from there to take in as many of the fourteen channels as the receiver can reach.
vdl2-setup = SETUP
vdl2-which-channels-listen-how-hard = Which channels to listen on, how hard a burst has to be, and how much log to keep
vdl2-save-vdl2-message-log-file = Save the VDL2 message log to a file
vdl2-vdl2-channel = VDL2 channel
vdl2-messages = MESSAGES
vdl2-address-flight-registration-label = address, flight, registration or label
vdl2-held = HELD
vdl2-hold = HOLD
vdl2-hold-message-log-where-so = Hold the message log where it is, so one can be read without the next transmission pushing it up the screen. The decoder keeps running and the counters keep moving; letting go shows everything that arrived meanwhile.
vdl2-nothing-decoded-yet = nothing decoded yet
vdl2-nothing-matches-filter = nothing matches the filter
vdl2-stations = STATIONS
vdl2-nobody-heard-yet = nobody heard yet
vdl2-vdl2-setup = VDL2 Setup
vdl2-channels = Channels
vdl2-every-25-khz-slot-136 = Every 25 kHz slot from 136.650 to 136.975 MHz. One downconverter each, all inside the same receiver window. Switching one off saves a little processor time; it does not make the others any more sensitive.
vdl2-burst-threshold = Burst threshold
vdl2-above-each-channel-s-own = above each channel's own noise floor
vdl2-lower-catches-weaker-transmissions-costs = Lower catches weaker transmissions and costs processor time on noise; higher misses them. The floor is learned per channel and a change here does not throw it away.
vdl2-keep-log = Keep in the log
vdl2-messages-2 = {" "}messages
vdl2-track-most = Track at most
vdl2-stations-2 = {" "}stations
vdl2-longest-silent-go-first = the longest silent go first
vdl2-forget-station-after = Forget a station after
vdl2-nothing-heard = with nothing heard from it
vdl2-show-unread-payloads = Show unread payloads
vdl2-hex = as hex
vdl2-frames-carrying-x-25-clnp = Frames carrying X.25, CLNP or the datalink applications above them. SDRoxide names them and shows the bytes rather than reading them, and hiding them would hide how much of the traffic that is.
vdl2-registration = Registration
vdl2-flight = Flight
vdl2-label = Label
vdl2-block = Block
vdl2-sequence = Sequence
vdl2-mode = Mode
vdl2-more-blocks-follow = more blocks follow
vdl2-check = Check
vdl2-message-crc-good = message CRC good
vdl2-message-crc-not-checked = message CRC not checked
vdl2-parity = Parity
vdl2-link-control-no-payload = link control, no payload
vdl2-name = NAME
vdl2-addr = ADDR
vdl2-type = TYPE
vdl2-msgs = MSGS

## cw (src/app/panels/cw.rs)

cw-tone-being-copied-tone-transmitted = The tone being copied, and the tone transmitted — in CW they are the same frequency. Click the waterfall to move it onto a signal.
cw-down-10-hz = Down 10 Hz
cw-up-10-hz = Up 10 Hz
cw-qrg-main-readout-tuning-line = QRG: the main readout and the tuning line are on the frequency being worked. Click for the dial instead, a sidetone pitch below it — what most radios show.
cw-put-main-readout-tuning-line =
    Put the main readout and the tuning line on the signal rather than on the dial, so the frequency shown is the one both operators would quote and the tuning line sits in the middle of the passband. Tuning is unchanged; only the numbers move.

    Clicking a signal lands it on the cursor only as closely as the click step allows — Controls → click-to-tune rounding, 10 Hz by default. A coarse step leaves the signal off the pitch by up to half of it, and the readout will say so.
cw-sending-speed-read-off-signal = Sending speed read off the signal
cw-signal-noise-500-hz-same = Signal to noise in 500 Hz — the same figure a report quotes
cw-how-far-off-cursor-signal = How far off the cursor the signal actually is. The decoder follows it; the passband does not, so nudge the cursor if it grows.
cw-listening = — listening —
cw-tx = ● TX
cw-trainer = TRAINER
cw-learn-morse-read-characters-words = Learn Morse: read characters and words, play them at your own speed, and a Koch drill that adds a character at a time. It plays here, through this computer's speakers — nothing keys the radio.
cw-nothing-copied-yet = — nothing copied yet —
cw-key-send-characters-you-send = Key to send — the characters you send appear here.
cw-type-line-return-sends = Type a line, Return sends it…
cw-type-here-send = Type here to send…
cw-key = {" "}KEY ●{" "}
cw-key-2 = {" "}KEY{" "}
cw-hold-key-bound-cw-straight = Hold the key bound to CW straight key — Space by default, and any key you like in Settings → Controls — as a straight key: down while it is held, up on release, instead of typing text. The transmit box is locked while it is on, and the whole keyer is handed to the key: whatever text was queued is dropped.
cw-radio-sends-own-keyer-text =
    This radio sends from its own keyer: the text goes over the control port and the rig times the elements, so there is nothing between the straight key and the air for a hand to drive.

    To hand-key it, set CW keying to "Sound card (MCW)" in Settings → Radio. The rig is then held on a sideband and the keyer's own sidetone is transmitted as audio, which is the route the straight key drives.
cw-watchdog = WATCHDOG
cw-straight-key-was-held-down = The straight key was held down too long — a lost key-up rather than a hand — so the carrier was dropped and transmit switched off. Press the key again to carry on.
cw-tx-2 = {"  "}TX ON{"  "}
cw-tx-3 = {"   "}TX{"   "}
cw-send-what-box-now-without = Send what is in the box now, without waiting for Return
cw-hold-key-down-between-characters = Hold the key down between characters, so nothing typed waits
cw-call-cq = {" "}CALL CQ{" "}
cw-nocall = NOCALL
cw-clear = {" "}CLEAR{" "}
cw-stop-sending-drop-whatever-has = Stop sending and drop whatever has not gone out. With the straight key on, the read-back over what you keyed goes with it.
cw-sidetone = SIDETONE
cw-play-keyed-tone-through-computer = Play the keyed tone through this computer's speakers as well as sending it, so you hear what you are sending. On `Sound card (MCW)` the tone goes out to the rig and nowhere else, so without this you send in silence. Off where the rig's own monitor or an off-air copy already does the job.
cw-hold-what-typed-until-return = Hold what is typed until Return, then send the line in one piece instead of keying each character as it is typed. Worth having on a transceiver that keys itself from text, where every hand-off to its keyer is another transmit-receive cycle.
cw-your-own-message-buttons-contest = Your own message buttons — a contest exchange, a name-and-QTH reply, TNX 73 GL. Each sends its whole text in one go, and F2–F10 press the first nine. They travel with the station's configuration, so a remote client has them too.
cw-cw-messages = CW MESSAGES
cw-clear-rx = {" "}CLEAR RX{" "}
cw-empty-receive-window-straight-key = Empty the receive window and the straight key's read-back. Nothing that is on the air stops.
cw-lock = LOCK
cw-decode-speed-set-here-instead = Decode at the speed set here instead of reading it off the signal. Helps a signal too weak for the speed search to settle.
cw-farnsworth-send-characters-full-speed = Farnsworth: send the characters at full speed and stretch only the gaps between them, so they are heard at the right rhythm but arrive slowly enough to write down.
cw-off-normal-spacing = Off — normal spacing
cw-keying-speed = Keying speed
cw-idle-0 = IDLE 0
cw-how-long-transmit-held-after = How long transmit is held after the last character, or after the straight key is let go, before the carrier drops. Longer bridges a slow typist's pauses; shorter gets off the frequency sooner; Off drops it as soon as everything queued has gone out.
cw-off-drop-once = Off — drop at once

## textmodem (src/app/panels/text_modem.rs)

textmodem-tune-down-10-hz = Tune down 10 Hz
textmodem-tune-up-10-hz = Tune up 10 Hz
textmodem-tx = ● TX
textmodem-listening = — listening —
textmodem-type-line-return-sends = Type a line, Return sends it…
textmodem-type-here-transmit = Type here to transmit…
textmodem-tx-2 = {"  "}TX ON{"  "}
textmodem-tx-3 = {"   "}TX{"   "}
textmodem-call-cq = {" "}CALL CQ{" "}
textmodem-nocall = NOCALL
textmodem-clear = {" "}CLEAR{" "}
textmodem-hold-what-typed-until-return = Hold what is typed until Return, then send the line in one piece instead of streaming each character onto the air as it is typed. Lets a line be read back and corrected before any of it goes out.
textmodem-your-own-message-buttons-working = Your own message buttons — working conditions, the weather, a standard reply. Each sends its whole text in one go, and F2–F10 press the first nine. They travel with the station's configuration, so a remote client has them too.
textmodem-messages = MESSAGES
textmodem-hell = HELL
textmodem-contrast = Contrast
textmodem-harder-softer-dots-redraws-whole = Harder or softer dots — redraws the whole strip
textmodem-width = Width
textmodem-t-2row = {" "}2ROW{" "}
textmodem-draw-every-column-twice-stacked = Draw every column twice, stacked. Hell has no vertical sync, so this keeps one complete copy of the text readable whatever the phase.
textmodem-rev = {" "}REV{" "}
textmodem-reverse-video-light-dots-dark = Reverse video — light dots on dark paper
textmodem-clear-rx = {" "}CLEAR RX{" "}
textmodem-hold-channel-idle-sends-blank = Hold the channel: idle sends blank paper, so the strip keeps scrolling
textmodem-hold-what-typed-until-return-2 = Hold what is typed until Return, then send the line in one piece instead of painting each character onto the strip as it is typed. A correction typed live is already on the paper; one made before Return never was.
textmodem-shift = Shift
textmodem-shift-5-hz = Shift −5 Hz
textmodem-shift-5-hz-2 = Shift +5 Hz
textmodem-baud = Baud
textmodem-reverse-swap-mark-space-needed = Reverse: swap mark and space. Needed when the signal is received on the opposite sideband to the one it was sent on, which inverts the tones.
textmodem-track-tuning-error-automatically-instead = Track tuning error automatically instead of staying pinned to the cursor.
textmodem-tones = Tones
textmodem-mode = Mode
textmodem-off-keyed = on/off keyed

## ais (src/app/panels/ais.rs)

ais-starting-ais-decoder = starting the AIS decoder…
ais-tune-between-two-ais-channels = Tune between the two AIS channels — 161.975 (AIS 1, marine 87B) and 162.025 (AIS 2, 88B). The dial goes in the middle because nothing transmits there, so a zero-IF receiver's DC spike lands on neither.
ais-setup = SETUP
ais-channels-timeouts-trail-length-how = Channels, timeouts, trail length and how far ahead the vectors reach
ais-vessels = VESSELS
ais-nothing-heard-yet = nothing heard yet
ais-virtual-nothing-there = virtual — nothing is there
ais-position-stale = position stale
ais-search-rescue-transmitter-man-overboard = a search-and-rescue transmitter, man-overboard beacon or AIS EPIRB
ais-call-sign = call sign
ais-station-time = station time
ais-dgnss = {" "}(DGNSS)
ais-first-heard = first heard
ais-center = CENTER
ais-put-vessel-middle-chart = Put this vessel in the middle of the chart
ais-last-message-station-nmea-sentences = The last message from this station, as the NMEA sentences a receiver would put on the wire. Paste it into any other AIS decoder to check what sdroxide made of it.
ais-ais-setup = AIS Setup
ais-channels = Channels
ais-drop-chart-after = Drop from chart after
ais-without-position-report = without a position report
ais-five-minutes-default-not-ads = Five minutes by default, not ADS-B's ten seconds: a vessel at anchor reports once every three minutes, and a shorter window would blank most of a harbour between two perfectly good reports. Past it the ship comes off the chart and its row greys — it is not faded, because a dim symbol at a stale position is still a claim about where a ship is, in the same ink as the true ones.
ais-drop-list-after = Drop from list after
ais-nothing-heard-all = with nothing heard at all
ais-message-carrying-ship-s-name = The message carrying a ship's name comes round every six minutes, so a short list window keeps throwing vessels away just before they say what they are called.
ais-trail-length = Trail length
ais-history-behind-each-target = of history behind each target
ais-minutes-rather-than-points-because = In minutes rather than in points, because AIS reporting rates span two orders of magnitude: a fixed count would be eighty seconds of a ferry and two hours of an anchored tanker, drawn identically. Zero switches trails off.
ais-speed-vector = Speed vector
ais-how-far-ahead-vector-reaches = how far ahead the vector reaches
ais-slot-threshold = Slot threshold
ais-above-channel-s-learned-noise = above the channel's learned noise floor
ais-track-most = Track at most
ais-vessels-2 = {" "}vessels
ais-ais-receive-only-here-always = AIS is receive-only here and always will be: it is a safety-of-life service, and putting false vessel traffic on it is not something a licence covers.
ais-fill-my-grid-digimode-setup = Fill in My grid in the digimode setup for ranges and bearings, and so the chart frames itself around where you are rather than around whatever is furthest away.
ais-name = NAME
ais-mmsi = MMSI
ais-type = TYPE

## qo100 (src/app/qo100.rs)

qo100-receiver-cannot-reach-10489-750 = This receiver cannot reach 10489.750 MHz on its own — set up an LNB/converter offset first (Settings ▸ Radio)
qo100-run-fast-spectral-tracker-every = Run the fast spectral tracker: every second, look in the parking window for the beacon's two symmetric lobes and report where the carrier sits
qo100-telemetry = TELEMETRY
qo100-also-run-ao-40-uncoded = Also run the AO-40 uncoded frame decoder — sync word, CRC and the telemetry text — with a step-by-step readout (carrier → sync → CRC) of how far each pass gets
qo100-auto = AUTO
qo100-auto-correct-let-tracker-adjust = Auto-correct: let the tracker adjust the converter/LNB offset by itself — a slow closed loop that, on a clean and steady estimate, nudges the beacon back onto 10489.750 MHz and holds it there as the LNB drifts. Reopens the front end each time it acts.
qo100-narrower-search-smaller-slice = Narrower — search a smaller slice
qo100-wider-when-beacon-isn-t = Wider — for when the beacon isn't found at the current width
qo100-clear-mark = clear mark
qo100-drop-hand-placed-beacon-mark = Drop the hand-placed beacon mark and go back to the decoder's own reading
qo100-lo = lo −
qo100-lo-2 = lo +
qo100-hi = hi −
qo100-hi-2 = hi +
qo100-radio-s-own-tuning-range = This radio's own tuning range doesn't reach 10489.750 MHz — it needs an LNB/converter offset (Settings ▸ Radio ▸ Converter) before this window can do anything.
qo100-open-settings-radio = Open Settings ▸ Radio
qo100-tune-10489-750-mhz = Tune to 10489.750 MHz
qo100-tip-if-you-can-see = tip: if you can see the beacon in the strip but it won't lock, double-click it — that marks it as 10489.750 MHz and lets APPLY correct to it
qo100-tracker = TRACKER
qo100-no-twin-lobe-shape-parking = no twin-lobe shape in the parking window
qo100-shape = SHAPE
qo100-drift = DRIFT
qo100-need-longer-run-estimates = — (need a longer run of estimates)
qo100-cycles = CYCLES
qo100-receiver = RECEIVER
qo100-target = TARGET
qo100-measured = MEASURED
qo100-clicked = {"  "}(clicked)
qo100-tracker-2 = {"  "}(tracker)
qo100-not-locked-yet = not locked yet
qo100-converter-offset = CONVERTER OFFSET
qo100-decode = DECODE
qo100-decode-filling-first-frame-buffer = DECODE — filling the first frame buffer…
qo100-decode-off-turn-telemetry-chip = DECODE — off. Turn on the TELEMETRY chip (top row) for the carrier → sync → CRC readout.
qo100-apply-correction = {" "}APPLY CORRECTION{" "}
qo100-write-converter-lnb-offset-puts = Write the converter/LNB offset that puts the beacon where you clicked onto 10489.750 MHz, and reopen the receiver — the same brief interruption Settings ▸ Radio ▸ Apply makes
qo100-write-converter-lnb-offset-puts-2 = Write the converter/LNB offset that puts the tracker's estimate onto 10489.750 MHz, and reopen the receiver — the same brief interruption Settings ▸ Radio ▸ Apply makes
qo100-waiting-second-crc-valid-frame = Waiting for a second CRC-valid frame before offering to write this — one lock alone could be a chance match
qo100-write-corrected-converter-lnb-offset = Write the corrected converter/LNB offset and reopen the receiver — a brief interruption, the same one Settings ▸ Radio ▸ Apply makes
qo100-decoder-runs-receiving-station-readout = decoder runs on the receiving station — its readout is not sent to remote clients
qo100-starting = starting…
qo100-searching-first-window-fills-after = searching — the first window fills after about 24 s, then repeats

## js8 (src/app/panels/js8.rs)

js8-multi = MULTI
js8-decode-every-js8-speed-not = Decode every JS8 speed, not only the one you transmit at. The four speeds share the sub-band and are four different waveforms, so without this a station on another speed is invisible — and an exchange between two speeds cannot happen at all. Costs about four times the receive CPU.
js8-hb-auto = HB AUTO
js8-turbo-does-not-beacon-local = Turbo does not beacon — it is the local and VHF speed
js8-beacon-your-callsign-grid-every = Beacon your callsign and grid every 15 minutes
js8-until-next-heartbeat = Until the next heartbeat
js8-setup = ⚙ SETUP
js8-tx = ● TX
js8-heard = HEARD
js8-nothing-heard-yet = — nothing heard yet —
js8-dxcc = DXCC
js8-band = BAND
js8-grid = GRID
js8-dupe = DUPE
js8-reply = REPLY
js8-no-messages = — no messages —
js8-cq = {" "}CQ{" "}
js8-hb = {" "}HB{" "}
js8-clear = {" "}CLEAR TO{" "}
js8-forget-selected-station-composer-goes = Forget the selected station — the composer goes back to @ALLCALL
js8-already-addressing-allcall = Already addressing @ALLCALL
js8-stop = {" "}STOP{" "}
js8-send = {" "}SEND{" "}
js8-message = Message…
js8-auto-resp = AUTO-RESP
js8-auto-resp-no-rules = No auto-response rules yet — click to write one in SETUP
js8-auto-pending = AUTO
js8-auto-pending-text-hint = The reply, as it will be sent
js8-auto-pending-count-hint = Sends made / sends planned. ABORT or sending anything yourself cancels the rest.

## logbook (src/app/logbook.rs)

palette-hint = Go to… a mode, a band, a window, a setting, a memory, or a frequency (14.074)
palette-nothing = nothing by that name
palette-group-window = Window
palette-group-settings = Settings
palette-group-memory = Memory
palette-group-tune = Tune
palette-tune-to = Tune to { $mhz } MHz
palette-win-bands = Band conditions
palette-win-awards = Awards
palette-win-windows = Window list
logbook-logbook = LOGBOOK
logbook-new-entry = + NEW ENTRY
logbook-adif = ADIF
logbook-import = IMPORT
logbook-import-qsos-adif-adi-adif = Import QSOs from an ADIF (.adi or .adif) file
logbook-new-qso = NEW QSO
logbook-edit-qso = EDIT QSO
logbook-call = Call
logbook-lookup = LOOKUP
logbook-look-up-name-qth-grid = Look up name / QTH / grid
logbook-grid = Grid
logbook-freq-mhz = Freq MHz
logbook-mode = Mode
logbook-rst-sent = RST sent
logbook-rst-rcvd = RST rcvd
logbook-name = Name
logbook-state = State
logbook-country = Country
logbook-date-utc = Date UTC
logbook-time = Time
logbook-pwr-w = Pwr W
logbook-contest = Contest
logbook-s-sent = S# sent
logbook-s-rcvd = S# rcvd
logbook-comment = Comment
logbook-save = {" "}SAVE{" "}
logbook-cancel = CANCEL
logbook-lotw = LoTW ✓
logbook-eqsl = eQSL ✓
logbook-card = card ✓
logbook-lotw-2 = LoTW ↑
logbook-eqsl-2 = eQSL ↑
logbook-qrz = QRZ ↑
logbook-hamqth = HamQTH ↑
logbook-club-log = Club Log ↑
logbook-wavelog = Wavelog ↑
logbook-delete-entry = Delete this entry
logbook-edit = EDIT
logbook-upload-qso-configured-logs = Upload this QSO to configured logs
logbook-no-qsos-yet-run-ft8 = no QSOs yet — run FT8/FT4 or add a manual entry

## scanner (src/app/scanner.rs)

scanner-scanner = Scanner
scanner-work-through-stored-memory-channels = Work through the stored memory channels
scanner-work-through-slice-band-channel = Work through a slice of a band on a channel grid
scanner-stop = STOP
scanner-start = START
scanner-from = From
scanner-step = Step
scanner-mode = Mode
scanner-range-empty-high-edge-has = That range is empty — the high edge has to be at least one channel above the low one.
scanner-skip-while-holding-pass-over = SKIP while it is holding to pass over that channel from now on
scanner-skipping = Skipping
scanner-clear = CLEAR
scanner-stop-passing-over-any-them = Stop passing over any of them
scanner-stop-passing-over-channel = Stop passing over this channel
scanner-no-memory-channels-stored-yet = No memory channels stored yet — store some in the MEM window.
scanner-skip-channel-pass-over = SKIP a channel to pass over it
scanner-skip = SKIP
scanner-look-all-channels-fall-inside = Look for all the channels that fall inside one receiver window on the same transform the panadapter is made from, and only tune to the ones something is on. A list on one band then costs one tune a lap however long it is, instead of a settling time per channel. The scan still listens on each candidate before stopping, so what stops it is unchanged — but the sweep measures through the FFT rather than through the receiver's filter, so check the threshold if it starts stopping on nothing.
scanner-radio-hands-over-demodulated-audio = This radio hands over demodulated audio and has no spectrum of its own to search, so its memory scan visits every channel either way.
scanner-fast = FAST
scanner-read-list-off-spectrum-instead = read the list off the spectrum instead of visiting every channel
scanner-folders = Folders
scanner-scan-every-folder-every-folder = Scan every folder, and every folder made from now on. Pick folders instead to scan only those.
scanner-unfiled = Unfiled
scanner-nothing-filed-under-folders-you = Nothing is filed under the folders you picked.
scanner-stops = Stops at
scanner-use-receiver-s-own-squelch = Use the receiver's own squelch, so the scan stops exactly where the audio would open
scanner-squelch-off-scan-will-stop = squelch is off — the scan will stop on the first thing it looks at
scanner-channel-power-signal-has-reach = Channel power a signal has to reach to stop the scan
scanner-listens = Listens for
scanner-how-long-stay-candidate-before = How long to stay on a candidate before judging it. Below about a tenth of a second the level meter has not settled and weak signals get missed
scanner-resumes = Resumes
scanner-carry-once-signal-drops = Carry on once the signal drops
scanner-carry-after-fixed-time-regardless = Carry on after a fixed time, regardless
scanner-stay-until-you-press-next = Stay until you press NEXT
scanner-how-long-stay = How long to stay
scanner-how-long-wait-after-signal = How long to wait after the signal drops
scanner-move-don-t-stop-channel = Move on, and don't stop on this channel again
scanner-next = NEXT
scanner-move-now = Move on now

## adsb (src/app/panels/adsb.rs)

adsb-starting-ads-b-decoder = starting the ADS-B decoder…
adsb-ads-b = ADS-B
adsb-t-1090-mhz-mode-s = 1090 MHz Mode S
adsb-tune-receiver-1090-000-mhz = Tune the receiver to 1090.000 MHz, the worldwide ADS-B downlink. There is only one channel.
adsb-setup = SETUP
adsb-timeouts-trail-length-how-far = Timeouts, trail length and how far ahead the speed vectors reach
adsb-aircraft = AIRCRAFT
adsb-nothing-heard-yet = nothing heard yet
adsb-ground = on ground
adsb-position-stale = position stale
adsb-gnss-altitude = GNSS altitude
adsb-ground-speed = ground speed
adsb-vertical-rate = vertical rate
adsb-first-heard = first heard
adsb-center = CENTER
adsb-put-aircraft-middle-map = Put this aircraft in the middle of the map
adsb-last-frame-accepted-aircraft = The last frame accepted from this aircraft
adsb-ads-b-setup = ADS-B Setup
adsb-drop-map-after = Drop from map after
adsb-without-position-report = without a position report
adsb-past-aircraft-comes-off-map = Past this the aircraft comes off the map and its row greys. It is not faded: a dim square at a stale position is still a claim about where an aeroplane is, in the same ink as the true ones.
adsb-drop-list-after = Drop from list after
adsb-nothing-heard-all = with nothing heard at all
adsb-trail-length = Trail length
adsb-points = {" "}points
adsb-history-dots-behind-each-target = history dots behind each target
adsb-speed-vector = Speed vector
adsb-how-far-ahead-leader-line = how far ahead the leader line reaches
adsb-one-minute-usual-radar-convention = One minute is the usual radar convention: the line is exactly as long as the distance the aircraft covers in that time, so two equal leaders are two equal speeds at any zoom. Zero switches them off.
adsb-track-most = Track at most
adsb-aircraft-2 = {" "}aircraft
adsb-aircraft-ground-are-placed-against = Aircraft on the ground are placed against the station's own position — a surface squitter has no unambiguous decode of its own — so fill in My grid in the digimode setup if the airport nearby shows nothing.
adsb-call = CALL
adsb-icao = ICAO
adsb-v-s = V/S

## drm (src/app/drm.rs)

drm-waiting-receiver = waiting for the receiver…
drm-not-drm-set-mode-drm = Not in DRM. Set the mode to DRM on a digital shortwave broadcast — the dial goes on the channel centre, not on a sideband.
drm-no-drm-signal-locked-decoder = No DRM signal locked. The decoder needs a few seconds on a clean carrier: DRM interleaves over 400 ms or 2 s before any of it can be read.
drm-station-s-audio-codec-not = This station's audio codec is not decoding here. xHE-AAC needs libfdk-aac on the system — it cannot be built in, because its licence and this program's are incompatible. Install it (Debian/Ubuntu: libfdk-aac2, Arch: libfdk-aac, macOS: brew install fdk-aac, Windows: libfdk-aac-2.dll beside sdroxide.exe) and restart. Everything else on this screen is decoding normally.
drm-quality = QUALITY
drm-last-minute = last minute
drm-waiting-locked-signal = waiting for a locked signal
drm-audio-frames-decoding-but-station = Audio frames decoding — but this station's codec cannot be decoded here
drm-audio-frames-decoding = Audio frames decoding
drm-samples-reaching-decoder = Samples reaching the decoder
drm-time = TIME
drm-symbol-timing-recovered = Symbol timing recovered
drm-frame = FRAME
drm-transmission-frames-found = Transmission frames found
drm-fast-access-channel-what-transmission = Fast Access Channel — what the transmission is
drm-service-description-channel-what-services = Service Description Channel — what the services are
drm-audio = AUDIO
drm-constellation = CONSTELLATION
drm-waiting-decoded-symbols = waiting for decoded symbols
drm-sampled-across-frame = sampled across the frame
drm-whole-frame = whole frame
drm-mode = MODE
drm-robustness-mode-channel-width-ground = Robustness mode and channel width. A is for a ground-wave path and carries the most; D is for a badly scattered sky-wave one and carries the least.
drm-interleave = INTERLEAVE
drm-how-far-transmission-spreads-each = How far the transmission spreads each frame in time. Long rides out deeper fades and takes correspondingly longer to acquire.
drm-protection = PROTECTION
drm-offset = OFFSET
drm-residual-sample-clock-error-against = Residual sample-clock error against the transmitter. Large and steady means the receiver's reference is off, not the broadcast.
drm-doppler = DOPPLER
drm-delay = DELAY
drm-doppler-delay-spread-path-how = Doppler and delay spread of the path — how fast it is moving and how far apart its echoes arrive.
drm-service = SERVICE
drm-decode-service-multiplex = Decode this service of the multiplex

## atchat (src/app/panels/atchat.rs)

atchat-starting-atchat-net-station = starting the AtCHAT NET station…
atchat-atchat = ATCHAT
atchat-net-multi-station-keyboard-file = NET — multi-station keyboard & file
atchat-net = {" "}ON NET{" "}
atchat-net-click-leave-state-kept = On the net — click to leave (state is kept; click again to rejoin)
atchat-rejoin = {" "}REJOIN{" "}
atchat-off-net-click-rejoin = Off the net — click to rejoin
atchat-off-net = off net
atchat-carrier = ● CARRIER
atchat-another-station-transmitting-channel-busy = Another station is transmitting — the channel is busy.
atchat-tx = ● TX
atchat-setup = ⚙ SETUP
atchat-station-callsign-virtual-channel-address = Station callsign and the virtual-channel address
atchat-virtual-channel = VIRTUAL CHANNEL
atchat-work-net-over-tcp-endpoint = Work the net over a TCP endpoint instead of the radio — a channel_server-compatible loopback for radio-less development. Off puts the station on the air.
atchat-roster = ROSTER
atchat-nobody-heard-yet = nobody heard yet
atchat-chat = {" "}CHAT{" "}
atchat-common-channel-lines-here-go = The common channel — lines here go to everyone (ALL)
atchat-log = {" "}LOG{" "}
atchat-station-s-own-air-activity = The station's own on-air activity — master election, roster ageing, ARQ retries
atchat-no-activity-logged-yet = No activity logged yet.
atchat-no-messages-yet = No messages yet.
atchat-message-everyone-all = message to everyone (ALL)
atchat-send = {" "}SEND{" "}
atchat-transfers = TRANSFERS
atchat-all = → ALL
atchat-sent-file-follows-active-chat = A sent file follows the active chat tab
atchat-send-file = {" "}SEND FILE{" "}
atchat-send-file-image-over-air = Send a file or image over the air, block-CRC-ARQ
atchat-nothing-flight = nothing in flight
atchat-images = IMAGES
atchat-no-images-received = No images received.


## rds (src/app/rds.rs)

rds-station = STATION
rds-diagnostics = DIAGNOSTICS
rds-which-programme-type-table-read = Which programme-type table to read this station against. RDS is used everywhere outside North America; RBDS adds the call sign a US station's identity code spells out. Auto follows the extended country code the station sends, and until one arrives it will not spell out a call sign — the identity ranges it would read are shared with nine countries' RDS codes, so select RBDS to name a station that sends no country code.
rds-waiting-receiver = waiting for the receiver…
rds-no-rds-station-not-every = No RDS on this station. Not every transmitter carries it, and a weak one may carry it without it arriving.
rds-rds-only-carried-wfm-broadcast = RDS is only carried on WFM broadcast stations.
rds-holding = HOLDING
rds-block-sync-has-dropped-last = Block sync has dropped — this is the last station read.
rds-identity = IDENTITY
rds-programme = PROGRAMME
rds-described = DESCRIBED AS
rds-content = CONTENT
rds-music = Music
rds-speech = Speech
rds-traffic = TRAFFIC
rds-announcement-air-now = announcement on air now
rds-carried-station = carried by this station
rds-not-carried = not carried
rds-radio-text = RADIO TEXT
rds-station-clock = STATION CLOCK
rds-country-code = COUNTRY CODE
rds-also = ALSO ON
rds-also-tagged = ALSO TAGGED
rds-sync = SYNC
rds-no-sync = NO SYNC
rds-blocks-failed-their-check-plus = Blocks that failed their check, plus those the error corrector had to repair. Corrections count as errors on purpose: a rising figure is the first sign a station is about to stop decoding.
rds-front-end-overloading = FRONT END OVERLOADING
rds-converter-clipping-which-puts-distortion = The converter is clipping, which puts distortion right across the multiplex — including on the 57 kHz data subcarrier. Turn the RF gain down, or switch in an attenuator: RDS goes about a decibel before anything is audible, so a station that sounds perfect can still be far too strong for its data to survive.
rds-group-types = GROUP TYPES
rds-recent-groups-newest-last = RECENT GROUPS — newest last
rds-nothing-decoded-yet = nothing decoded yet

## winlink (src/app/winlink.rs)

winlink-mail = MAIL
winlink-abort = ABORT
winlink-stop-session-progress = Stop the session in progress
winlink-connect = CONNECT
winlink-compose = COMPOSE
winlink-connecting = connecting…
winlink-not-connected-yet = not connected yet
winlink-loading = loading…
winlink-date = DATE
winlink-from = FROM
winlink-subject = SUBJECT
winlink-no-subject = (no subject)
winlink-delete-message = Delete this message
winlink-arch = ARCH
winlink-move-archive = Move to the archive
winlink-select-message = select a message
winlink-reply = REPLY
winlink-delete = DELETE
winlink-to = To
winlink-cc = Cc
winlink-subject-2 = Subject
winlink-callsign-smtp-someone-example-org = A callsign, or SMTP:someone@example.org for internet mail. Separate several with commas.
winlink-file-outbox = FILE IN OUTBOX
winlink-discard = DISCARD
winlink-filed-messages-go-out-next = filed messages go out on the next session
winlink-mail-log = MAIL LOG
winlink-session-running = session running…
winlink-no-session-yet = no session yet

## wspr (src/app/panels/wspr.rs)

wspr-receptions = RECEPTIONS
wspr-save-wspr-reception-list-csv = Save the WSPR reception list as CSV
wspr-wspr = WSPR
wspr-weak-signal-propagation-beacon = weak-signal propagation beacon
wspr-time-left-two-minute-slot = Time left in this two-minute slot. At the end of it the recording goes to the decoder and the next transmit decision lands.
wspr-tx = ● TX
wspr-decoding = decoding…
wspr-band = Band
wspr-last-slot = Last slot
wspr-nothing-heard = nothing heard
wspr-receive-only = receive only
wspr-next-slot = Next slot
wspr-waiting-radio-take-change = waiting for the radio to take the change…
wspr-hop-next = Hop next
wspr-transmit = TRANSMIT
wspr-receive-only-2 = Receive only.
wspr-power = POWER
wspr-what-you-actually-radiate = what you actually radiate
wspr-goes-out-message-everyone-who = This goes out in the message, and everyone who hears you judges the path by it — so an optimistic figure here makes their measurements wrong as well as yours.
wspr-roam = ROAM
wspr-pick-different-offset-inside-200 = Pick a different offset inside the 200 Hz window for every transmission. This is the convention: two hundred hertz shared by everyone only works if nobody parks in the middle of it. Off holds where the cursor is.
wspr-band-hop = BAND HOP
wspr-move-dial-band-band-between = Move the dial from band to band between slots, so one receiver samples the whole spectrum instead of one slice of it. Turning the VFO yourself pauses it — press this twice to resume.
wspr-upload = UPLOAD
wspr-send-what-station-decodes-wsprnet = Send what this station decodes to wsprnet.org. Reporting what you hear is what makes a WSPR receiver part of the network; it puts nothing on the air.
wspr-who-heard-me = WHO HEARD ME
wspr-ask-wsprnet-org-every-few = Ask wsprnet.org every few minutes which stations decoded this one. WSPR has no acknowledgement of any kind, so this is the only way a beacon learns anything about its own reach.
wspr-bands = BANDS
wspr-none-selected-hopping-has-nowhere = none selected — hopping has nowhere to go
wspr-set-your-callsign-grid-general = Set your callsign and grid on the General tab of Settings before transmitting.

## packet (src/app/panels/packet.rs)

packet-monitor = MONITOR
packet-busy = BUSY
packet-frames-arrived-but-failed-their = Frames that arrived but failed their check sequence — a collision, a fade, or a signal too weak to read.
packet-setup = ⚙ SETUP
packet-station-callsign-speed-tx-delay = Station callsign, speed, TX delay, the digipeater path, the beacon and the KISS server
packet-starting-packet-modem = starting the packet modem…
packet-nothing-heard-yet = nothing heard yet
packet-terminal = TERMINAL
packet-connected = {" "}CONNECTED{" "}
packet-calling = {" "}CALLING{" "}
packet-idle = {" "}IDLE{" "}
packet-extended-sequence-numbers-window-up = Extended sequence numbers — a window of up to 127 frames.
packet-frames-sent-not-yet-acknowledged = Frames sent and not yet acknowledged by the far end.
packet-retries-against-link-s-limit = Retries against this link's limit. A count climbing while the unacknowledged frames stay put is what a fading path looks like from this side.
packet-mail-window-using-link = The MAIL window is using the link.
packet-one-radio-one-channel-one = One radio, one channel, one link. A Winlink session and this terminal are two ways to use it, and whichever asks first gets it.
packet-digipeaters-nearest-first-separated-commas = Digipeaters, nearest first, separated by commas — OE3XLR-1,OE3XMS-1. Leave it empty for a station you can hear directly, or to use the default path from the packet settings.
packet-disconnect = {" "}DISCONNECT{" "}
packet-hang-up-properly-wait-far = Hang up properly, and wait for the far end to agree. Changing mode instead ends the session without telling anybody.
packet-connect = {" "}CONNECT{" "}
packet-mail-window-has-link-finish = The MAIL window has the link. Finish or stop that session first.
packet-station-has-no-callsign-yet = This station has no callsign yet — set one under SETUP, above the monitor pane. Nothing transmits until it is set.
packet-call-station-connected-mode-node = Call this station in connected mode — a node, a BBS, or another operator.
packet-needs-callsign-call = Needs a callsign to call.
packet-clear = {" "}CLEAR{" "}
packet-empty-transcript-link-untouched-connected = Empty the transcript. The link is untouched — a connected station stays connected.
packet-not-connected-type-callsign-above = Not connected. Type a callsign above and press CONNECT to call a node or a BBS.
packet-type-here = type here
packet-not-connected = not connected
packet-send = {" "}SEND{" "}
packet-send-line-node-bbs-reads = Send the line. A node or a BBS reads one line at a time.

## fsq (src/app/panels/fsq.rs)

fsq-speed = Speed
fsq-call = Call
fsq-callsign-directed-fsqcall-messages-defaults = Callsign for directed (FSQCALL) messages; defaults to your callsign
fsq-tx = ● TX
fsq-contacts = CONTACTS
fsq-heard = HEARD
fsq-none = — none —
fsq-allcall = ALLCALL
fsq-image = IMAGE
fsq-clear = CLEAR
fsq-forget-pictures-received-so-far = Forget the pictures received so far
fsq-send-image = Send image…
fsq-right-click-remove = Right-click to remove
fsq-remove-picture = Remove this picture
fsq-listening = — listening —
fsq-message = Message…
fsq-send = {" "}SEND{" "}
fsq-call-cq = {" "}CALL CQ{" "}
fsq-heard-2 = {" "}? heard{" "}
fsq-clear-2 = {" "}CLEAR{" "}
fsq-fsq-contacts = FSQ Contacts
fsq-add = Add

## publicsdr (src/app/publicsdr.rs)

publicsdr-not-stated = not stated
publicsdr-add-as-radio = ADD AS RADIO
publicsdr-add-as-radio-tip = Add as radio, named from directory
publicsdr-public-sdrs = PUBLIC SDRS
publicsdr-available = AVAILABLE
publicsdr-hide-receivers-are-full-ones = Hide receivers that are full, and the ones whose operator has not opened any channels to apps other than a browser
publicsdr-band = IN BAND
publicsdr-only-receivers-cover-current-dial = Only receivers that cover the current dial frequency
publicsdr-low-bw = LOW BW
publicsdr-map-cities = CITIES
publicsdr-map-cities-tip = Cities on this map, on or off.
publicsdr-view-list = LIST
publicsdr-view-map = MAP
publicsdr-view-map-tip = Receivers on world map; click dot to pick all under it
publicsdr-map-pick = Click a dot to pick the receivers under it.
publicsdr-map-ghost = { $name }: not in the directory now — last seen here on { $seen }
publicsdr-map-names = NAMES
publicsdr-map-names-tip = Names of all receivers; off: picked only
publicsdr-check-online = ONLINE
publicsdr-check-silent = SILENT
publicsdr-check-waiting = CHECKING…
publicsdr-check-no-answer = No answer within 20 seconds.
publicsdr-take-spyserver-low-bandwidth-shape = Take a SpyServer in its low-bandwidth shape: a narrow I/Q window that follows the dial plus the server's own band view, instead of megabits of wideband I/Q. No effect on a KiwiSDR, which has only the one shape.
publicsdr-refresh = ⟳ REFRESH
publicsdr-fetch-both-directories-again = Fetch both directories again
publicsdr-name-place-antenna-band = name, place, antenna, band
publicsdr-clear-search = Clear the search
publicsdr-fetching-receiver-lists = Fetching the receiver lists…
publicsdr-machine-radio-attached-does-not = The machine this radio is attached to does not answer device questions, so it cannot fetch the lists either.
publicsdr-no-match = no match
publicsdr-nothing-matches-try-turning-filter = nothing matches — try turning a filter chip back on

## hd (src/app/hd.rs)

hd-hd-radio = HD Radio
hd-waiting-receiver = waiting for the receiver…
hd-not-hd-radio-set-mode = Not in HD Radio. Set the mode to HD Radio on an FM broadcast carrying the digital sidecars — the dial goes on the analog carrier's centre, not beside it.
hd-no-hd-radio-lock-digital = No HD Radio lock. The digital sidebands are transmitted about 20 dB below the analog carrier, so a listenable station can still be too noisy to decode: the decoder needs a few seconds on a clean one.
hd-sync = SYNC
hd-ofdm-frame-timing-recovered = OFDM frame timing recovered
hd-audio = AUDIO
hd-audio-frames-decoding = Audio frames decoding
hd-quality = QUALITY
hd-lower = LOWER
hd-upper = UPPER
hd-last-minute = last minute
hd-waiting-locked-signal = waiting for a locked signal
hd-mer-l = MER L
hd-modulation-error-ratio-lower-digital = Modulation error ratio of the lower digital sideband.
hd-mer-u = MER U
hd-modulation-error-ratio-upper-digital = Modulation error ratio of the upper digital sideband.
hd-cber = CBER
hd-channel-bit-error-ratio-after = Channel bit-error ratio after the inner code. Below about 1e-3 the outer code can correct it; above that the audio starts to break up.
hd-offset = OFFSET
hd-residual-carrier-frequency-offset-large = Residual carrier frequency offset. Large and steady means the receiver's reference is off, not the broadcast.
hd-psmi = PSMI
hd-primary-service-mode-indicator-which = Primary Service Mode Indicator: which of the FM hybrid service modes the station is transmitting.
hd-programme = PROGRAMME
hd-restricted-access = {" "}— restricted access

## bands (src/app/bands.rs)

bands-bands = BANDS
bands-daylight = ☀ DAYLIGHT
bands-night = ☾ NIGHT
bands-no-forecast-yet-fetching-hamqsl = · no forecast yet — fetching from HAMQSL.com
bands-forecast-download-off = · forecast download is off (Settings → UI)
bands-band = BAND
bands-condx = CONDX
bands-paths = PATHS
bands-reach = REACH
bands-best = BEST
bands-condx-global-forecast-rest-what = CONDX is a global forecast; the rest is what has actually been heard. An empty row means nothing was decoded — which may mean the band was shut, or only that nobody was on it.
bands-ibp-beacons = IBP BEACONS
bands-meteor-showers = METEOR SHOWERS
bands-peak = PEAK

## spots (src/app/spots.rs)

spots-pota = POTA
spots-sota = SOTA
spots-freedv = FREEDV
spots-spots = SPOTS
spots-longwave-shortwave-broadcast-stations-air = Longwave & shortwave broadcast stations on air now
spots-view = IN VIEW
spots-only-spots-inside-panadapter-span = Only spots inside the panadapter span
spots-view-list = LIST
spots-view-map = MAP
spots-view-map-tip = Spots with location on world map; click dot to pick, right-click to release
spots-map-names = NAMES
spots-map-names-tip = Names beside dots
spots-map-cities = CITIES
spots-map-cities-tip = Cities on this map, on or off
spots-map-hint = Click dot or waterfall label: location and path; right-click releases
spots-map-pick = { $name } · { $place } · { $km } km · { $deg }°
spots-map-pick-nowhere = { $name } · location unknown
spots-map-status = { $shown } on map · { $approx } of them approximate · { $unplaced } without location
spots-map-approx-city = ≈ place from the name ({ $city })
spots-map-approx-country = ≈ middle of the country (site { $site })
spots-map-size-legend = Dot size: transmitter power (log)
spots-map-tune = TUNE
spots-setup = ⚙ SETUP
spots-feeds-lookup-upload-settings = Feeds, lookup & upload settings
spots-call-station-site-frequency = call, station, site, frequency
spots-clear-search = Clear the search
spots-no-match = no match
spots-no-spots-enable-feed-setup = no spots — enable a feed in ⚙ SETUP
spots-nothing-matches-search = nothing matches the search

## ism (src/app/ism.rs)

ism-ism-devices = ISM DEVICES
ism-decoding = DECODING
ism-needs-wideband-iq-source-cat = Needs a wideband IQ source; a CAT rig sends only audio
ism-how-far-burst-must-stand = How far a burst must stand above the channel's own noise floor before it is decoded
ism-give-each-band-width-normally = Give each band the width it normally needs — a quarter of a megahertz for the OOK bands at 315, 345 and 433 MHz, a full one for 868 and 915.
ism-several-hundred-more-device-decoders = Several hundred more device decoders, from the rtl_433 project, built in. Reads the 433 MHz OOK devices sdroxide's own decoders do not reach at all. Where both can read a device, rtl_433 knows far more variants and takes over.
ism-t-1-your-own = 1 of your own
ism-device-decoders-registered-how-many = Device decoders registered, how many came from your rtl433_flex.conf, and how many transmissions have been decoded since it started.
ism-reload-decoders = RELOAD DECODERS
ism-re-read-rtl433-flex-conf = Re-read rtl433_flex.conf after editing it. The devices already heard stay in the list.
ism-rtl-433-knows-more-variants = rtl_433 knows more variants of these than sdroxide's own decoders do, so it reads them while it is listening here. Select another band and sdroxide's own take over again.
ism-decoder-your-rtl433-flex-conf = A decoder in your rtl433_flex.conf was refused. The others still loaded. Fix the line, then press RELOAD DECODERS.
ism-waiting-decoder = waiting for the decoder…
ism-where-decoder-actually-listening-follows = Where the decoder is actually listening. This follows the receiver's hardware centre, not the VFO — on a wide front end the two can be a long way apart, and it is the window that decides which channels are reachable.
ism-transmissions-gate-opened-how-many = Transmissions the gate opened on, and how many produced a valid frame. Many bursts and no decodes means the band is busy with devices sdroxide cannot read yet. None at all means nothing is reaching the threshold — lower it, or check that the dial is on the band.
ism-nothing-heard-yet = nothing heard yet
ism-click-tune-receiver =
    {""}

    Click to tune the receiver to it

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

navtex-messages = MESSAGES
navtex-sync = SYNC
navtex-character-phase-locked-signal-being = The character phase is locked — a signal is being read.
navtex-no-character-phase-yet-no = No character phase yet: no signal, or not this one.
navtex-characters-whose-first-copy-was = Characters whose first copy was corrupt and were taken from the repeat five slots later, and those where both were bad. A NAVTEX character is sent twice; that is the whole of its error correction.
navtex-swap-mark-space-tones-signal = Swap the mark and space tones, for a signal received on the other sideband. Off is upper sideband on the channel frequency, which is what every published tuning instruction for the service says.
navtex-starting-navtex-receiver = starting the NAVTEX receiver…
navtex-nothing-yet-station-transmits-ten = nothing yet — a station transmits for ten minutes every four hours
navtex-cut-short = cut short
navtex-message = MESSAGE
navtex-time-day-stated-message-utc = Time of day stated in the message (UTC): display only
navtex-receiving = RECEIVING
navtex-monitor = MONITOR
navtex-everything-decoded-message-not = everything decoded, message or not

## rfpaint (src/app/panels/rf_paint.rs)

rfpaint-rf-paint = RF PAINT
rfpaint-scan-speed = Scan speed
rfpaint-how-fast-text-image-scanned = How fast the text/image is scanned onto the waterfall. Lower is slower and more legible; 100% = base rate, 25% (centre) is the default.
rfpaint-abort = Abort
rfpaint-text-paint = TEXT PAINT
rfpaint-type-text-paint = Type text to paint…
rfpaint-preview-waterfall = PREVIEW WATERFALL
rfpaint-type-text-preview = type text to preview
rfpaint-transmit = {"  "}TRANSMIT{"  "}
rfpaint-image-paint = IMAGE PAINT
rfpaint-no-image-loaded = no image loaded
rfpaint-load-image = Load image…
rfpaint-load-image-preview = load an image to preview

## hfdl (src/app/hfdl.rs)

hfdl-listen = LISTEN
hfdl-switches-engine-s-hfdl-downconverter = Switches the engine's HFDL downconverter and decoder on or off. Decoding costs a 24 kHz lane and a worker thread whether the window is open or not.
hfdl-channel-assigned-frequency-listen-anything = The channel (the assigned frequency) to listen on. Anything in the band works — HFDL stations transmit on 2.8 to 22 MHz — against a fixed 24 kHz lane centred here.
hfdl-save-hfdl-decode-log-file = Save the HFDL decode log to a file
hfdl-riverhead-northern-atlantic-north-america = Riverhead (northern Atlantic/north America) — the default, and the channel the decoder was validated on.
hfdl-hfdl-assigned-frequency = An HFDL assigned frequency.
hfdl-one-your-saved-channels-click = One of your saved channels — click to tune, right-click to remove.
hfdl-hfdl-selected-but-decoding-off = HFDL is selected but decoding is off — press LISTEN above
hfdl-decodes = DECODES
hfdl-nothing-decoded-yet-ground-station = Nothing decoded yet — a ground station's squitter repeats every ~32 s once one is in the channel.
hfdl-no-decode-matches-filter = No decode matches the filter.
hfdl-decoding = DECODING
hfdl-decoding-off = DECODING OFF
hfdl-dbfs = — dBFS

## rade (src/app/panels/rade.rs)

rade-rade = RADE
rade-freedv-v1-digital-voice = FreeDV V1 digital voice
rade-tx = ● TX
rade-mute-analog = MUTE ANALOG
rade-mute-demodulated-audio-so-only = Mute the demodulated audio, so only decoded speech is heard. The raw signal is otherwise passed through whenever the modem has nothing to play — that hiss is how you find an over before it syncs, so leave this off while tuning.
rade-sync = SYNC
rade-how-far-received-signal-sits = How far the received signal sits from where the modem expects it. Large values still decode — the acquisition loop tracks them — but nudging the dial to bring this near zero gives the best margin.
rade-snr = SNR —
rade-callsign-station-put-end-over =
    The callsign the station put in its End-of-Over frame — the only point in a RADE over at which one is sent. It is also what is reported to FreeDV Reporter, so a station you hear appears on qso.freedv.org as heard by you.

    Held for a minute after the signal goes, and dropped when you start an over of your own: it names the station on frequency, not every station the receiver has ever heard.
rade-stop-talking = STOP TALKING
rade-talk-ptt = TALK (PTT)
rade-open-close-rade-over-modem = Open or close a RADE over. The modem needs ~120 ms of speech before the first frame goes out, and sends an end-of-over frame when you stop, so transmit runs on a little past the button.
rade-rade-v1-occupies-roughly-1060 = RADE V1 occupies roughly 1060–1880 Hz of the passband — upper sideband, or lower on 160, 80 and 40 m, where RADE keeps phone practice and sdroxide switches for you. Put the signal inside the shaded band on the waterfall; the modem finds it from there.

## frame (src/app/frame.rs)

frame-acknowledge = Acknowledge
frame-curtain-handle-tip = Drag: panel up/down over panadapter
    Double-click: all the way up / back
frame-dismiss = Dismiss
frame-get-sdroxide-com = Get it at sdroxide.com
frame-reconnecting = Reconnecting…
frame-not-connected = Not connected.
frame-reconnect-now = Reconnect now
frame-transmit-lockout-disabled = ⚠ TRANSMIT LOCKOUT DISABLED
frame-engine-was-started-oob-tx = This engine was started with --oob-tx. The amateur-band lockout is off: it will key the transmitter on any frequency the hardware supports.
frame-transmitting-outside-your-licence-offence = Transmitting outside your licence is an offence in every country that issues one. Only continue if you are authorised to use the frequencies you are about to key on — a MARS/CAP or commercial licence, an experimental permit, or a dummy load.
frame-i-understand = {"  "}I UNDERSTAND{"  "}
frame-restart-without-oob-tx-put = Restart without --oob-tx to put the lockout back.

## smeter (src/widgets/smeter.rs)

smeter-alc-how-hard-transmitter-being = ALC — how hard the transmitter is being driven, as a percentage of the most it can put out. 100 % is full scale, not full power in watts: what that is worth at the antenna depends on the radio and on Drive.
smeter-radio-reports-no-forward-power =
    {""}

    This radio reports no forward power, so the figure beside ALC is the ALC reading again rather than watts.
smeter-no-swr-radio-has-no =
    {""}

    No SWR: this radio has no bridge to measure it with, and sdroxide will not invent one — it can only show what the radio itself reports. A rig with an SWR meter (over CAT, TCI or a LAN link) and an HPSDR board both fill this in; a plain SDR transmitter cannot.
smeter-no-signal-report-yet-receiver = No signal report yet — the receiver has not delivered a meter reading.
smeter-s9-73-dbm-each-s =
    {""}

    S9 is −73 dBm and each S-unit below it is 6 dB. The dBm figure is only as honest as the front end's calibration — an uncalibrated receiver reports dBFS with a dBm label.
smeter-ovld-converter-into-rails-so =
    {""}

    OVLD: the converter is into its rails, so the level shown understates the signal. Wind the front-end gain back.
smeter-table-held = , table held
smeter-ps = PS --

## multi (src/multi.rs)

multi-tx = ● TX
multi-mute-radio-s-audio = Mute this radio's audio
multi-show-radio-here = Show this radio here
multi-show-on-map = Show on the map
multi-add-radio-computer-settings-radio = Add a radio on this computer (Settings → Radio to add one at a station)
multi-add-radio = Add a radio
multi-take-radio-off-row = Take off row; keeps running
multi-open-pane = New pane with hidden radio
multi-pick-radio = Radio ▼
multi-pick-radio-tip = Add radio to row; rings = link open
multi-new-radio = New radio…
multi-asked-station-another-radio = Asked the station for another radio…
multi-client-cannot-open-connection = This client cannot open a connection.

## net (src/app/net.rs)

net-fail = FAIL
net-will-retry = {" "}(will retry)
net-full-resync-downloading = Full resync: downloading…
net-downloading = Downloading…
net-full-resync-log-already-matches = Full resync: the log already matches Wavelog
net-full-resync-review-changes-below = Full resync: review the changes below
net-full-resync-discarded-nothing-changed = Full resync discarded; nothing changed
net-adif-import-failed-file-could = ADIF import failed: the file could not be parsed
net-channel-import-failed-file-could = Channel import failed: the file could not be read
net-channel-import-nothing-read-chirp = Channel import: nothing to read — a CHIRP CSV file starts with a header row naming its columns, one of which must be Frequency

## widgets (src/app/panels/widgets.rs)

widgets-free-text = free text
widgets-hashed-callsign-not-yet-heard = hashed callsign, not yet heard in full
widgets-entity-unknown = entity unknown
widgets-band = this band
widgets-new-entity-never-worked-any = New entity — never worked, on any band
widgets-new-grid-square = New grid square
widgets-not-worked-before = Not worked before
widgets-worked-before-but-not-band = Worked before, but not on this band
widgets-call-queue = In the call queue
widgets-image = Image

## pi4 (src/app/panels/pi4.rs)

pi4-receptions = RECEPTIONS
pi4-save-pi4-reception-list-csv = Save the PI4 reception list as CSV
pi4-next-generation-beacon = Next Generation Beacon
pi4-time-left-one-minute-beacon = Time left in this one-minute beacon cycle. The PI4 message is searched for once the window has had time to fill past where it would end.
pi4-decoding = decoding…
pi4-band = Band
pi4-last-cycle = Last cycle
pi4-nothing-heard = nothing heard
pi4-receive-only-tune-so-beacon = Receive only. Tune so the beacon's CW identification sits at 800 Hz audio — the network's own convention — and the four PI4 tones fall where this decoder searches for them by default.

## awards (src/app/awards.rs)

awards-awards = AWARDS
awards-band = Band
awards-all = All
awards-dxcc = DXCC
awards-grids = Grids
awards-worked-all-states = Worked All States
awards-cq-zones-waz = CQ Zones (WAZ)
awards-dxcc-entities = DXCC entities

## acars (src/app/panels/acars.rs)

acars-acars = ACARS
acars-audio-decoder-s-passband = Audio in the decoder's passband.
acars-blocks-framed-blocks-whose-check = Blocks that framed, and blocks whose check sequence failed.
acars-starting-acars-receiver = starting the ACARS receiver…
acars-channel = CHANNEL
acars-tune-dial-here = Tune the dial here.
acars-nothing-yet-acars-quiet-between = Nothing yet. ACARS is quiet between aircraft — leave it on a channel and wait for a burst.
acars-check-failed = check failed
acars-block-check-sequence-did-not = The block-check sequence did not match — text may be wrong.
acars-no-text = (no text)

## login (src/login.rs)

login-sdr-oxide = SDR OXIDE
login-username = Username
login-password = Password
login-remember-device = Remember on this device
login-signs-without-asking-next-time = Signs in without asking next time, and lets the 3D view's tab in without asking at all. The password is kept in the clear, so leave this off on a machine other people use.
login-keeps-password-device-clear = Keeps the password on this device, in the clear.
login-remember-station = Remember this station
login-remember-me = REMEMBER ME
login-station-keeps-sign-in = This station keeps your sign-in, not this page — nothing to fill in on the other radios, and nothing to forget when you leave.
login-not-this-time = Not this time: this browser forgets it when it closes, so you are asked again next session and nothing about this sign-in is left on the device.
login-checking = {" "}CHECKING…{" "}
login-sign = {" "}SIGN IN{" "}
login-wrong-password-shuts-server-s = A wrong password shuts the server's door for a few seconds, so this can take a moment.

## help (src/help.rs)

help-find = FIND
help-find-manual = find in manual
help-previous-match-shift-enter-shift = Previous match (Shift+Enter, Shift+F3)
help-next-match-enter-f3 = Next match (Enter, F3)
help-no-matches = no matches
help-clear-search-esc = Clear the search (Esc)
help-sdroxide-manual = SDROXIDE MANUAL
help-contents = CONTENTS

## aismap (src/ais_map.rs)

aismap-t-1-station-heard-waiting-report = 1 station heard — waiting for it to report a position
aismap-double-click-reframe = double-click to reframe
aismap-zoom-out-wheel-pinch-do = Zoom out — the wheel or a pinch do this too
aismap-zoom-about-middle-chart = Zoom in about the middle of the chart
aismap-frame-everything-being-tracked-again = Frame everything being tracked again — a double-click does too

## specview (src/widgets/spectrum_view.rs)

specview-waiting-spectrum = waiting for spectrum…
specview-hounds = HOUNDS

## spectrum (src/app/spectrum.rs)

spectrum-window-has-no-gpu-renderer = This window has no GPU renderer, so the waterfall stays at its standard width.
spectrum-machine-drawing-without-gpu-every = This machine is drawing without a GPU — every column of the waterfall is the processor's work, and it is already sharing that with the radio.
spectrum-window-renders-through-opengl-which = This window renders through OpenGL, which is sdroxide's compatibility path — a Raspberry Pi, an older graphics chip, or a browser without WebGPU. A wider waterfall is not worth the frame rate there.
spectrum-wider-than-renderer-can-draw = Wider than this renderer can draw.

## macros (src/app/panels/macros.rs)

macros-nocall = NOCALL
macros-each-button-sends-whole-text = Each button sends its whole text in one go. F2–F10 press the first nine, so long as nothing on screen has the keyboard.
macros-remove-button = Remove this button

## gridtracker (src/app/grid_tracker.rs)

gridtracker-grid-tracker = GRID TRACKER
gridtracker-shade-squares-heard-live-decode = Shade the squares heard on the live decode list, not only the ones in the log — what is on the air now, in cyan under the worked squares.
gridtracker-drag-pan-wheel-zoom-amber = Drag to pan, wheel to zoom. Amber = worked, green = confirmed, cyan = heard.

## remote (src/remote.rs)

remote-server-busy-another-client-connected = server busy — another client is connected
remote-transmitting-but-client-s-microphone = Transmitting, but this client's microphone is producing no audio at all — nothing is being modulated. In a browser, check that the page was allowed the microphone and that no other application holds it.
remote-connection-closed = connection closed

## solarwin (src/solar3d/mod.rs)

solarwin-sdroxide-solar-system = sdroxide — solar system

## solarbtn (src/app/solar.rs)

solarbtn-solar-system-3d-view-sun = Solar system 3D view — Sun, Earth, Moon, sunspots and CMEs (separate window)
solarbtn-solar-system-3d-view-sun-2 = Solar system 3D view — Sun, Earth, Moon, sunspots and CMEs (new browser tab)

## widespec (src/widgets/wide_spectrum.rs)


## speech (src/app/speech.rs)

speech-loading-voice = Loading the voice…
speech-browser-client-cannot-speak-yet = the browser client cannot speak yet

## freqdisplay (src/widgets/freq_display.rs)

freqdisplay-double-click-type-frequency = Double-click to type a frequency
freqdisplay-tap-type-frequency = Tap to type a frequency

## solarstate (src/solar3d/state.rs)

solarstate-home = HOME
solarstate-dwarf-planets = DWARF PLANETS

## adsbmap (src/adsb_map.rs)

adsbmap-double-click-reframe = double-click to reframe

## aprsmap (src/aprs_map.rs)

aprsmap-double-click-reframe = double-click to reframe

## solarnet (src/solar3d/net.rs)

solarnet-connection-closed = connection closed

## alerts (src/app/alerts.rs)

alerts-could-not-start-alert-thread = could not start the alert thread

## hfdlmap (src/hfdl_map.rs)

hfdlmap-no-aircraft-located-yet = no aircraft located yet

## worldmap (src/widgets/worldmap.rs)

worldmap-double-click-reframe = DOUBLE-CLICK TO REFRAME

## chrome (src/chrome.rs)



## duopower (src/app/duo_power.rs)

duopower-sets-fdm-duo-s-own = Sets the FDM-DUO's own power setting (menu 34 TX POWER). Drive and Tune then set the power within it, and SDRoxide sends the setting again each time it connects.

## Band conditions window: a peak date.

# $month is 1–12.
bands-peak-date = { $day } { $month ->
        [1] Jan
        [2] Feb
        [3] Mar
        [4] Apr
        [5] May
        [6] Jun
        [7] Jul
        [8] Aug
        [9] Sep
        [10] Oct
        [11] Nov
       *[12] Dec
    }

## download (src/download.rs)

download-browser-could-not-read-file = the browser could not read that file

## morse (src/app/morse.rs)

morse-translate = TRANSLATE
morse-practice = PRACTICE
morse-learn = LEARN
morse-morse = MORSE
morse-text-morse = Text → Morse
morse-morse-text = Morse → Text
morse-per-character-space-between-characters = `.` and `-` per character, a space between characters, ` / ` between words. The ITU alphabet, digits, punctuation and the common prosigns.
morse-reference-translator-available-desktop-build = The reference translator is available in the desktop build.
morse-play-what-you-type-your = Play what you type, to your own speakers
morse-play = PLAY
morse-stop = STOP
morse-every-character-learned-well-done = Every character learned — well done.
morse-drill-needs-sound-which-desktop = The drill needs sound, which the desktop build has.
morse-reveal = REVEAL
morse-reset = RESET
morse-replay = REPLAY
morse-what-did-you-hear = What did you hear?
morse-check = CHECK
morse-press-new-hear-character = Press NEW to hear a character.
morse-tone = Tone
morse-speed = Speed
morse-spacing = Spacing
morse-characters-go-out-speed-above = Characters go out at the speed above; this is the speed the *gaps* are sent at, so a beginner hears the letter as a whole before they can send it that fast.
morse-playback-available-desktop-build = Playback is available in the desktop build.

## ADS-B panel: aircraft detail rows.

adsb-row-altitude = altitude
adsb-row-track = track
adsb-row-turning = turning
adsb-row-squawk = squawk
adsb-row-range = range
adsb-row-position = position
adsb-row-signal = signal
adsb-row-frames = frames
rade-transmitting = transmitting
decodes-odd = ODD
decodes-new = NEW
js8-new = NEW

## AIS panel: vessel detail rows.

ais-row-status = status
ais-row-speed = speed
ais-row-course = course
ais-row-heading = heading
ais-row-turning = turning
ais-row-altitude = altitude
ais-row-size = size
ais-row-draught = draught
ais-row-destination = destination
ais-row-range = range
ais-row-signal = signal
ais-row-type = type
ais-row-position = position
ais-row-messages = messages
ism-off = OFF
textmodem-frequency-shifted = frequency-shifted
winlink-to-heading = TO

## adsbmap: messages with values (src/adsb_map.rs)

adsbmap-ft-min =
    {""}
    { $vr } ft/min

## aismap: messages with values (src/ais_map.rs)

aismap-heard-no-position-newer-than = { $heard } heard — no position newer than { $drop_map_s }s
aismap-stations-heard-waiting-position-report = { $heard } stations heard — waiting for a position report
aismap-km-across = { $lon_span }° · ~{ $km } km across

## alerts: messages with values (src/app/alerts.rs)

alerts-alarming = alarming on { $device }
alerts-alarm-device-unavailable = alarm device unavailable: { $error }

## awards: messages with values (src/app/awards.rs)

awards-worked = { $w } worked
awards-confirmed = { $value } confirmed

## bands: messages with values (src/app/bands.rs)

bands-forecast-hamqsl-com-old = · forecast from HAMQSL.com, { $value } old
bands-ncdxf-iaru-next-beacon-s = NCDXF/IARU · next beacon in { $now } s
bands-hearing-means-path-open-each = { $callsign } at { $location } — hearing it means the { $band } path is open. Each beacon steps up a band every 10 s, so this row changes every slot.
bands-radiants-placed = radiants placed for { $lat }°, { $lon }°
bands-zhr = ZHR { $zhr }
bands-radiant = radiant { $alt_deg }° { $az_deg }
bands-radiant-down = radiant down ({ $alt_deg }°)
bands-km-s-parent-peak-rate =
    { $value } ({ $code }) — { $velocity_kms } km/s, parent { $parent }. Peak rate ZHR { $zhr } around { $peak }.

    A radiant above the horizon means the trails can reach you; a fast shower leaves longer-lived ionised trails for meteor scatter on 6 m and 2 m, and the brief 10 m openings.

## drm: messages with values (src/app/drm.rs)

drm-not-decoding = { $channel } not decoding
drm-symbols = { $value } symbols · { $frame }
drm-not-decodable = { $value } — not decodable
drm-broadcaster-s-clock-utc = broadcaster's clock { $year }-{ $month }-{ $day } { $hour }:{ $minute } UTC

## duopower: messages with values (src/app/duo_power.rs)

duopower-max-fdm-duo-s-full =
    MAX: the FDM-DUO's full output, about 8 W. The setting to use with the HR500 in line.

    { $base }
duopower-t-5-w-hr500-line-will =
    5 W. With the HR500 in line it will not reach full output on most bands at this setting: use MAX, and Drive to bring it down.

    { $base }
duopower-w-running-barefoot-qrp =
    { $code } W, for running barefoot or QRP.

    { $base }

## frame: messages with values (src/app/frame.rs)

frame-re-enables-transmit-after-1 = Re-enables transmit after { $swr }:1. Check the antenna first: if the fault is still there, the next transmission stops too.
frame-sdroxide-has-been-released = SDRoxide { $version } has been released — this is { $sdroxide_version }.
frame-reconnecting-s = Reconnecting in { $value }s…

## gridtracker: messages with values (src/app/grid_tracker.rs)

gridtracker-worked = { $worked } worked
gridtracker-confirmed = { $confirmed } confirmed
gridtracker-heard = { $heard } heard

## hd: messages with values (src/app/hd.rs)

hd-decode = Decode { $value } ({ $codec_label })

## hfdl: messages with values (src/app/hfdl.rs)

hfdl-khz-already-has-chip = { $cur_khz } kHz already has a chip
hfdl-save-current-channel-khz-one = Save the current channel ({ $cur_khz } kHz) as one of your own chips
hfdl-aircraft = { $aircraft } aircraft
hfdl-bursts = { $bursts } bursts
hfdl-decodes-2 = { $decodes } decodes
hfdl-fixed = { $fec } fixed

## ism: messages with values (src/app/ism.rs)

ism-not-decoded-yet = { $hint } — not decoded yet
ism-listen-tunes-radio-there-one = Listen on { $value } — tunes the radio there. One band at a time: they are too far apart for any receiver to cover two at once.
ism-watch-hz-around-band-centre = Watch { $value }Hz around the band centre, whatever the band would have asked for. Narrower fits a receiver that cannot deliver the band's own width; wider reaches devices sitting further off centre. Your receiver has to hand over about a third more than this.
ism-rtl-433 = rtl_433 — { $why }
ism-rtl-433-khz = rtl_433 { $value } · { $khz } kHz
ism-your-own = { $flex } of your own
ism-decoders-decoded = { $decoders } decoders ({ $flex }), { $decodes } decoded
ism-handled-rtl-433 = handled by rtl_433: { $names }
ism-flex = flex: { $error }
ism-window-mhz-mhz-wide = window { $mhz } MHz, { $e62 } MHz wide
ism-bursts-decoded = { $bursts } bursts, { $decodes } decoded
ism-devices = { $ism_reports } devices
ism-id-mhz-db-heard-first =
    { $protocol } id { $device }
    { $e6 } MHz { $snr_db } dB heard { $value }×
    first heard { $first_at } ago

## logbook: messages with values (src/app/logbook.rs)

logbook-confirmations-downloaded-newly-confirmed = Confirmations: { $recs } downloaded, { $matched } newly confirmed
logbook-qso = { $qso_log } QSO
logbook-worked-before = ⚠ WORKED BEFORE ({ $dupe_band })
logbook-utc-qso = { $oldest }–{ $newest } UTC · { $value } QSO

## app: messages with values (src/app/mod.rs)

app-radio = radio { $id }

## morse: messages with values (src/app/morse.rs)

morse-could-not-start-morse-audio = could not start the Morse audio thread: { $error }
morse-no-audio-output = no audio output: { $error }
morse-about-s = about { $farnsworth_wpm } s
morse-characters-play = Characters in play: { $set }
morse-score-run = Score: { $correct } / { $total }. Run: { $streak } of { $value }.
morse-correct = correct — { $value }
morse-was = that was { $value }

## net: messages with values (src/app/net.rs)

net-synced-z-new-already-log = Synced { $when }Z: { $added } new, { $matched } already in the log
net-wavelog-2 = Wavelog: { $line }
net-full-resync-stopped-nothing-changed = Full resync stopped, nothing changed: { $error }
net-full-resync-applied-added-updated = Full resync applied: { $add } added, { $update } updated, { $remove } removed, { $mark } of yours marked as in Wavelog
net-wavelog-3 = Wavelog: { $line }
net-adif-import-failed = ADIF import failed: { $error }
net-not-unicode-read = {" "}(not Unicode; read as { $enc })
net-received-reports-swl-left-out = , { $n } received reports (SWL) left out
net-adif-import-added-duplicates-skipped = ADIF import: { $added } added, { $skipped } duplicates skipped{ $reports }{ $assumed }
net-channel-import-failed = Channel import failed: { $error }
net-line-s-skipped = , { $n } line(s) skipped
net-not-unicode-read-2 = {" "}(not Unicode; read as { $enc })
net-channel-import-channel-s-read = Channel import: { $n } channel(s) read{ $skipped }{ $assumed }

## acars: messages with values (src/app/panels/acars.rs)

acars-frames-bad = { $frames } frames · { $bad } bad
acars-mode = mode { $mode }

## adsb: messages with values (src/app/panels/adsb.rs)

adsb-aircraft-3 = { $aircraft } aircraft
adsb-frames = { $frames } frames
adsb-preambles = { $preambles } preambles
adsb-bad-crc = { $bad_crc } bad CRC
adsb-tune = TUNE { $mhz }
adsb-ft-barometric = { $ft } ft barometric

## ais: messages with values (src/app/panels/ais.rs)

ais-vessels-3 = { $vessels } vessels
ais-messages = { $messages } messages
ais-slots = { $bursts } slots
ais-bad-fcs = { $bad_fcs } bad FCS
ais-ais-mhz = AIS { $value } ({ $mhz } MHz) — { $value2 }
ais-ais-mhz-slots-messages-floor = AIS { $value } ({ $mhz } MHz) — { $bursts } slots, { $messages } messages, floor { $floor_dbfs } dBFS
ais-ais = AIS { $value }
ais-tune = TUNE { $mhz }
ais-ships-heard-are-khz-off = the ships heard are { $khz } kHz off frequency ({ $mhz } ppm at 162 MHz) — set the front end's frequency correction, or transmissions past about 5 kHz will stop decoding
ais-over-ground = { $value }° over ground
ais-dbfs-db-snr = { $rssi_dbfs } dBFS, { $snr_db } dB SNR
ais-last-type-ais = { $messages } (last type { $last_type }, AIS { $channel })
ais-ais-2 = AIS { $ch }
ais-mhz-marine-channel-ship-alternates = { $mhz } MHz — marine channel { $ch }. A ship alternates between the two, so switching one off halves how often every vessel is heard.

## aprs: messages with values (src/app/panels/aprs.rs)

aprs-object = object from { $reported_by }
aprs-via = via { $via }
aprs-object-reported = object reported by { $reported_by }
aprs-digit-s-blanked = ± { $ambiguity } digit(s) blanked
aprs-wind-m-s = wind { $value }° { $value2 } m/s
aprs-gust = gust { $value }
aprs-rain-mm-h = rain { $value } mm/h
aprs-frame-s-last-ago = { $packets } frame(s), last { $last_heard } ago
aprs-via-2 = via { $via }
aprs-bad = { $bad_frames } bad
aprs-other = { $non_aprs } other
aprs-sent = sent ×{ $tries }
aprs-stn = { $stations } stn
aprs-queued = { $tx_queue } queued

## atchat: messages with values (src/app/panels/atchat.rs)

atchat-master = master { $value }
atchat-last-heard-s-ago-click = { $status } — last heard { $age_s }s ago · click to open a direct-message tab
atchat-close-tab = close the { $peer } tab
atchat-direct-message = direct message to { $value }
atchat-cannot-display = cannot display { $path }

## cw: messages with values (src/app/panels/cw.rs)

cw-click-decoder-neural-one-copies =
    { $hint }

    Click for the { $neural } decoder. The neural one copies further down and reads hand-sent CW a timing fit will not accept; the timing one is the only one that copies the accented letters — Ä, Ö, Å, Ü, É — because the model has no output class for them.
cw-idle-s = IDLE { $idle }s

## decodes: messages with values (src/app/panels/decodes.rs)

decodes-save-decodes-list-adif-received = Save the { $n } decodes in the list as ADIF — received reports, not contacts. Decodes that name no sender are left out.
decodes-save-decodes-list-csv-one = Save the { $n } decodes in the list as CSV, one row each
decodes-session-qso = Session: { $session } QSO
decodes-save-whole-logbook-qso-adif = Save the whole logbook ({ $logged } QSO) as ADIF
decodes-save-whole-logbook-qso-text = Save the whole logbook ({ $logged } QSO) as text
decodes-hz-slots = { $audio_hz } Hz · { $odd } slots
decodes-your-slot-timing-against-stations =
    Your slot timing against the stations you are hearing.
    { $after }

    It covers the whole receive path, so a slow audio or network chain counts the same as a wrong clock. Under 0.5 s is comfortable.
decodes-pile-up = PILE-UP { $fox_queue }
decodes-queue = QUEUE { $call_queue }
decodes-db-hz-click-remove =
    { $waiting } · { $snr_db } dB · { $audio_hz } Hz{ $unwrap_or_default }
    Click to remove

## js8: messages with values (src/app/panels/js8.rs)

js8-every-speed-being-decoded-marks = Every speed is being decoded; ▸ marks { $speed }, the one you transmit at
js8-every-speed-being-decoded-click = Every speed is being decoded. Click to transmit at { $speed } instead
js8-transmit-decode = Transmit and decode at { $speed }
js8-beaconing-every-min-click-stop = Beaconing every { $hb_min } min — click to stop
js8-last-beacon-went-out-hz = The last beacon went out at { $hz } Hz — a free slot in the { $value }–{ $value2 } Hz heartbeat sub-band, chosen so it lands clear of the signals being decoded.
js8-frames-s = { $tx_frames_pending }/{ $tx_frames_total } frames · { $left }s
js8-decoded = Decoded at { $speed }
js8-frames = … ({ $frames } frames)
js8-sent = Sent at { $speed }
js8-partial-frames-sent = partial ({ $frames } frames sent)
js8-reply-2 = Reply to { $value }: “{ $value2 }”
js8-address-composer = Address the composer at { $value }
js8-auto-resp-on = Answering group messages with { $n } { $n ->
        [one] rule
       *[other] rules
    } — click to stop
js8-auto-resp-ready = { $n } auto-response { $n ->
        [one] rule
       *[other] rules
    } ready — click to start answering
js8-auto-pending-next-slot = { $sent }/{ $total } · next slot
js8-auto-pending-next = { $sent }/{ $total } · next { $left }

## macros: messages with values (src/app/panels/macros.rs)

macros-f-sends = F{ $i }: sends “{ $text }”
macros-sends = Sends “{ $text }”
macros-most = { $value } is the most

## navtex: messages with values (src/app/panels/navtex.rs)

navtex-repaired-lost = { $repaired } repaired · { $lost } lost
navtex-characters-lost = { $lost } characters lost

## packet: messages with values (src/app/panels/packet.rs)

packet-bad = { $bad_frames } bad
packet-link-layer = The link layer is in { $link }
packet-via = {" "}via { $via }
packet-unacked = { $unacked } unacked
packet-retry = retry { $retries }

## pi4: messages with values (src/app/panels/pi4.rs)

pi4-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] beacon
       *[other] beacons
    }
pi4-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } ago

## rade: messages with values (src/app/panels/rade.rs)

rade-snr-db = SNR { $snr_db } dB
rade-offset-hz = offset { $freq_offset_hz } Hz
rade-heard = heard { $call }
rade-samples-dropped-between-receiver-decoder = ⚠ { $dropped } samples dropped between the receiver and the decoder — this machine is not keeping up with the neural decode.

## textmodem: messages with values (src/app/panels/text_modem.rs)

textmodem-char-s = { $chars_per_sec } char/s
textmodem-char-s-hz-wide = { $value } — { $chars_per_sec } char/s, { $bandwidth_hz } Hz wide, { $keyed }

## vdl2: messages with values (src/app/panels/vdl2.rs)

vdl2-frames = { $frames } frames
vdl2-bursts = { $bursts } bursts
vdl2-sync = { $syncs } sync
vdl2-hdlc-bad = { $hdlc_bad } HDLC bad
vdl2-bad-fcs = { $fcs_bad } bad FCS
vdl2-mhz-bursts-frames-noise-floor = { $mhz } MHz, { $what } — { $bursts } bursts, { $frames } frames, noise floor { $floor_dbfs } dBFS
vdl2-tune = TUNE { $mhz }
vdl2-held-2 = HELD +{ $since }
vdl2-use-worldwide-where-every-link = { $what } — in use worldwide, and where every link starts. The one to keep if you keep only one.
vdl2-assigned-which-channels-carry-anything = Assigned to an { $what }. Which channels carry anything depends on where you are: leave them all on unless you know otherwise.
vdl2-mhz-db-snr-dbfs-evm = { $mhz } MHz · { $snr_db } dB SNR · { $rssi_dbfs } dBFS · EVM { $evm_deg }° · { $freq_err_hz } Hz · { $rs_corrected } RS symbols fixed
vdl2-characters = { $parity_errors } characters
vdl2-parameter-sdroxide-does-not-read = { $unknown } { $unknown ->
        [one] parameter
       *[other] parameters
    } SDRoxide does not read — shown above as identifier and bytes

## wefax: messages with values (src/app/panels/wefax.rs)

wefax-lines = { $lines } lines
wefax-lines-held = { $live_size } lines held
wefax-pixels-per-line = { $width } pixels per line
wefax-shift-picture-pixels = Shift the picture { $px } pixels
wefax-charts-are-saved-click-copy =
    Charts are saved { $place }

    Click to copy the path
wefax-finished-charts-are-saved-collect = Finished charts are saved { $place } and collect here.
wefax-pixels-right-click-delete =
    { $when_full }
    { $size } × { $size2 } pixels
    { $value }

    Right-click to delete
wefax-older-store = { $older } older in the store
wefax-older-load-more = { $older } older — load more
wefax-published-carrier-khz-dial-khz = Published carrier { $f } kHz → dial { $value } kHz USB
wefax-delete-chart-2 = Delete this chart { $dir }

## widgets: messages with values (src/app/panels/widgets.rs)

widgets-cq-zone-itu-zone = { $continent } · CQ zone { $cq_zone } · ITU zone { $itu_zone }
widgets-new-entity = New entity on { $band_label }
widgets-worked-before = Worked before on { $band_label }
widgets-calling-cq-includes-you = Calling CQ { $target } — that includes you
widgets-calling-cq-not-aimed-you = Calling CQ { $target } — not aimed at you

## wspr: messages with values (src/app/panels/wspr.rs)

wspr-rx-heard-us = { $heard_us } rx · { $heard_us2 } heard us
wspr-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] beacon
       *[other] beacons
    }
wspr-beaconing-duty = beaconing — { $wspr_power_dbm }, { $wspr_tx_percent }% duty
wspr-listening-duty = listening ({ $wspr_tx_percent }% duty)
wspr-beacon-one-two-minute-slot = Beacon in one two-minute slot out of every { $pct } — { $pct2 }% of them, evenly spaced — at { $wspr_power_dbm }. Which slot the cycle starts on is drawn from your callsign, so two stations running this program do not start together.
wspr-transmitting-set-general-tab-settings = Transmitting as { $call } in { $sent } — set on the General tab of Settings.
wspr-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } ago

## publicsdr: messages with values (src/app/publicsdr.rs)

publicsdr-antenna =
    { $address }
    { $device }
    antenna: { $antenna }
    { $e3 }
publicsdr-receivers-fetched-ago = { $entries } receivers · { $per_net } · fetched { $age } ago
publicsdr-showing-search-narrow = showing { $shown } of { $n } — search to narrow it
publicsdr-match = { $n } match
publicsdr-map-status = { $shown } on the map · { $unplaced } without a position
publicsdr-opening-another-radio = Opening { $entry } as another radio…

## qo100: messages with values (src/app/qo100.rs)

qo100-khz-3 = +{ $park_lo_hz } … +{ $park_hi_hz } kHz
qo100-before-switching-tune-so-beacon = Before switching ON: tune so the beacon's two lobes sit between +{ $park_lo_hz } and +{ $park_hi_hz } kHz in the strip (the shaded lane).
qo100-receiver-currently-listening-around-mhz = The receiver is currently listening around { $mhz } MHz — nowhere near the 10489.750 MHz beacon, so there is nothing here to draw. The decoder keeps searching regardless.
qo100-receiver-currently-covers-only-khz = receiver currently covers only ±{ $khz } kHz here — the rest of the strip stays blank
qo100-null-db-sym-snr-db = null { $est_null_depth_db } dB sym { $est_symmetry } snr { $est_snr_db } dB
qo100-hz-s-hz-s-de = { $est_drift_hz_s } Hz/s { $est_drift_accel_hz_s2 } Hz/s² (de-rotated before decode)
qo100-hz-s-de-rotated-before = { $est_drift_hz_s } Hz/s (de-rotated before decode)
qo100-found-empty = { $est_updates } found, { $est_misses } empty
qo100-last-s-ago = {"   "}last { $auto_last_hz } ({ $auto_last_unix }s ago)
qo100-over-correction = { $auto_total_hz } over { $auto_applies } { $auto_applies ->
        [one] correction
       *[other] corrections
    }{ $last }
qo100-sync-32-err = sync { $sync_bit_errors } / 32 err ×{ $sync_matches }
qo100-frame-buffer = frame buffer { $frame_fill }%
qo100-blocks-tried-decoded = { $blocks_tried } blocks tried, { $blocks_locked } decoded
qo100-last-applied-s-ago-hz = last applied { $ago }s ago: { $old } → { $new } Hz ({ $old2 } Hz)
qo100-locked-block-tried-locked = locked — { $blocks_tried } { $blocks_tried ->
        [one] block
       *[other] blocks
    } tried, { $blocks_locked } locked
qo100-searching-block-tried-none-locked = searching — { $blocks_tried } { $blocks_tried ->
        [one] block
       *[other] blocks
    } tried, none locked yet

## rds: messages with values (src/app/rds.rs)

rds-auto = Auto ({ $standard })
rds-type = type { $class }
rds-groups = · { $groups } groups
rds-blocks-error = · { $ber } % of blocks in error
rds-clean-corrected-lost = { $blocks_ok } clean · { $blocks_corrected } corrected · { $blocks_bad } lost

## sat: messages with values (src/app/sat.rs)

sat-utc-local = { $unix } UTC ({ $offset } local)
sat-aos = AOS { $rise_unix } { $rise_unix2 }
sat-los = LOS { $set_unix } { $set_unix2 }
sat-above-horizon = { $rise_unix } above the horizon
sat-min = in { $value } min
sat-h-min = in { $value } h { $value2 } min
sat-searching = searching… { $next } of { $entries }
sat-locked-2 = ● LOCKED — { $t }
sat-pass-until-max = Pass until { $set_unix } · max { $max_el }°
sat-next-pass-rises-max = Next pass { $rise_unix } { $rise_unix2 } ({ $now }) · rises { $rise_az }° { $rise_az2 } · max { $max_el }°
sat-rigctld-client-connected-satellite-software = ⚠ { $clients } rigctld { $clients ->
        [one] client
       *[other] clients
    } connected — satellite software steering the dial there would correct Doppler twice
sat-rotator-antenna-az-el = Rotator: antenna at { $az }° az { $el }° el
sat-rotator-2 = Rotator: { $error }
sat-up-now-2 = ● up now, { $el }°
sat-inv = { $mode } · inv
sat-norad-2 = NORAD { $norad_id }

## scanner: messages with values (src/app/scanner.rs)

scanner-holding-mhz = holding { $here } MHz
scanner-scanning-mhz = scanning · { $here } MHz

## spectrum: messages with values (src/app/spectrum.rs)

spectrum-renderer-will-not-hold-texture = This renderer will not hold a texture wider than { $max_texture_dim } pixels.
spectrum-mem = Mem: { $f } / { $value }
spectrum-mem-2 = Mem: { $value }

## speech: messages with values (src/app/speech.rs)

speech-voice-2 = Voice: { $voice }
speech-speech-unavailable = Speech is unavailable: { $error }

## spots: messages with values (src/app/spots.rs)

spots-transmissions = { $stations } transmissions

## winlink: messages with values (src/app/winlink.rs)

winlink-last-session-failed = last session failed — { $err }
winlink-last-session-received-sent = last session: { $last_received } received, { $last_sent } sent
winlink-empty = { $to_lowercase } is empty
winlink-showing = showing { $entries } of { $total }
winlink-fetching = fetching { $mid }…
winlink-from-2 = from { $msg } { $date }
winlink-attachment-bytes = attachment: { $att } ({ $data } bytes)

## aprsmap: messages with values (src/aprs_map.rs)

aprsmap-object =
    {""}
    object from { $reported_by }
aprsmap-heard-ago =
    {""}
    heard { $age } ago

## download: messages with values (src/download.rs)

download-reading = reading { $path }: { $error }

## help: messages with values (src/help.rs)

help-missing-image = {"["}missing image: { $path }]

## hfdlmap: messages with values (src/hfdl_map.rs)

hfdlmap-mhz =
    {""}
    { $e3 } MHz
hfdlmap-fix =
    {""}
    { $fixes } fix{ $es }
hfdlmap-heard-ago =
    {""}
    heard { $last_at } ago

## multi: messages with values (src/multi.rs)

multi-radio = Radio { $n }
multi-could-not-add-radio = Could not add a radio: { $error }
multi-was-closed-station = { $value } was closed at the station.
multi-could-not-open = Could not open { $peer }: { $error }
multi-has-tab-own-now = { $value } has a tab of its own now.

## remote: messages with values (src/remote.rs)

remote-protocol-error = protocol error: { $error }

## solarnet: messages with values (src/solar3d/net.rs)

solarnet-bad-message = bad message: { $error }
solarnet-solar-protocol-mismatch-server-client = solar protocol mismatch: server { $proto }, client { $value }

## smeter: messages with values (src/widgets/smeter.rs)

smeter-forward-power-w-measured-radio =
    {""}

    Forward power — { $w } W, measured by the radio's own sensor.
smeter-po-radio-s-own-power =
    {""}

    PO — the radio's own power-output meter, at { $clamp } % of its full scale. A needle position as the radio reports it, not a calibrated wattage; what to watch is whether it falls while Drive stays put.
smeter-swr-1-match-antenna-presenting =
    {""}

    SWR — { $swr }:1, the match the antenna is presenting, as the radio measures it. Under 2:1 is unremarkable; past 3:1 the scale turns red.
smeter-received-signal-strength-s9-db = Received signal strength: S9 + { $over } dB ({ $s_dbm } dBm in the passband).
smeter-received-signal-strength-s-dbm = Received signal strength: S{ $value } ({ $s_dbm } dBm in the passband).
smeter-radio-reports-c =
    {""}

    The radio reports { $t } °C.
smeter-ps-puresignal-correcting-db-compression =
    {""}

    PS — PureSignal is correcting { $correction_db } dB of compression{ $held } (the feedback matched the transmission at { $score }).
smeter-ps-puresignal-switched-but-has =
    {""}

    PS −− — PureSignal is switched on but has not found the transmission in the feedback path (best match { $score }), so the transmitter is going out uncorrected. Check that a sample of the amplifier's output really reaches a receive input the T/R switch does not take away on transmit, and that its attenuator is not swallowing it. The loop listens on this receiver only — a coupler wired to a second receiver (RX2, ADC2) is never read as the feedback.

## specview: messages with values (src/widgets/spectrum_view.rs)

specview-rit-hz = RIT { $hz } Hz
specview-xit-hz = XIT { $hz } Hz

## decodes: nested messages

decodes-you-transmit-s-everyone-else = You transmit { $off } s { $after } everyone else — the usual reason calls go unanswered. Sync your computer clock.

## publicsdr: nested messages

publicsdr-noise-floor-score = noise-floor score { $snr }
publicsdr-up-khz-i-q = up to { $khz } kHz of I/Q

## txaudio: more messages

txaudio-gr-gate-de-ess-comp = GR gate { $gate_gr_db } de-ess { $deess_gr_db } comp { $comp_gr_db } lim { $limiter_gr_db }

## amp: more messages

amp-h-step = ~{ $l_step } µH · step { $l_step2 }
amp-pf-step = ~{ $c_step } pF · step { $c_step2 }

## sstv: more messages

sstv-bit-octets-chunks-first-pass = { $from } · { $filename } · { $width }×{ $height } { $bits_per_pixel }-bit · { $media_type } / { $content_encoding } · { $encoded_size } octets in { $chunk_count } chunks ({ $chunks_first_pass } first pass) · session { $session }
sstv-baud-cpfsk-hz-khz-occupied = { $value } — { $symbol_rate } baud CPFSK, ±{ $deviation_hz } Hz, { $bandwidth_hz } kHz occupied bandwidth

## Clock, and a few short labels with values.

time-utc-and-local = { $utc } UTC · { $local } local
sat-in-seconds = in { $s } s
winlink-to-list = to { $to }
winlink-cc-list = cc { $cc }
spots-n-of-total = { $n } of { $total }
wefax-n-of-total = { $n } of { $total }
adsb-range-bearing = { $km } km at { $bearing }°
ais-range-bearing = { $km } km at { $bearing }°

## More values from sdroxide-types, as pickers name them (src/i18n/labels.rs).
rtl-sdr-hf-mode-off = Off (tuner only, 24 MHz up)
rtl-sdr-hf-mode-auto = Automatic
rtl-sdr-hf-mode-direct-q = Direct sampling (Q branch)
rtl-sdr-agc-manual = Manual (no AGC)
rtl-sdr-agc-tuner = Tuner AGC
rtl-sdr-agc-rtl = RTL digital AGC
rtl-sdr-agc-both = Tuner + RTL AGC
mode-control-cat = CAT
mode-control-radio = Radio controlled
line-state-none = None
line-state-high = High
line-state-low = Low
elad-tx-input-usb-audio = USB audio
elad-tx-input-mic = Microphone
elad-tx-input-auto = Auto
elad-tx-input-leave = Leave as set on the radio
tr-usdx-audio-one-cable = One cable (audio in the CAT stream)
tr-usdx-audio-sound-card = USB sound card (3.5 mm jack)
spy-server-format-uint8 = 8-bit (lowest bandwidth)
spy-server-format-int16 = 16-bit
spy-server-format-float32 = 32-bit float
sound-format-iq = IQ (stereo)
sound-format-demod-audio = Demod audio
sdr-play-duo-tuner-tuner1 = Tuner 1 (50 Ohm / Hi-Z)
sdr-play-duo-tuner-tuner2 = Tuner 2 (50 Ohm)
sdr-play-duo-role-diversity = A second aerial (diversity / QRM suppression)
sdr-play-duo-role-second-radio = A second radio, on its own frequency
sdr-play-agc-off = Off (manual IF gain)
sdr-play-agc-hz5 = 5 Hz
sdr-play-agc-hz50 = 50 Hz
sdr-play-agc-hz100 = 100 Hz
rotator-protocol-rotctld = Hamlib rotctld (TCP)
rotator-protocol-pst-rotator = PSTRotator (UDP)
pluto-ptt-off = Off
pluto-ptt-gpo01 = GPO0 = RX, GPO1 = TX
pluto-ptt-gpo23 = GPO2 = RX, GPO3 = TX
pluto-duplex-fdd = FDD (both at once)
pluto-duplex-tdd = TDD (one at a time)
pluto-agc-manual = Manual
pluto-agc-slow-attack = Slow attack
pluto-agc-fast-attack = Fast attack
pluto-agc-hybrid = Hybrid
parity-none = None
parity-even = Even
parity-odd = Odd
panadapter-tap-antenna = Antenna (shared with the radio)
panadapter-tap-if-output = The radio's I.F. output
panadapter-audio-attached = The attached receiver
panadapter-audio-transceiver = The transceiver
kenwood-send-ts2000 = TS-2000 style (TX;)
kenwood-send-ts590 = TS-590 style (TX1;)
digi-mode-usb = USB
digi-mode-data = DIGI
digi-mode-radio = Radio controlled
cw-keying-cat = Rig keyer (CAT)
cw-keying-audio = Sound card (MCW)
audio-route-both = Both channels
audio-route-left = Left only
audio-route-right = Right only
amp-model-hr500 = Hardrock-500
amp-model-hr50 = Hardrock-50 (untested)
alert-sound-ding = Ding
alert-sound-two-tone = Two-tone
alert-sound-triplet = Triplet
alert-sound-warble = Warble
alert-sound-digital = Digital
alert-reply-tone = Tone
alert-reply-voice = Voice
alert-reply-both = Tone + voice
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
icom-model-other = Other (set the address by hand)
backend-auto = Auto-detect (SoapySDR / CAT)
backend-soapy = SoapySDR
backend-cat = CAT / Audio
backend-hpsdr = HPSDR (network)
backend-tci = TCI (network)
backend-icom-net = Icom LAN (network)
backend-smart-sdr = SmartSDR / FlexRadio (network)
backend-pluto = PlutoSDR (network)
backend-rtl-sdr = RTL-SDR (USB)
backend-rtl-tcp = RTL-SDR over rtl_tcp (network)
backend-spy-server = SpyServer (network)
backend-spy-server-vfo = SpyServer VFO+FFT, low bandwidth (network)
backend-kiwi-sdr = KiwiSDR / Web-888 (network)
backend-rx888 = RX-888 (USB)
backend-airspy-hf = Airspy HF+ (USB)
backend-airspy = Airspy R2 / Mini (USB)
backend-hack-rf = HackRF One / Pro (USB)
backend-sdr-play = SDRplay RSP (USB)
backend-elad = ELAD FDM-DUO / FDM-S (USB)
backend-lime = LimeSDR + LimeRFE (LimeSuite)
backend-hydra-sdr = HydraSDR RFOne (USB)
backend-fobos = RigExpert Fobos SDR (USB)
backend-usb-audio = USB audio radio (sound card)
backend-none = Not configured
cat-family-xiegu = Xiegu
cat-family-icom = Icom
cat-family-yaesu = Yaesu
cat-family-kenwood = Kenwood
cat-family-elecraft = Elecraft
cat-family-elad = ELAD
cat-family-qrp-labs = QRP Labs
cat-family-rs-hfiq = RS-HFIQ
cat-family-tr-usdx = (tr)uSDX
cat-family-rigctld = Hamlib rigctld (network)
cat-family-flrig = flrig (network)

## Values from sdroxide-types drawn on chips, menus and lists (src/i18n/labels.rs).
agc-mode-off = Off
agc-mode-slow = Slow
agc-mode-med = Med
agc-mode-fast = Fast
airspy-gain-linearity = Linearity (best strong-signal handling)
airspy-gain-sensitivity = Sensitivity (best weak-signal)
ais-kind-class-a = Class A
ais-kind-class-b = Class B
ais-kind-base-station = base station
ais-kind-sar-aircraft = SAR aircraft
ais-kind-aid-to-navigation = aid to navigation
ais-kind-sart = SART / MOB / EPIRB
ais-kind-craft = associated craft
alert-event-called = A station is calling me
alert-event-cq = A CQ I could answer
alert-event-new-dxcc = New DXCC entity
alert-event-new-dxcc-band = New entity on this band
alert-event-new-grid = New grid square
audio-routing-both = Left + right
audio-routing-left = Left only
audio-routing-right = Right only
button-mode-momentary = Hold
button-mode-toggle = Toggle
contest-mode-none = None
contest-mode-eu-vhf = EU VHF Contest
converter-tx-off = Off while converting
converter-tx-transverter = Through the same converter
converter-tx-own = Its own offset
cw-engine-neural = NEURAL
cw-engine-timing = TIMING
decode-sort-none = None
decode-sort-signal = SNR
decode-sort-distance = Dist
decode-sort-country = Country
diversity-mode-cancel = Cancel — null a noise source
diversity-mode-combine = Combine — diversity reception
dxped-mode-normal = Normal
dxped-mode-hound = Hound
dxped-mode-fox = Fox
fail-safe-refuse-tx = Refuse to transmit
fail-safe-warn-only = Transmit anyway, warn
ftp-security-plain = FTP (no encryption)
ftp-security-explicit-tls = FTPS explicit (AUTH TLS)
ftp-security-implicit-tls = FTPS implicit
hpsdr-filter-board-none = None — outputs stay off
hpsdr-filter-board-n2adr = N2ADR filter board
hpsdr-filter-board-alex = Alex / Hermes band code (Zeus, HiQSDR, Quisk)
hpsdr-filter-board-custom = Custom — my own table below
hpsdr-io-rx-input-radio = Radio's own input
hpsdr-io-rx-input-io-board = IO board J9
hpsdr-io-rx-input-io-board-pure-signal = IO board J9, PureSignal on transmit
hydra-sdr-gain-linearity = Linearity (best strong-signal handling)
hydra-sdr-gain-sensitivity = Sensitivity (best weak-signal)
icom-rx-source-af = AF — the radio demodulates
icom-rx-source-if12k = 12 kHz IF — sdroxide demodulates
icom-scope-span-radio = As set on the radio
icom-scope-span-khz2-5 = 5 kHz (±2.5k)
icom-scope-span-khz5 = 10 kHz (±5k)
icom-scope-span-khz10 = 20 kHz (±10k)
icom-scope-span-khz25 = 50 kHz (±25k)
icom-scope-span-khz50 = 100 kHz (±50k)
icom-scope-span-khz100 = 200 kHz (±100k)
icom-scope-span-khz250 = 500 kHz (±250k)
icom-scope-span-khz500 = 1 MHz (±500k)
ism-family-weather = WEATHER
ism-family-meter = METERS
ism-family-home-auto = HOME
ism-family-lo-ra = LORA
ism-family-unidentified = UNKNOWN
ism-protocol-la-crosse-it = LaCrosse IT+
ism-protocol-fine-offset = Fine Offset
ism-protocol-bresser = Bresser
ism-protocol-z-wave = Z-Wave
ism-protocol-homematic = Homematic
ism-protocol-rtl433 = rtl_433
ism-protocol-unidentified = unidentified
js8-speed-normal = NORMAL
js8-speed-fast = FAST
js8-speed-turbo = TURBO
js8-speed-slow = SLOW
js8-auto-reply-to-sender = Sender
js8-auto-reply-to-group = Group
js8-auto-reply-to-as-written = As written
lime-aux-role-off = Not used
lime-aux-role-diversity = A second aerial (diversity / QRM suppression)
lime-aux-role-pure-signal = Transmit feedback (PureSignal predistortion)
lookup-provider-none = Off
lookup-provider-qrz = QRZ.com
lookup-provider-ham-qth = HamQTH
mail-folder-inbox = INBOX
mail-folder-outbox = OUTBOX
mail-folder-sent = SENT
mail-folder-archive = ARCHIVE
memory-sort-name = Name
memory-sort-freq = Freq
memory-sort-band = Band
memory-sort-stored = Stored
mouse-button-middle = Middle
mouse-button-extra1 = Back (extra 1)
mouse-button-extra2 = Forward (extra 2)
nr-level-off = Off
nr-level-low = Low
nr-level-medium = Mid
nr-level-high = High
nr-level-rnn-low = RNN Low
nr-level-rnn-med = RNN Med
nr-level-rnn-high = RNN High
nr-level-spec-low = SPEC Low
nr-level-spec-med = SPEC Med
nr-level-spec-high = SPEC High
nr-level-nr2-low = NR2 Low
nr-level-nr2-med = NR2 Med
nr-level-nr2-high = NR2 High
nr-level-df-low = DFNR Low
nr-level-df-med = DFNR Med
nr-level-df-high = DFNR High
nr-strength-low = Low
nr-strength-med = Med
nr-strength-high = High
orbit-rings-none = none
orbit-rings-curated = curated
orbit-rings-all = all
qso-step-idle = Idle
qso-step-wait-cq = Wait CQ
qso-step-calling-cq = Calling CQ
qso-step-tx-grid = Tx Grid
qso-step-tx-report = Tx Report
qso-step-tx-r-report = Tx R+Report
qso-step-tx-rr73 = Tx RR73
qso-step-tx73 = Tx 73
qso-step-confirming = Confirming
rds-standard-auto = Auto
rds-standard-rds = RDS
rds-standard-rbds = RBDS
region-r1 = Region 1 — Europe, Africa, Middle East, Northern Asia
region-r2 = Region 2 — The Americas
region-r3 = Region 3 — Southern & Eastern Asia, Australasia, Pacific
relative-mode-absolute = Absolute (fader)
relative-mode-sign-magnitude = Relative: sign/magnitude
relative-mode-twos-complement = Relative: two's complement
relative-mode-binary-offset = Relative: 64-centred
relay-link-off = None
relay-link-serial = USB relay board (serial)
relay-link-serial-lines = Serial RTS/DTR line
relay-link-hid = USB relay board (HID)
relay-link-cm108 = CM108/CM119 sound-card GPIO
relay-link-gpio = Linux GPIO line
relay-link-command = External command
relay-role-unused = Not used
relay-role-sdr-antenna = Ground the SDR antenna
relay-role-amplifier = Key an amplifier / T-R relay
relay-role-aux = Auxiliary
relay-role-band-decoder = Band decoder (filter / transverter)
rfe-channel-wb1000 = Wideband 1 – 1000 MHz
rfe-channel-wb4000 = Wideband 1 – 4 GHz
rfe-channel-ham0030 = HF, up to 30 MHz
rfe-channel-ham0070 = 6 m / 4 m (50 – 70 MHz)
rfe-channel-ham0145 = 2 m (140 – 150 MHz)
rfe-channel-ham0220 = 1.25 m (220 – 225 MHz)
rfe-channel-ham0435 = 70 cm (400 – 450 MHz)
rfe-channel-ham0920 = 33 cm (902 – 928 MHz)
rfe-channel-ham1280 = 23 cm (1220 – 1420 MHz)
rfe-channel-ham2400 = 13 cm (2.3 – 2.5 GHz)
rfe-channel-ham3500 = 9 cm (3.3 – 3.7 GHz)
rfe-channel-cell-band01 = Cellular band 1
rfe-channel-cell-band02 = Cellular band 2
rfe-channel-cell-band03 = Cellular band 3
rfe-channel-cell-band07 = Cellular band 7
rfe-channel-cell-band38 = Cellular band 38
rfe-channel-not-selected = None selected
rfe-link-off = Not connected
rfe-link-serial = Its own USB cable (serial)
rfe-link-board = Through the LimeSDR (GPIO / I²C)
rfe-mode-control-auto = Automatic (follow the cabling)
rfe-mode-control-rx = Always receive
rfe-mode-control-tx = Always transmit
rfe-mode-control-tx-rx = Always both (split connectors only)
rifp-encoding-auto = Auto
rifp-encoding-png = PNG
rifp-encoding-jpeg = JPEG
rifp-encoding-raw = Raw
rifp-encoding-rle8 = RLE8
rifp-encoding-zlib = Zlib
rifp-encoding-group4 = G4
rifp-encoding-group3 = G3
scan-kind-memories = MEM
scan-kind-range = RANGE
scan-resume-carrier = CARRIER
scan-resume-timed = TIMED
scan-resume-manual = MANUAL
sense-line-off = Not wired
sense-line-cts = CTS
sense-line-dsr = DSR
sense-line-dcd = DCD (carrier detect)
shift-simplex = SIMPLEX
shift-minus = −
shift-plus = +
smeter-trace-right-click = Right-click: direction and speed of the trace
smeter-trace-caption-flow = flow
smeter-trace-flow-right = RIGHT
smeter-trace-flow-left = LEFT
smeter-trace-flow-hint = The way the plot moves: LEFT puts the newest reading at the right edge, RIGHT at the left
smeter-trace-range-hint = Bottom and top of the plot in S-units: drag either knob, e.g. S3–S9 or S6–S9 to spread a band that lives there over the whole height. Receive only
smeter-trace-caption-speed = speed
smeter-trace-speed-hint = The waterfall's steps, in its proportions: SLOW holds 168 s, MEDIUM 30 s, FAST 15 s, FASTER 7.5 s, FASTEST 3.75 s
smeter-snr-right = middle { $mid } dB · { $khz } kHz · { $frames } fr
smeter-snr-off = S/N is off
smeter-snr-hover = S/N face: the passband's power over the noise floor of the S/N guide, in dB ((S+N)/N, like the S/N line); the bar is the mean over the cursor's width, the marker the strongest cell at its middle (±10 % of the width). The S-meter's other faces read absolute dBm instead.
speed-slow = Slow
speed-medium = Medium
speed-fast = Fast
speed-faster = Faster
speed-fastest = Fastest
tls-verify-system = System trust store
tls-verify-pinned = Pinned fingerprint
tls-verify-off = Don't verify
tone-mode-off = OFF
tone-mode-ctcss = CTCSS
tone-mode-dcs = DCS
tx-voice-stage-gate = Gate
tx-voice-stage-eq = EQ
tx-voice-stage-de-ess = De-esser
tx-voice-stage-comp = Compressor
upload-naming-sequence = image1 … imageN (rotate)
upload-naming-original = Original file name
wheel-action-zoom = Zoom
wheel-action-tune = Tune
wheel-action-none = Nothing

## jtty (src/app/panels/jtty.rs)

jtty-starting-receiver = starting the JTTY receiver…
jtty-jtty = JTTY
jtty-receive-audio-level = Receive audio level
jtty-n-rx = { $n } rx
jtty-listening-transmissions-start-any-time = Listening. JTTY transmissions start at any time — a message appears here a moment after it is heard.
jtty-tx = TX
jtty-sending = {" "}SENDING{" "}
jtty-tx-2 = {"   "}TX{"   "}
jtty-call-cq = {" "}CALL CQ{" "}
jtty-message-sent-once-press-tx = The message is sent once when you press TX — JTTY is asynchronous, so there is no period to wait for and nothing repeats.

## frame, settings/remote, js8, top_bar: certificates, sent JS8, FLEX tuner

frame-certificate-not-signed-authority = The server's certificate is not signed by an authority this client trusts — typically a self-signed one. It presented this certificate (SHA-256):
frame-compare-certificate-trust-only-if-match = Compare it with your server's — e.g. openssl x509 -noout -fingerprint -sha256 -in cert.pem — and trust it only if they match. From then on this exact certificate is accepted from this server and any other is refused.
frame-trust-certificate-connect = Trust this certificate and connect
settings-remote-trusted-certificates = Trusted certificates
settings-remote-forget = Forget
settings-remote-forget-hint = Go back to ordinary checking against the public certificate authorities. A server still on this certificate will ask to be trusted again.
js8-transmitted-by-this-station = Transmitted by this station
topbar-swr-match-antenna-presents =
    The match the antenna presents, as the radio measures it — live while the radio is on the air, whoever keyed it: an ATU tune, TUNE, PTT or the radio itself. Dimmed, it is the last reading, from the last over.

    Green to { $good }:1; yellow up to the SWR guard's { $limit }:1; red past it. With an external tuner, bypass the internal one and tune for the shortest bar.
topbar-atu-idle = No tune since the radio started.
topbar-atu-tuning = Tuning — the radio is transmitting a carrier into the tuner.
topbar-atu-tuned-from-memory = Matched, from the tuner's memory for this frequency.
topbar-atu-tuned = Matched.
topbar-atu-bypass = Out of circuit.
topbar-atu-failed = The last tune found no match.
topbar-atu-aborted = The last tune was cut short.
topbar-atu-built-in-tuner =
    The radio's built-in antenna tuner.

    { $state }
topbar-atu-start-tune = Start a tune. The radio keys itself with a carrier for as long as the match takes, at its tune power (the Tune level), so the antenna must be connected. Lit while the match it found is in circuit.
topbar-atu-bypass-hint = Take the tuner out of circuit. Lit while it is.

# Undocked windows (from sdroxide-brown)
frame-detached-title = { $flavor } — { $name }
frame-dock-band-menu-hint = DOCK — return the band and mode selector into the main window, the same as closing this one without hunting for its close box.
frame-dock-panel-hint = DOCK — put this panel back under the waterfall
frame-dock-spectrum-hint = DOCK — return this spectrum into the main window, the same as closing this one without hunting for its close box.
frame-every-module-own-window = every module is in its own window
frame-module-aux-panadapter = aux panadapter
frame-module-band-mode = band & mode
frame-module-controls = controls
frame-module-panadapter = panadapter
frame-module-panel = panel
multi-dock-all-windows = DOCK ALL WINDOWS
multi-module-panel = Operating panel
multi-undock-all-windows = UNDOCK ALL WINDOWS
settings-ui-dock-all-hint = Bring every undocked window back into this one.
settings-ui-docked = Docked in this window
settings-ui-module-aux = AUX SP (second spectrum)
settings-ui-module-band-menu = Band & mode selector
settings-ui-module-controls = Controls
settings-ui-module-panadapter = Panadapter
settings-ui-undock-all-hint = Put every module in a window of its own — the main window is left as the spectrum and the decoders. A module with nothing to show, like the operating panel in a voice mode, opens no window.
settings-ui-undock-hint =
    Undocked, this module moves into its own window and the main window gives the space back to what is left — put the waterfall on a second monitor, or the controls on a third. Closing its window docks it again.
    
    On Wayland the size comes back but the compositor decides where a window goes: float it and pin it to a monitor with a window rule matching app-id "{ $app_id }".
settings-ui-undocked = Undocked — its own window
settings-ui-window-label = { $module } window
frame-window-chip-hint = WINDOW — move this window into its own OS window, to put it on another monitor
frame-dock-panel-pane-hint = DOCK — return this panel to the pane below the waterfall
frame-panel-window-title = { $mode } — panel
frame-roll-up-hint = Roll up — leave the title bar and give the window back to the waterfall
frame-unroll-hint = Unroll — show the panel again
frame-window-panel-hint = WINDOW — give this panel a window of its own, to put on another monitor
topbar-dock-all = DOCK ALL
topbar-dock-all-hint = Bring every module window back into this window
topbar-module-windows-title = Module windows
topbar-undock-all = UNDOCK ALL
topbar-undock-all-hint = Put every module in a window of its own — this window is left as the spectrum and the decoders
topbar-windows-chip-hint =
    ▣  Left: bring every module window home
    Right: the module windows list
topbar-wname-aux = SP2 (AUX) — a second spectrum
topbar-wname-band-menu = Band & mode
topbar-wname-controls = RX control
topbar-wname-panadapter = SP1 — the spectrum
topbar-wname-panel = Operating panel
topbar-wstate-float = Lift { $name } out of the pane and float it over this window
topbar-wstate-floating-panel-only = Floating is the operating panel's own state — a spectrum or a console has one place in this window
topbar-wstate-here-now = { $name } is here now
topbar-wstate-no-panel = No operating panel in { $mode } — this applies in a mode that has one
topbar-wstate-own-window = Give { $name } a window of its own, on any monitor
topbar-wstate-put-back = Put { $name } back in this window
frame-workspace-missing = No workspace { $n } saved — save one in Settings › UI › Workspaces and this key will call it back.
settings-ui-workspaces = Workspaces
settings-ui-workspaces-note = A named window arrangement — which modules are in their own windows and where those windows are. Save the one you have, or put a saved one back. Up to ten.
settings-ui-workspaces-none = No workspaces saved yet.
settings-ui-workspace-restore-home = Put this arrangement back — every module in the main window
settings-ui-workspace-restore-out = Put this arrangement back — { $n } module(s) in their own window(s)
settings-ui-workspace-forget = Forget this workspace
settings-ui-workspace-name = name this arrangement
settings-ui-workspace-full = Ten workspaces is the limit SDRuno keeps; delete one first
settings-ui-workspace-save = SAVE CURRENT
settings-ui-workspace-save-hint = Save the arrangement you have now under this name
settings-ui-workspace-name-first = Type a name first
topbar-band-window2-hint = WINDOW — open the selector in a window of its own, as wide as you like, on any monitor
tnc-starting = Starting the modem…
tnc-link = LINK
tnc-connected = CONNECTED
tnc-calling = CALLING
tnc-incoming = INCOMING
tnc-idle = LISTENING
tnc-unavailable = NOT RUNNING
tnc-problem = Problem
tnc-tx = TX
tnc-busy = BUSY
tnc-busy-hint = Someone else is on the channel. A call now would land on top of them.
tnc-setup = ⚙ SETUP
tnc-setup-hint = Station callsign, bandwidth, the lead-in before each burst, the TCP host ports
tnc-peer = Peer
tnc-width = Width
tnc-speed = Speed
tnc-snr = S/N
tnc-bytes = Bytes
tnc-queued = Queued
tnc-host = Host
tnc-ports = Ports
tnc-ports-off = off
tnc-nobody = nobody
tnc-holder-panel = this panel
tnc-holder-winlink = the MAIL window
tnc-holder-tcp = a program at { $addr }
tnc-tune = TUNE
tnc-tune-hint = A steady tone for an antenna tuner (Mercury). Not while a link is up; press again to stop.
tnc-abort = ABORT
tnc-abort-hint = Drop the link now, whoever is driving it. The far end will time out rather than be told.
tnc-terminal = TERMINAL
tnc-callsign = callsign
tnc-connect = CONNECT
tnc-connect-hint = Call this station
tnc-disconnect = DISCONNECT
tnc-disconnect-hint = Hang up properly: what is queued goes first, then the far end agrees.
tnc-held-by = The modem is in use by { $holder }.
tnc-no-callsign = Set a callsign first (⚙ SETUP)
tnc-clear = CLEAR
tnc-clear-hint = Empty the transcript. The link is untouched.
tnc-nothing-yet = Nothing yet. Type a callsign above and CONNECT, or turn on Listen in ⚙ SETUP to take calls.
tnc-type-here = type here, Enter sends
tnc-not-connected = not connected
tnc-send = SEND
setup-tnc-call-hint = The callsign the modem answers to, with an optional SSID. Empty uses your callsign above. A program driving the modem (Pat, VarAC, BPQ) sets its own.
setup-tnc-bandwidth = Bandwidth
setup-tnc-lead = PTT lead
setup-tnc-lead-auto = Auto
setup-tnc-lead-hint = Silence the modem puts ahead of each burst while the transmitter settles. Auto picks 50 ms for an IQ SDR and 150 ms for a sound-card rig. Raise it if the start of each burst is cut off. Mercury reads it once, when it first starts.
setup-tnc-listen = Answer calls
setup-tnc-listen-check = Listen from the panel
setup-tnc-listen-hint = Take incoming calls in the panel's terminal. Ignored while the TCP host ports are on, since a program attached there wants the calls.
setup-tnc-host = TCP host
setup-tnc-host-hint = Offer the modem on a TNC command port and the one after it (8300/8301 by default, localhost only), so Pat, Winlink Express, BPQ or VarAC can use this radio as their modem.
setup-mercury-port = Mercury's own port
setup-mercury-port-hint = Mercury always opens a command and data port of its own; sdroxide keeps them on localhost and carries the TCP host's lines to them. Change this only if 18300–18302 are taken. Read once, when Mercury first starts.
settings-radio-mercury = Radio (Mercury)
settings-call-gateway-mercury = Call a gateway over the air on Mercury. Put the radio in MERCURY first.
settings-radio-data2g = Radio (Data2G)
settings-call-gateway-data2g = Call a gateway over the air on Data2G. Put the radio in DATA2G first.
settings-tnc-dial-note = The dial frequency, upper sideband, as the gateway's keeper publishes it. Zero leaves the dial where it is.

# Playing a recording back into the decoders.
playback-radio-n = { " " }· radio { $n }

playback-no-stamp = no stamp in the name — plays in the mode you are in

playback-title = PLAYBACK — recorded audio

playback-find-hint = mode, MHz, date, name…

playback-refresh-hint = Read the recordings folder again

playback-asking = Asking the station for its recordings…

playback-folder = Folder: { $dir }

playback-none = No recordings yet. Press REC → MP3 while something is on the air; the file appears here, named with its time, frequency and mode.

playback-on = ▶ ON

playback-play = ▶ PLAY

playback-iq-hint = An I/Q capture, not audio. Playback takes the audio the recorder writes; an I/Q capture is played with --file.

playback-not-while-tx = Not while transmitting

playback-play-hint = Play this recording into the decoders from the start

playback-busy-playing = Playing back — stop it before deleting it

playback-busy-recording = Still being recorded — stop REC before deleting it

playback-delete-ask = DELETE?

playback-delete-hint-armed = Press again to delete this file for good. It cannot be undone.

playback-delete-hint = Delete this recording from the folder (asks once more first)

playback-strip-label = ▶ PLAYBACK

playback-not-live = not live — the radio is standing down

playback-carry-on = Carry on playing

playback-pause = ⏸ PAUSE

playback-pause-hint = Pause — the decoders hear silence until you carry on

playback-again = ↺ AGAIN

playback-again-hint = Back to the start — another try at a picture or a decode

playback-loop = LOOP

playback-loop-hint = Start again from the top when the file ends

playback-stop = ■ STOP

playback-stop-hint = End the playback and put the radio back where it was

playback-files = FILES

playback-end-of-file = end of file

playback-paused = paused

playback-recorded-on-mode = Recorded on { $mhz } MHz in { $mode } — the dial and mode were set from the file's name.

playback-recorded-on = Recorded on { $mhz } MHz — the dial and mode were set from the file's name.

playback-no-dial-mode = The name records no dial or mode, so it plays in the mode you are in — pick the decoder from the mode menu.

playback-format-stereo = { $rate } Hz, stereo — the left (receive) channel is played.

playback-format-mono = { $rate } Hz, mono — the one channel is played.

playback-stopped = Stopped: { $fault }

playback-intro = Play a recording back into the decoders — SSTV, CW, RTTY, PSK, FT8 and every other mode that decodes from audio. The radio stands down while it plays, the dial and mode go to what the recording was made on, and STOP puts the radio back. Change the mode while it plays to try another decoder.

topbar-stereo-yields-hint = WFM stereo is off while noise reduction or the auto-notch runs — they would smear the two channels. Switch them off to hear stereo

topbar-stereo-playback-hint = Stereo: the recording is playing back in both ears, as recorded. Click to force mono

topbar-stereo-mono-recording-hint = This recording is mono — there is no second channel to play

topbar-caption-play-back = Play back

topbar-recordings-chip = ▶ RECORDINGS…

topbar-recordings-hint = Play a recording back into the decoders — a missed SSTV picture, a weak RTTY or CW signal, an FT8 slot — with the waterfall and the decode log, as if it were on the air. The radio stands down while it plays.

playback-find = Find

playback-refresh = REFRESH

## Weather satellites (NOAA APT, Meteor LRPT)

wxsat-title = WEATHER SATELLITES

sat-wxsat-hint = WEATHER SATELLITES — the picture a NOAA (APT) or Meteor (LRPT) satellite sends on 137 MHz, drawn as it comes in, in a window you can detach

sat-wxsat = WEATHER SATELLITES

wxsat-decode = Decode

wxsat-decode-hint = Run the decoder on the chosen channel. It works from the sample window and leaves the radio's mode alone, so the channel has to be inside the window.

wxsat-channel = Satellite

wxsat-tune = Tune here

wxsat-tune-hint = Switch the decoder on and bring the dial to this channel, so that the signal is inside the sample window.

wxsat-clear = Clear

wxsat-clear-hint = Start a blank picture. What has been received so far is gone unless it was saved.

wxsat-save = Save PNG

wxsat-save-hint = Save the picture as it is drawn (turned over if Flip is on) as a grey PNG; an LRPT picture saves the channel shown.

wxsat-flip = Flip

wxsat-flip-hint = Turn the picture over: a satellite flying south is drawn upside down.

wxsat-state-off = Decoder off

wxsat-level = Level { $db } dBFS

wxsat-locked = Line sync locked

wxsat-searching = Looking for the line sync

wxsat-lines = { $n } lines

wxsat-outside-window = The channel is outside the sample window, or too close to its edge for the whole signal. Use Tune here, or a wider window.

wxsat-waiting = No picture yet. Switch the decoder on when a satellite is above the horizon; a pass lasts about ten minutes.

wxsat-signal = Signal

wxsat-kind-apt = APT

wxsat-kind-lrpt = LRPT

wxsat-image-channel = Channel { $n }

wxsat-rows = { $n } rows

wxsat-frame-locked = Frame sync locked

wxsat-frames = { $n } good frames

wxsat-waiting-lrpt = No picture yet. Meteor LRPT is digital: the picture starts a few seconds after the satellite is heard, row by row, and each channel is a separate picture.

wxsat-searching-frame = Looking for the frame sync

sat-picture = PICTURE

sat-picture-hint = Open the WEATHER SATELLITES window on this link: the decoder switches to its signal and channel, and the dial moves there.
sondemap-no-sonde-yet = No radiosonde heard yet
sonde-decoding = DECODING
sonde-decoding-off = DECODER OFF
sonde-count = Sondes: { $n }
sonde-frames = Frames: { $n }
sonde-channel = Channel
sonde-channel-hover = The sonde frequency to listen on (400–406 MHz). The radio is tuned there too, so the channel is inside the window.
sonde-col-serial = Sonde
sonde-col-alt = Altitude
sonde-col-climb = Climb
sonde-col-speed = Speed
sonde-col-battery = Battery
sonde-col-heard = Heard
sonde-scan = Scan
sonde-scan-hover = Look across the whole sample window for sondes and decode up to four at once, instead of listening on one channel. The window must cover them: the channel box centres the radio.
sonde-online = Online (SondeHub)
sonde-online-hover = Also show sondes other stations have heard, from SondeHub's public list. Only reads: your locator, rounded to 0.1°, and a 400 km radius are sent; nothing is uploaded. Off until you switch it on.
sonde-online-count = { $n } online
sonde-online-no-grid = Set your locator in the digital-mode setup to see online sondes.
sonde-online-failed = SondeHub did not answer.
sonde-export = Export flight
sonde-export-kml-hover = Save this sonde's flight as a KML track (Google Earth): time, longitude, latitude, altitude in metres
sonde-export-csv-hover = Save this sonde's flight as a CSV table for a spreadsheet: time, latitude, longitude, altitude in metres
sonde-export-none = No position kept yet for this sonde.
sonde-col-temp = Temp.
sonde-col-humidity = Humidity
sonde-col-pressure = Pressure
sonde-online-why-signin = The station wants you signed in first.
sonde-online-why-busy = The station is already asking SondeHub as often as it may. Try again in a minute.
sonde-online-why-upstream = The station could not reach SondeHub.
sonde-online-why-unreadable = The answer was not a list of sondes.
sonde-online-why-no-station = No station to ask, and this page cannot reach SondeHub itself.
sonde-online-why-http = Answer: HTTP { $n }.
sonde-card-source = Source
sonde-card-source-radio = This radio
sonde-card-source-hub = SondeHub (heard by another station)
sonde-card-model = Model
sonde-card-maker = Maker
sonde-card-variant = Variant
sonde-card-position = Position
sonde-card-from-you = From you
sonde-card-heading = Heading
sonde-card-satellites = Satellites
sonde-card-snr = Signal-to-noise
sonde-card-station = Heard by
sonde-card-station-position = Station position
sonde-card-antenna = Antenna
sonde-card-software = Software
sonde-card-frames = Frames heard
sonde-card-open = Open on { $site }
sonde-card-close = Close
sonde-card-source-both = Radio + SondeHub (both hear it)
sonde-card-stale = Status
sonde-card-stale-value = Gone from the list: last known position
sonde-card-detach = Detach
sonde-card-attach = Attach
sonde-card-listen-hover = Click to tune the radio to this channel: the dial goes there and the decoder listens on it alone.
sonde-card-decoding = Decoded by this radio
sonde-card-waiting = Listening on this channel, nothing decoded yet
sonde-card-click-to-listen = Click the frequency to listen
sonde-type-all = All types
sonde-type-hover = Look for one type of sonde: choosing one starts the scan, which moves the window across the band until that type is heard. Only that type is listed and drawn.
sonde-export-saved = Saved { $name }.
sonde-title = RADIOSONDES
sonde-tune = TUNE
sonde-tune-hover = Switch the decoder on and bring the radio to the sonde channel, so the channel is inside the sample window. A mode that parks the receiver elsewhere (ADS-B, AIS, HFDL…) is left for NFM.
sonde-needs-iq = The radiosonde decoder needs a wideband I/Q source; a rig that only hands over audio cannot feed it.
sonde-off-hint = The radiosonde decoder is off. Press DECODER OFF to start it, or TUNE to start it and go to the channel.
sonde-outside-window = The sample window does not reach the sonde band (400–406 MHz): press TUNE to bring the radio there.
ism-sonde-chip = RADIOSONDES
ism-sonde-chip-hover = Weather-balloon radiosondes on 400–406 MHz (RS41, DFM, M10/M20, iMet and more): a map of every sonde heard with its flight, in a window of its own

## Keyboard modes: MT63 (src/app/panels/text_modem.rs)
textmodem-mt63-long-hint = MT63-{ $label }, { $bw } Hz wide — the long interleave spreads a burst over { $chars } characters
textmodem-mt63-short-hint = MT63-{ $label }, { $bw } Hz wide, short interleave — the short interleave spreads a burst over { $chars } characters
textmodem-mt63-not-offered = 1K and 500L not offered yet
textmodem-mt63-not-offered-hint = MT63-1K and the 500 Hz long interleave read with the batch decoder but not yet through the live receiver: the lock searches the symbol phase and the polarity but not the band centre, and a search that does was found to trade one variant for another.

## Keyboard modes: MFSK (src/app/panels/text_modem.rs)
textmodem-mfsk-hint = { $label } — { $baud } baud, { $tones } tones, { $bw } Hz wide{ $caveat }
textmodem-mfsk-reverse-hint = Invert the tone order. A station received on the opposite sideband to the one it was sent on arrives mirrored about its centre and reads as noise until this is on.

## Keyboard modes: DominoEX (src/app/panels/text_modem.rs)
textmodem-domino-hint = { $name } — { $baud } baud, 18 tones, incremental frequency keying with no forward error correction
textmodem-ifk-reverse-hint = Invert the tone steps. IFK+ carries its data in the step between successive tones, so a station received on the opposite sideband has every step negative and reads as noise until this is on.

## Keyboard modes: PSK families (src/app/panels/text_modem.rs)
textmodem-psk-family = Family
textmodem-psk-family-plain-hint = PSK31's own family: one differential carrier, no forward error correction
textmodem-psk-family-bpsk63f-hint = The robust code at 62.5 baud, with no interleaver — the one coded mode fldigi sends without one
textmodem-psk-family-robust-hint = One carrier with the robust code and a 2x2xN interleaver
textmodem-psk-family-multi-hint = Several carriers, PSK31's varicode and **no** code — wider and faster, and it needs a clean signal
textmodem-psk-family-robust-multi-hint = Several carriers **and** the robust code: the widest and the most robust of the PSK modes
textmodem-psk-baud-fast-hint = { $baud } baud — reads from about a second in: at this rate the receiver's lock time is a third of a message
textmodem-psk-baud-hint = { $baud } baud
textmodem-psk-bpsk-hint = One bit per symbol, differentially detected
textmodem-psk-8psk-hint = Three bits per symbol, no code: the fastest at any given speed and the least robust. Reads its own transmission back; fldigi's 8PSK recordings do not read yet, so it is unproven against other programs
textmodem-psk-coded = coded
textmodem-psk-no-code = no code
textmodem-psk-coded-hint =
    { $baud } baud, { $n } { $n ->
            [one] carrier
           *[other] carriers
        }, { $code }, { $bw } Hz wide
textmodem-psk-rx-only = receive only
textmodem-psk-rx-only-hint = This bank receives — fldigi's recording of it reads — but our own transmission of it does not read back through the same receiver yet, so transmit is refused rather than putting an unproven signal on the air.
