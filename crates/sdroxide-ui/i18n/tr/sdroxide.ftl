# sdroxide — arayüz metni, İngilizce; her çevirinin karşılaştırıldığı başvuru.
# Kurallar için crates/sdroxide-ui/src/i18n.rs dosyasına bakın.
#
# Kimlikler gösterildikleri yere göre gruplanmış ve oraya göre adlandırılmıştır:
# settings-ui-* Ayarlar penceresinin UI sekmesidir, vb. Büyük harflerle
# gösterilen bir ileti burada da büyük harflerle yazılır; kod çevrilmiş metnin
# harf durumunu değiştirmez.

## Dil seçicisinin listelediği, dilin kendi adı.
language-name = Türkçe

## sdroxide-types değerleri, ekranın adlandırdığı biçimde (src/i18n/labels.rs).

layout-mode-auto = Otomatik
layout-mode-desktop = Masaüstü
layout-mode-tablet = Tablet
layout-mode-small = Küçük ekran
layout-mode-phone = Telefon

ui-theme-default = Varsayılan
ui-theme-light = Açık
ui-theme-high-contrast = Yüksek karşıtlık
ui-theme-green-phosphor = Yeşil fosfor
ui-theme-amber-phosphor = Kehribar fosfor
ui-theme-teal-orange = Deniz mavisi / turuncu
ui-theme-rainbow = Gökkuşağı
ui-theme-nord = Nord
ui-theme-nord-dark = Nord koyu
ui-theme-gruvbox = Gruvbox
ui-theme-everforest = Everforest
ui-theme-solarized-dark = Solarized koyu
ui-theme-solarized = Solarized
ui-theme-dracula = Dracula
ui-theme-catppuccin-mocha = Catppuccin mocha
ui-theme-catppuccin-latte = Catppuccin latte
ui-theme-modern-minimalist = Modern minimalist

chrome-style-angled = Köşeli
chrome-style-rectangular = Dikdörtgen
chrome-style-rounded = Yuvarlak
chrome-style-gradient = Geçişli
chrome-style-bevel = 3B kabartma
chrome-style-terminal = Terminal

font-size-small = Küçük
font-size-medium = Orta
font-size-large = Büyük

bandplan-kind-ham = Amatör
bandplan-kind-cw = CW
bandplan-kind-digi = Dijital
bandplan-kind-phone = Sesli
bandplan-kind-beacon = Işaret
bandplan-kind-broadcast = Yayın
bandplan-kind-am = AM / UD / OD
bandplan-kind-cb = CB

speech-verbosity-terse = Kısa
speech-verbosity-normal = Normal
speech-verbosity-full = Tam

# Köşeli parantezler içindeki örnekler, (İngilizce) sesin söylediklerini gösterir.
speech-freq-style-dial = Skala (14 nokta 074)
speech-freq-style-digits = Her rakam
speech-freq-style-full = Birimli skala

speech-callsign-style-phonetic = Fonetik (kilo bir alfa)
speech-callsign-style-letters = Harfler (kay bir ay)
speech-callsign-style-as-is = Yazıldığı gibi

## Ayarlar penceresi, UI sekmesi (src/app/settings/ui_tab.rs).

settings-ui-display = Ekran
settings-ui-moved-note = Panadaptör ayrıntısı, spektrumun tepkisi ve şelalenin kayma hızı, ekran şeridindeki SPEC düğmesinden ayarlanır — dar pencerede DISP menüsünden.
settings-ui-language = Dil
    .hint = Arayüzün dili. F1 ile açılan kılavuz ve sesli duyurular, burada hangisi seçilmiş olursa olsun İngilizcedir.
settings-ui-language-system = Sistem ({ $language })
settings-ui-layout = Düzen
settings-ui-theme = Tema
settings-ui-button-style = Düğme stili
settings-ui-window-style = Pencere stili
settings-ui-frame-rate = Ekran yenileme hızı
settings-ui-palette = Şelale paleti
settings-ui-tuning-buttons = Ayar düğmeleri
settings-ui-tuning-buttons-show = Telefon ve tablette göster
    .hint = Dokunmatik ekranda kontrol şeridinin altında bir sıra adım-azalt / adım / adım-artır düğmesi. Bir rakamı kaydıracak tekerlek ve skala yoktur; bunlar olmadan bilinen bir adımı taşımanın tek yolu bütün frekansı yazmaktır. Masaüstünde hiç çizilmez.
settings-ui-tuning-step = Adım: { $step }
settings-ui-tuning-snap = İlk basışta adıma oturt
    .hint = Adımın iki katı arasında kalan bir skala değeri, basılan yöndeki bir sonraki kata gider — 1 kHz adımda 7 074 300, yukarı basılınca 7 075 000'e, aşağı basılınca 7 074 000'e. Bundan sonra düğmeler her zamanki gibi adım adım hareket eder. Kapalıyken her basış tam olarak bir adım hareket ettirir.
settings-ui-smoothing = Şelale yumuşatma
settings-ui-smoothing-interpolate = Ara değer hesapla
    .hint = Her ekran pikselini çevresindeki bin'ler ve satırlarla karıştırır; böylece ekran dönüşümden geniş olduğunda bir sinyal sürekli görünür. Dikdörtgen bir şelale için kapatın — bin başına bir blok, satır başına bir — çünkü bir sinyalin şeklini resimden okumak buna bağlıdır: ara değerli bir sinyal, gerçekten daha geniş olanından ayırt edilemez. Daha büyük bir FFT (FFT çipi) bunun diğer yarısıdır.
settings-ui-clock = Saat
settings-ui-clock-show = Panadaptörde UTC + yerel saat
    .hint = Panadaptörün sağ üst köşesinde, UTC ve kendi saatinizde bir duvar saati.
settings-ui-background = Spektrum arka planı
settings-ui-background-gradient = Geçiş
settings-ui-background-top = üst
settings-ui-background-bottom = alt
settings-ui-spot-colours = Spot etiketi renkleri
    .hint = Her spot kaynağının taşıdığı ton: şelalenin altındaki kutular, SPOTS listesindeki rozetler ve dünya haritasındaki noktalar.
settings-ui-spot-colour = { $kind } spotları için renk
settings-ui-spot-colours-reset = Her spot rengini varsayılanına döndür
settings-ui-reset = Sıfırla
settings-ui-bandplan-colours = Bant planı renkleri
    .hint = Şelalenin altındaki bant planı şeridinde her tahsis sınıfının boyandığı ton. Bloklar şelalenin üzerinde yarı saydam çizildiği için orada buradaki örnekten daha koyu görünürler.
settings-ui-bandplan-colour = { $kind } tahsisleri için renk
settings-ui-bandplan-colours-reset = Her bant planı rengini varsayılanına döndür
settings-ui-skimmer-font = Skimmer yazı boyutu
settings-ui-waterfall-font = Şelale / spektrum yazı boyutu
settings-ui-interface-font = Arayüz yazı boyutu
    .hint = Bütün arayüzü ölçekler — menüler, iletişim kutuları, pencereler, radyo sekmeleri, üst çubuk ve düğmeleri. Yukarıdaki iki boyut bunun göreline göredir.
settings-ui-cities = Haritalardaki şehirler
    .hint =
        Dünyanın şehirlerini — her yer için bir nokta, yer varsa yanında adıyla — düz haritalarda çizer: FT8/WSPR, APRS, ADS-B ve AIS.

        Dünyanın tamamının haritasında bir noktanın *nerede* olduğunu en çok bunlar söyler; aynı zamanda haritadaki en yoğun şeydir de. Adlar okuduğunuz istasyonların önüne geçiyorsa kapatın. 3B küre etkilenmez — buradaki şehirler işaretçi değil gece tarafındaki ışıklardır ve hiçbir şey bir temasın üstüne yazılmaz.
settings-ui-cities-show = şehirleri ve adlarını göster
settings-ui-update-check = Başlangıçta güncellemeleri denetle
    .hint = Her başlatmada bir kez sdroxide.com'a daha yeni bir sürümün yayımlanıp yayımlanmadığını sorar ve bunu şelalenin üzerindeki kapatılabilir bir afişte bildirir. İsteğin kendisinden başka hiçbir şey gönderilmez.
settings-ui-3d-view = 3B görünüm
settings-ui-clouds = Bulut çizimi
settings-ui-clouds-layered = Katmanlı
settings-ui-clouds-volumetric = Hacimsel
settings-ui-clouds-note = 3B görünümdeki BULUTLAR katmanının havayı nasıl çizdiği. Katmanlı, troposferden dilimler üst üste koyar ve ucuz seçenektir. Hacimsel bunun yerine bir ışını içinden geçirir; böylece Güneş bulut tepelerini altındaki zemine düşürür ve fırtına, dışını yalnızca aydınlatmak yerine onu oluşturan yerden şimşek ışıması çıkar — piksel başına birkaç kat maliyetle.

## Ayarlar penceresi, UI sekmesi: sesli duyurular.

speech-title = Sesli duyurular
speech-enabled = Radyodaki değişiklikleri seslendir
    .hint = Neyin değiştiğini okur; böylece radyo görmeden kullanılabilir
speech-voice = Ses
speech-voice-shipped = Yüklü ses
speech-speed = Hız
    .hint = Ses kendi cümle yapısını uzatır ya da kısaltır, böylece perde değişmez. Yaklaşık 2×'in üzerinde artık kısalmaz.
speech-volume = Ses düzeyi
speech-output = Çıkış
speech-detail = Ayrıntı
speech-duck = Alıcıyı kıs
speech-duck-while = Konuşurken
speech-test = Dene
speech-what = Ne duyurulsun
speech-cat-frequency = Frekans
speech-cat-mode-band = Mod ve bant
speech-cat-vfo-split = VFO ve split
speech-cat-agc-gain = AGC ve kazanç
speech-cat-levels = Sürüş, ayar ve mikrofon
speech-cat-ptt = Yayın ve alma
speech-cat-rit-xit = RIT ve XIT
speech-cat-memory-scan = Hafızalar ve tarama
speech-cat-band-edge = Bir amatör bantdan çıkma
speech-cat-notices = Uyarılar ve iletiler
speech-cat-filters = Filtreler, susturma ve gürültü azaltma
    .hint = Varsayılan olarak kapalı: bir sinyali kovalarken bunlar sürekli değişir
speech-decoded = Çözülen iletiler
speech-ft8-to-me = Bana gelen FT8 çağrıları
speech-ft8-cq = Yanıtlayabileceğim FT8 CQ'ları
    .hint = Yirmi metrede yoğun bir akşam, dakikada yüz tane demektir
speech-js8 = Bana gelen JS8 iletileri
speech-js8-allcall = JS8 @ALLCALL de
speech-fsq = Bana gelen FSQ iletileri
speech-include-snr = Raporu ekle
    .hint = Yalnızca ileti kendi raporunu taşımıyorsa — kendi sayısını zaten bildiren bir çözümlemeye bizimki de eklenmez
speech-ft8-qso = Kendi FT8 alışverişim
    .hint = Sıralayıcının göndermek üzere olduğu şey ve temasın bitişi
speech-reading-text = Çözülen metni yüksek sesle okuma
speech-text-cw = CW
speech-text-rtty-psk = RTTY, PSK, Olivia, THOR, FSQ
speech-text-cw-locked = Yalnızca çözücü kilitliyken CW
    .hint = Kilitli olmayan bir çözücünün çıktısını okumak sessizlikten kötüdür
speech-text-note = İkisi de varsayılan olarak kapalıdır. Bir çözücü metni konuşmanın okuyabileceğinden hızlı üretir, bu yüzden canlı sesin çok gerisinde kalan her şey sıraya alınmak yerine atılır.
speech-tuning-up = Ayarlama
speech-swr-while-tuning = TUNE basılıyken SWR'yi oku
speech-swr-every = Her
speech-swr-summary = Bırakıldığında en iyi uyumu bildir
speech-swr-alarm = Her yayında yüksek SWR'ye karşı uyar
speech-how-read = Şeyler nasıl okunur
speech-frequencies = Frekanslar
speech-callsigns = Çağrı işaretleri
speech-quiet-on-tx = Yayın yaparken sessiz kal
    .hint = Konuşma hoparlörlerinize gider ve dolayısıyla mikrofonunuza girer. Yüksek SWR uyarıları yine de iletilir.
speech-device-note = Duyurular kendi ses aygıtlarında çalar; bu yüzden hiçbir zaman kaydedilmez ve uzaktan dinleyen kimseye gönderilmez. Durumu seslendirme, son duyuruyu tekrarlama ve bir cümlenin ortasında durdurma tuşları Controls sekmesindedir.

## Ayarlar penceresi (src/app/settings/mod.rs)

settings-waiting-station-s-network-configuration = İstasyonun ağ yapılandırması bekleniyor…
settings-device-reports = aygıtın bildirdiği gibi
settings-transverters = Transverterler
settings-one-row-per-box-while = Her kutu için bir satır. Skala bir satırın bandı içindeyken radyo skala + kayma değerine ayarlanır ve o satırın yayın kuralı ile sürüş sınırı uygulanır; her satırın dışında radyo kendi bantlarındadır. Satırlar yukarıdan aşağıya denenir.
settings-name = Ad
settings-band-low = Bant alt
settings-band-high = Bant üst
settings-offset = Kayma
settings-transmit = Yayın
settings-max-drive = En yüksek sürüş
settings-off-takes-transverter-out-line = Kapalı, bu transverteri satırı kaybetmeden hattan çıkarır — bant, radyonun kendi başına eriştiği yere döner.
settings-t-2-m = 2 m
settings-what-you-call-shown-log = Buna ne diyorsunuz. Skala onu seçtiğinde günlükte gösterilir.
settings-bottom-band-transverter-works-dial = Bu transverterin çalıştığı bandın alt ucu, skalada. 2 m için 144.
settings-top-band-dial-148-2 = Bandın üst ucu, skalada. 2. ve 3. Bölgede 2 m için 148, 1. Bölgede 146.
settings-radio-ends-up-dial-offset = Radyo skala + kayma değerine gelir; bu yüzden bir bandı bir A.F.'ye (ara frekans) indiren transverter negatiftir: 28 MHz A.F.'ye giren 2 m için -116, çünkü 144 - 116 = 28. Biraz kayık bir osilatör için her seferinde bir hertz ayarlamak üzere sürükleyin.
settings-off-while-converting-receive-only =
    Dönüştürürken kapalı: yalnızca alma — satır bir transverter değil bir dönüştürücüdür ve bu bantta hiçbir şey anahtarlanmaz.

    Aynı dönüştürücüden: iki yönde de çalışır; yayın aynı kaymayı alır.
settings-ceiling-transmit-drive-while-transverter =
    Bu transverter seçiliyken yayın sürüşüne, tamamın yüzdesi olarak bir tavan. Bir transverterin A.F. girişi miliwatt alır ve radyonun kendi bantları için doğru olan sürüş onu yok eder; bu yüzden bu satırın en önemli alanıdır. %100 sınır yoktur.

    Sürüş ayarınız bunun altında tutulur, değiştirilmez; böylece HF'de kullandığınız sayı skala banttan çıktığında geri gelir.
settings-remove = KALDIR
settings-add-transverter = TRANSVERTER EKLE
settings-takes-effect-apply-reconnect-like = Uygula / yeniden bağlan ile etkinleşir; yukarıdaki dönüştürücü kaymasında olduğu gibi.
settings-maximum-transmit-drive = En yüksek yayın sürüşü
settings-hard-limit-drive-tune-controls = Sürüş ve TUNE denetimlerinin geçemeyeceği sabit bir sınır; bant kalibrasyonundan sonra uygulanır, böylece hiçbir şey sürüşü ona geri çıkaramaz. Radyonun nominal gücünü verdiği yere koyun — tüm Sürüş denetimi o zaman kullanılabilir olur. Kendi örneklerini modüle eden bir vericide (HPSDR, LimeSDR, PlutoSDR, HackRF) tam sürüş, vericinin tam açık olması demektir ve yüksek kazançlı bir amplifikatörde final katlarının derecesinin çok ötesindedir. Kapalı, denetimlerin tam sürüşe ulaşması demektir.
settings-limit-drive = Sürüşü şu değerle sınırla
settings-per-cent-full-drive-key = Tam sürüşün yüzdesi. Radyoyu bir yapay yüke anahtarlayın ve ölçer amplifikatörün nominal gücünü gösterene dek bunu düşürün.
settings-applies-immediately-every-band-tune = Hemen uygulanır; her bantta ve TUNE için de.
settings-transmit-drive-band = Banda göre yayın sürüşü
settings-every-amplifier-has-different-gain = Her amplifikatörün her bantta farklı bir kazancı vardır, bu yüzden tek bir Sürüş ayarı her birinde farklı bir güç üretir. Her bantta çıkışı ölçün, sonra hepsi eşleşene dek en yüksek çıkan bantları kısın: Sürüşü en çok gereken bant için ayarlayın (genellikle en yüksek olan) ve diğerlerini ona indirin. Ses, dijital ve TUNE'a birlikte uygulanan çıkış gücünde desibel. Her yerde sıfır — varsayılan — hiçbir şeyi değiştirmez.
settings-other = Diğer
settings-clear-all = HEPSİNİ TEMİZLE
settings-back-no-calibration-drive-setting = Kalibrasyon yok durumuna dön: Sürüş ayarı her bandı tam olarak etkiler.
settings-applies-immediately-band-you-would = Hemen uygulanır ve yayın yapacağınız banda — bir transverterin arkasında bu, skaladaki banttır; transverterin kendi sürüş sınırı da buradaki her şeyi geçersiz kılar.
settings-settings = Ayarlar
settings-enter-server-s-address-first = önce sunucunun adresini girin
settings-enter-receiver-s-address-first = önce alıcının adresini girin ya da Public SDRs'den birini seçin
settings-no-radio-selected-press-discover = radyo seçilmedi — Discover'a basın ya da bir adres girin
settings-enter-radio-s-address-first = önce radyonun adresini girin
settings-general = Genel
settings-radio = Radyo
settings-ui = UI
settings-alerts = Uyarılar
settings-controls = Denetimler
settings-spots = Spotlar
settings-freedv = FreeDV
settings-uploads = Yüklemeler
settings-winlink = Winlink
settings-servers = Sunucular
settings-t-r-switch = T/R anahtarı
settings-amplifier = Amplifikatör
settings-remote = Uzak
settings-tle = TLE
settings-profiles = Profiller
settings-station = İstasyon
settings-enter-digital-mode-ft8-sstv = Kaydedilmiş değerleri yüklemek için bir dijital modu (FT8 / SSTV / …) bir kez açın.
settings-callsign = Çağrı işareti
settings-grid-square = Lokatör
settings-your-callsign-grid-shared-across = Çağrı işaretiniz ve lokatörünüz; FT8/FT4/FT2, SSTV görüntü başlıkları ve günlük arasında ortaktır. FT8 / SSTV kurulum iletişim kutusundan da düzenlenebilir.
settings-iaru-region = IARU bölgesi
settings-which-radio-interface-server-uses = Sunucunun hangi radyo arayüzünü kullandığı, çalıştığı makinede ayarlanır.
settings-radio-interface = Radyo arayüzü
settings-waiting-machine-radio-attached-interface = Radyonun bağlı olduğu makine bekleniyor. Arayüz ayarları aşağıdadır ve bu arada buradan değiştirilebilir.
settings-converter = Dönüştürücü
settings-manual-type-offset-below = Elle — aşağıya kayma yazın
settings-what-box-reads-when-offset = Kayma hazır ayarlardan biri değilken bu kutunun okuduğu değer. Aşağıdaki Kayma alanına yazarak ayarlanır; burada seçilecek bir şey yoktur.
settings-frequency-converter-front-receiver-pick = Alıcının önündeki bir frekans dönüştürücü. Birini seçin ya da yanına bir kayma yazın.
settings-how-far-converter-moves-signal =
    Bir dönüştürücünün sinyali alıcıya giderken Hz cinsinden ne kadar kaydırdığı — her dönüştürücünün belgelerinin ve diğer her SDR programının belirttiği aynı sayı ve işaret. Radyo skala + kayma değerine gelir: bir yukarı dönüştürücü için pozitif (bir Ham It Up 125000000'dır), bir bandı bir A.F.'ye indiren her şey için negatif — bir uydu LNB'si ya da 28 MHz A.F.'ye 2 m transverteri, bu da -116000000'dır (28 − 144). 0 = dönüştürücü yok.

    Biraz kayık bir osilatörü olan dönüştürücünün istediği gibi, her seferinde bir hertz ayarlamak için sürükleyin.

    Bu alma yoludur. Yayın hattındaki şey aşağıdaki satırdır.

    Uygula ile etkinleşir.
settings-what-transmit-line-while-converter =
    Bir dönüştürücü ayarlıyken yayın hattında ne olduğu.

    Dönüştürürken kapalı: hiçbir şey yayınlanmaz — güvenli varsayılan ve yalnızca alan bir yardımcı cihaz için doğru olan.

    Aynı dönüştürücüden: bir kutu iki yönde de dönüştürür, bu bir transverterdir. Yayın yukarıdaki kaymayı alır ve o ayarlandığında onu izler.

    Kendi kayması: yayın hattı alma hattından farklıdır. Sıfır yaygın durumdur — alımda bir dönüştürücü (bir LNB, bir Ham It Up) ve verici kendi anteniyle, QO-100 istasyonu gibi.

    Amatör bant denetimi, yayın yaptığınız frekansta hâlâ geçerlidir.

    Uygula ile etkinleşir.
settings-how-far-transmit-line-moves =
    Yayın hattının sinyali Hz cinsinden ne kadar taşıdığı; alma kaymasıyla aynı işaret kuralıyla: radyo skala + kayma değerinde yayın yapar.

    0 = verici kendi anteniyle bağlıdır ve skalanın gösterdiği frekansta çalışır — bir QO-100 istasyonunun 2,4 GHz yukarı bağlantısı.

    Bir A.F.'yi *yukarı* alan bir yayın dönüştürücüsü negatif bir sayıdır: radyo skalanın altında çalışır.
settings-rx-range = Alma aralığı
settings-which-frequencies-radio-receives-mhz =
    Bu radyonun aldığı frekanslar, MHz cinsinden: 144-146, 430-440. Aygıtın kendisi hakkında bildirdiği şeyi kullanmak için boş bırakın.

    Aralığın dışındaki bant düğmeleri soluk görünür ve skala oraya gitmez.

    Bunlar skala frekanslarıdır. Bir dönüştürücü ayarlıyken, radyonun gerçekte olduğu A.F. yerine ayarladığınız bandı belirtin — bir 2 m transverter için 144-148.

    Uygula ile etkinleşir.
settings-tx-range = Yayın aralığı
settings-which-frequencies-radio-transmits-mhz =
    Bu radyonun yayın yaptığı frekanslar, MHz cinsinden: 144-146, 430-440. Aygıtın bildirdiğini kullanmak için boş bırakın — hiçbir şey bildirmiyorsa sürücünün sözü alınır ve herhangi bir frekansa izin verilir.

    Yukarıdaki alma aralığında olduğu gibi skala frekansları.

    Bu, sizin koyduğunuz bir sınırdır, bir lisans değildir: config.toml dosyasında kapatmadığınız sürece amatör bantların dışında yayın yapmaya her durumda izin verilmez. Ayrıca yalnızca alan bir aygıta verici de kazandırmaz.

    Uygula ile etkinleşir.
settings-antenna = Anten
settings-where-radio-listens-station-antenna =
    Bu radyonun dinlediği yer.

    İstasyonda: anten sizinkidir, bu yüzden kendi lokatörünüz nerede olduğunu söyler. Varsayılan budur ve shack'teki her şey için doğrudur.

    Başka bir yerde: çevrimiçi bir alıcı ya da bir tepede kurduğunuz kendi düzeneğiniz. Duyduğu şey, bunun yanındaki lokatörden PSK Reporter, WSPRnet ve FreeDV Reporter'a bildirilir; asla sizinkinden değil — ADS-B uçakları da buna göre yerleştirir.

    "Public SDRs" altından bir alıcı seçmek bunu sizin için doldurur.

    Uygula ile etkinleşir.
settings-receiver-s-maidenhead-locator-jn88ec =
    Alıcının Maidenhead lokatörü — JN88ec, DO30db. Dört ya da altı karakter.

    Alıcının nerede olduğunu bilmiyorsanız boş bırakın: o zaman hiçbir şey bildirilmez, ki bu tek dürüst cevaptır. Kendi kareniz üzerinden bildirmek, başkasının dinlemesini yanlış kıtaya koyardı.
settings-audio = Ses çıkışı
settings-gain-per-band = Banda göre kazanç
settings-remember-receive-gain-per-band = Banda göre alma kazancını hatırla
settings-each-band-keeps-receive-gain = Her bant, en son üzerinde ayarladığınız alma kazancını korur ve skala geri döndüğünde onu geri alır — bir bant düğmesi, bir bant kenarından sürükleme ya da radyonun kendi düğmesi. Genel kapsama HF, VHF, UHF ve üstü olarak ayrılır. Üzerinde kazanç ayarlamadığınız bant, o an geçerli olanı korur. Kapalı: her yerde tek kazanç.
settings-s-meter-calibration = S-metre kalibrasyonu
settings-added-radio-s-s-meter =
    Bu radyonun S-metre okumasına eklenir. Bir SDR'nin metresi dBFS okuyarak başlar — dönüştürücünün tam ölçeğine göre seviye — bu yüzden çok kazançlı bir alıcı, anten bağlı değilken S9 gösterebilir. Bunu metrenin gösterdiği ile bildiğiniz bir seviye arasındaki farka ayarlayın: bir sinyal üreteci ya da yaklaşık S2–S3'te durduğuna güvendiğiniz sessiz bir bant. Hemen uygulanır.

    config.toml içindeki cal_offset_db'nin üstüne eklenir; o ise bütün istasyon için tek bir rakamdır.
settings-t-0-db = 0 dB
settings-antenna-somewhere-else-no-locator = Anten başka bir yerde ve lokatör ayarlanmamış; bu yüzden bu radyonun duyduğu hiçbir şey PSK Reporter, WSPRnet ya da FreeDV Reporter'a bildirilmez. Oradan bildirmek için alıcının lokatörünü yukarıya yazın.
settings-ranges-are-mhz-low-high = Aralıklar MHz cinsindendir, alt-üst, virgülle ayrılmış: 144-146, 430-440 — yani 144000000-146000000 Hz ve 430000000-440000000 Hz. Skala frekanslarıdır: bir dönüştürücü kayması bunları taşımaz. Yukarıdaki dönüştürücü kayması, hertz cinsinden alandır. Aygıtın kendisi hakkında bildirdiğini kullanmak için bir aralığı boş bırakın; hiçbir şey bildirmeyen bir aygıtın sözü alınır.
settings-careful-rtl-sdr-blog-v4 = Bir RTL-SDR'de dikkatli olun: Blog V4, 28,8 MHz'in altında kendi başına yukarı dönüştürür; donanımı oraya düşüren negatif bir kayma iki kez kayar.
settings-transmit-audio = Yayın sesi
settings-use = kullanımda
settings-every-stage-off = her kademe kapalı
settings-open-tx-audio = TX Audio'yu aç…
settings-microphone-processing-chain-gate-eq = Mikrofon işleme zinciri — kapı, EQ, de-esser, kompresör ve sınırlayıcı — yayın yapmadan kulakla ayarlamak için bir test kaydıyla. Yalnızca ses modları. TX kutusundaki Mic'e tıklayarak da açılır.
settings-pick-radio-interface-above-configuration = Yukarıdan bir radyo arayüzü seçin (bu yapılandırma kaldırılmış otomatik algılama modunu kullanıyordu).
settings-configuration-used-removed-auto-detect = Bu yapılandırma kaldırılmış otomatik algılama modunu kullanıyordu. Radyonun bağlı olduğu makine yanıt verdiğinde yukarıdan bir arayüz seçin.
settings-radio-has-no-interface-yet = Bu radyonun henüz bir arayüzü yok — yukarıdan birini seçin ve Uygula / yeniden bağlan'a basın.
settings-radio-has-no-interface-yet-2 = Bu radyonun henüz bir arayüzü yok. Radyonun bağlı olduğu makine yanıt verdiğinde yukarıdan bir tane seçilebilir.
settings-apply-reconnect = Uygula / yeniden bağlan
settings-switch-interface-now-no-restart = Bu arayüze şimdi geç — yeniden başlatmaya gerek yok
settings-switch-server-s-radio-interface = Sunucunun radyosunu şimdi bu arayüze geçirin — yeniden başlatmaya gerek yok. Yeni olan açılamazsa eskisi çalışmaya devam eder.
settings-switches-live-radio-without-restarting = Canlı radyoyu yeniden başlatmadan değiştirir.
settings-everything-above-server-s-own = Yukarıdaki her şey sunucunun kendi yapılandırmasıdır ve değişiklikler orada kaydedilir. Çoğu ayar, siz değiştirdikçe uygulanır; aygıt açıldığında sabitlenenler — örnekleme hızı, bir adres, arayüzün kendisi — bu düğmeyi gerektirir.
settings-dx-cluster-telnet = DX kümesi (telnet)
settings-enabled = Etkin
settings-host = Ana makine
settings-port = Bağlantı noktası
settings-login-call = Giriş çağrı işareti
settings-commands = Komutlar
settings-one-per-line-e-g = satır başına bir tane, örn. SET/FT8
settings-reverse-beacon-network = Reverse Beacon Network
settings-read-world-s-cw-rtty = Dünyanın CW/RTTY skimmer'larını okuyun ve yayılım haritasını onlarla besleyin. Harita, bu radyonun dinlemediği bantları göstermesi bunun sayesindedir. Varsayılan olarak açıktır: havaya hiçbir şey koymaz, hesap gerektirmez ve Genel sekmesindeki çağrı işaretini kullanır. RBN spotları spot listesinde görünmez — onlar davet değil, ölçümdür.
settings-t-7000-cw-rtty-feed-7001 = 7000 CW/RTTY beslemesi, 7001 FT8/FT4 beslemesi
settings-one-per-line-e-g-2 = satır başına bir tane, örn. set/filter cont=eu
settings-sent-after-login-place-narrow = Girişten sonra gönderilir. Beslemeyi daraltacağınız yer — filtre olmadan bu, Dünya'daki her skimmer demektir.
settings-rbn-paths-are-placed-country = RBN yolları lokatörlerle değil ülke merkezleriyle yerleştirilir — küçük bir ülke için doğru, büyük bir ülke için çok uzakta. Yayılım haritasında ayrı bir katmandırlar ve orada kapatılabilirler.
settings-pota-sota-psk-reporter = POTA / SOTA / PSK Reporter
settings-pota-activator-spots = POTA aktivatör spotları
settings-sota-spots = SOTA spotları
settings-psk-reporter-current-band = PSK Reporter (mevcut bant)
settings-upload-my-ft8-ft4-ft2 = FT8/FT4/FT2/JS8 çözümlemelerimi yükle
settings-report-what-station-hears-pskreporter = Bu istasyonun duyduğunu pskreporter.info'ya bildirin; böylece orada bir alıcı olarak görünür. Genel sekmesindeki çağrı işaretini ve lokatörü kullanır.
settings-antenna-2 = Anten
settings-collector = Toplayıcı
settings-t-4739-live-collector-14739-test = 4739 canlı toplayıcı, 14739 test toplayıcısı
settings-max-age-s = En fazla yaş (sn)
settings-wsprnet = WSPRnet
settings-upload-my-wspr-decodes = WSPR çözümlemelerimi yükle
settings-send-every-wspr-reception-wsprnet = Her WSPR alımını wsprnet.org'a gönderin. Varsayılan olarak açıktır: havaya hiçbir şey koymaz ve duyduğunuzu bildirmek, bir WSPR alıcısını özel bir merak olmaktan çıkarıp ağın parçası yapan şeydir. Hiçbir şey çözülmeyen bir zaman dilimi de bildirilir; ağın kapalı bir bandı, kapatılmış bir alıcıdan ayırt etme yolu budur.
settings-download-who-heard-me = Beni kimin duyduğunu indir
settings-ask-wsprnet-org-which-stations = wsprnet.org'a bu istasyonu hangi istasyonların çözdüğünü sorun. WSPR'de hiçbir türde onay yoktur; bu yüzden bu, yayın yapan bir işaretin kendi erişimi hakkında bir şey öğrenmesinin tek yoludur.
settings-ask-every = Her
settings-looking-back = geriye bakarak
settings-apply = {" "}UYGULA{" "}
settings-persist-re-connect-feeds = Beslemeleri kaydet ve (yeniden) bağla
settings-broadcast-stations = Yayın istasyonları
settings-winlink-account = Winlink hesabı
settings-password = Parola
settings-winlink-account-password-not-gateway = Winlink hesabının parolası, ağ geçidi parolası değil. Büyük-küçük harf duyarlıdır — verildiği gibi tam olarak girin.
settings-locator = Lokatör
settings-how-connect = Nasıl bağlanılır
settings-route = Rota
settings-internet = İnternet
settings-forward-cms-over-internet = CMS ile internet üzerinden ilet
settings-radio-packet = Radyo (paket)
settings-call-rms-gateway-air-radio = Havada bir RMS ağ geçidini çağırın. Radyo PACKET ya da PACKET-HF modunda olmalıdır.
settings-gateway = Ağ geçidi
settings-via = Üzerinden
settings-digipeaters-order-separated-spaces-usually = Digipeater'lar, sırayla, boşlukla ayrılmış. Genellikle boş — doğrudan duyabildiğiniz bir ağ geçidi, doğrudan çağırmanız gereken ağ geçididir.
settings-speed = Hız
settings-most-rms-gateways-answer-1200 = Çoğu RMS ağ geçidi 1200'de yanıt verir. 1200 ağ geçidini 9600'de çağırmak, kapalı bir ağ geçidiyle tamamen aynı gelir; bu yüzden tahmin etmek yerine bunu ağ geçidinin sahibinin yayımladığından ayarlayın. 9600 ayrıca iki uçta da radyonun veri bağlantı noktasını gerektirir. 300 HF'dir.
settings-frequency = Frekans
settings-clear = TEMİZLE
settings-zero-leaves-dial-alone-which = Sıfır, skalaya dokunmaz; tek bir kanala park ettiğinizde istediğiniz budur. Başka her değer, oturum başladığında radyoyu ayarlar.
settings-my-gateways = Ağ geçitlerim
settings-winlink-s-published-gateway-list = Winlink'in yayımladığı ağ geçidi listesi, sdroxide'ın sahip olmadığı bir API anahtarı gerektirir; bu yüzden kendinizinkini tutun. Tek bir konumdan erişilen iki ya da üç ağ geçidi denemeyle öğrenilir ve nadiren değişirler.
settings-use-2 = KULLAN
settings-call-one-next-connect = Bunu bir sonraki bağlantıda çağır
settings-current-dial = mevcut skala
settings-forget = UNUT
settings-forget-gateway = Bu ağ geçidini unut
settings-add-gateway = + AĞ GEÇİDİ EKLE
settings-save-gateway-above-list = Yukarıdaki ağ geçidini listeye kaydet
settings-internet-gateway = İnternet ağ geçidi
settings-cms-address = CMS adresi
settings-client-name = İstemci adı
settings-winlink-s-production-servers-only = Winlink'in üretim sunucuları yalnızca tanıdıkları istemci adlarını kabul eder ve tanımadığı birine "Unknown client types are not allowed on production servers" yanıtı verir. sdroxide, Winlink Development Team ile kaydedilene kadar bağlanmak, onların tanıyacağı bir ad gerektirir.
settings-automatic-connection = Otomatik bağlantı
settings-connect-timer = Zamanlayıcıyla bağlan
settings-every = Her
settings-persist-winlink-account = Winlink hesabını kaydet
settings-callsign-lookup = Çağrı işareti sorgusu
settings-provider = Sağlayıcı
settings-auto-fill-name-qth-grid = Spot tıklanınca ve QSO'da ad/QTH/lokatörü otomatik doldur
settings-choose-provider-enter-login = Girişini yazmak için bir sağlayıcı seçin.
settings-qrz-user = QRZ kullanıcısı
settings-qrz-pass = QRZ parolası
settings-hamqth-user = HamQTH kullanıcısı
settings-hamqth-pass = HamQTH parolası
settings-one-hamqth-account-does-both = Tek bir HamQTH hesabı ikisini de yapar — bu giriş, HamQTH yükleme sekmesinin kullandığı girişle aynıdır.
settings-upload = Yükleme
settings-auto-upload-each-new-qso = Her yeni QSO'yu otomatik yükle
settings-master-switch-which-services-qso = Ana anahtar. Bir QSO'nun hangi hizmetlere gideceği, aşağıdaki her sekmede ayarlanır.
settings-turn-auto-upload-each-new = Önce yukarıdaki "Her yeni QSO'yu otomatik yükle" seçeneğini açın — burada işaretlenen bir hizmet, o açılana kadar gönderilmez.
settings-auto-upload-off-above-so = Otomatik yükleme yukarıda kapalı, bu yüzden hizmet işaretleri devre dışı ve henüz hiçbir şey kendiliğinden gönderilmiyor — günlükteki QSO başına UP düğmesi yine çalışır.
settings-qrz-log-key = QRZ günlük anahtarı
settings-qrz-logbook-api-key-your = QRZ günlük ayarlarınızdaki QRZ günlük API anahtarı — XML sorgu girişi değil.
settings-eqsl-user = eQSL kullanıcısı
settings-eqsl-pass = eQSL parolası
settings-eqsl-qth = eQSL QTH
settings-qth-nickname-eqsl-profile-upload = Yükleme ve indirme yapılacak eQSL profilinin QTH takma adı — yalnızca hesap birden fazla profile sahipse gerekir; aksi halde boş bırakın.
settings-same-login-hamqth-callsign-lookup = HamQTH çağrı işareti sorgusuyla aynı giriş — birini doldurmak ikisini de doldurur.
settings-club-log-email = Club Log e-postası
settings-club-log-pass = Club Log parolası
settings-club-log-key = Club Log anahtarı
settings-wrl-api-key = WRL API anahtarı
settings-developer-api-key-world-radio = World Radio League → Integrations → Developer API bölümündeki geliştirici API anahtarı. Oluşturduğunuzda bir kez gösterilir, bu yüzden o zaman kopyalayın. Temaslar varsayılan günlüğünüze gider — birden fazla günlüğünüz varsa WRL'de birini ayarlayın.
settings-confirmations-download = Onaylar (indirme)
settings-lotw-user = LoTW kullanıcısı
settings-lotw-pass = LoTW parolası
settings-lotw-upload-uses-tqsl-export = LoTW yüklemesi TQSL kullanır — günlükten ADIF dışa aktarıp imzalayın. LoTW/eQSL onayları, çalışıldı ile onaylandı arasını işaretlemek için burada indirilir.
settings-sync-confirmations = ONAYLARI EŞİTLE
settings-fetching-subscriptions = Abonelikler alınıyor…
settings-spectrum-waterfall-everything-read-off =
    Spektrumu, şelalesi ve onlardan okunan her şey o radyonun sekmesindedir; bu yüzden kendi sekmesi yoktur. Yukarıdaki arayüz ayarları yine bu radyonun ve yine uygulanır — değiştirdikten sonra Uygula / yeniden bağlan'a basın, böylece iki radyo birlikte yeniden açılır.

    Yeniden tek başına kullanmak için, o radyonun sayfasındaki Panadaptör alıcı kutusunu temizleyin.
settings-panadapter = Panadaptör
settings-give-radio-wideband-panadapter-borrowing = Bu radyoya, başka bir radyonun alıcısını ödünç alarak geniş bantlı bir panadaptör verin — aynı anten üzerindeki bir SDR ya da bu radyonun A.F. çıkışından beslenen bir alıcı. Spektrum, şelale, alt alıcı, dijital modlar ve skimmer hepsi o alıcıdan gelir; skala, mod, filtre ve verici burada kalır.
settings-receiver = Alıcı
settings-none = Yok
settings-which-radio-receives-one-leaves =
    Bu radyo için hangi radyonun alım yaptığı. Ödünç verildiği sürece sekme şeridinden çıkar — ön ucu bu radyonun motoruna aittir — ve bu Yok'a ayarlandığında hemen geri gelir.

    Uygula ile etkinleşir.
settings-connected = Bağlı olduğu
settings-how-far-above-radio-s =
    Alıcının bu radyonun skalasının Hz cinsinden ne kadar üstünde durduğu — aynı anten üzerindeki bir alıcı için 0. Bir A.F. kulağı için, alıcının üzerinde durduğu ara frekans: bir K3'ün KXV3B'si için 8215000, 9 MHz A.F. için 9000000.

    Bu, radyoyu yeniden ayarlayan yukarıdaki Dönüştürücü alanı değildir; ve CAT sekmesindeki I/Q merkez kayması da değildir, o bu radyonun kendi ses kartıyla ilgilidir. Buraya yazılan hiçbir şey hiçbir radyoya gönderilmez.

    Uygula ile etkinleşir.
settings-audio-2 = Ses kaynağı
settings-follow-dial = Skalayı izle
settings-keep-receiver-radio-s-dial = Alıcıyı bu radyonun skalasında tutun ve panadaptörde ayarlarken skalayı hareket ettirin. Kapalıyken alıcı olduğu yerde durur — radyo başka yere giderken bir segmentte sabit bir izleme.
settings-invert-spectrum = Spektrumu ters çevir
settings-mirror-receiver-s-span-about = Alıcının genişliğini merkezi etrafında aynalar; osilatörü sinyalin üzerinde duran ve bandı ters yönde veren bir kulak için. Şelale, skalanın yanlış tarafında olan sinyallerle dolmadıkça kapalı bırakın.
settings-mute-transmit = Yayında sustur
settings-silence-receive-audio-while-radio = Bu radyo yayın yaparken alma sesini sustur. Varsayılan olarak açıktır: alıcı aynı anten üzerindeyken ya da bu radyonun A.F.'sindeyken, bir yayın sırasında duyduğu şey kendi vericinizdir. Doğrudan radyonun üzerinden anahtarlanan yayınlar — mikrofonun PTT'si, ayak anahtarı, MOX — de sayılır; CAT bağlantısı cihazın yayına geçtiğini bildirdiği anda.
settings-blank-transmit = Yayında boşalt
settings-stop-panadapter-waterfall-while-radio = Bu radyo yayın yaparken panadaptörü ve şelaleyi durdur. Varsayılan olarak açıktır: bütün aralığa boyanan bir verici, arkasındaki bandı siler. Bir yayın boyunca bandı — ya da kendi sinyalinizi — izlemek için kapatın; alıcı ayrı bir antendeyken buna değer. sdroxide'ın anahtarladığı yayınlar için geçerlidir: radyonun üzerinden anahtarlananlar baştan sona alınır, bu ne derse desin görüntü çalışmaya devam eder.
settings-per-mode-offsets-radio-whose = Mod başına kaymalar, A.F.'si modla birlikte kayan bir radyo için. Boş bırakılan bir mod, yukarıdaki kaymayı kullanır.
settings-computer = Bu bilgisayarda
settings-add-radio-computer = Bu bilgisayara bir radyo ekle
settings-radio-s-settings-are-below = Bu radyonun ayarları aşağıdadır. Şeritte taşımak için sürükleyin.
settings-switch-radio-dialog-follows-drag = Bu radyoya geç (iletişim kutusu izler). Şeritte taşımak için sürükleyin.
settings-tx = ● TX
settings-mute-radio-s-audio = Bu radyonun sesini sustur
settings-link = BAĞLANTI
settings-close-radio-configuration-kept = Bu radyoyu kapat (yapılandırması korunur)
settings-close-tab-close-radio-station = Bu sekmeyi kapat ya da radyoyu istasyonunda kapat
settings-close-tab = Bu sekmeyi kapat
settings-transverters-full = { $max } bu tabloda tutulabilecek en fazla sayıdır
settings-dialling = { $url } çevriliyor…
settings-region-note = İstasyonun bulunduğu yer. Her bant planını ayarlar: bant kenarları, şelale şeridindeki CW / veri / sesli alt segmentler, skimmer'ların nerede dinlediği, sunulan çağrı frekansları ve yayın için bant dışı sayılan şey. { $region }. Bölgede bu, 70 cm için { $region ->
        [1] 430–440 MHz demektir — yani 446 bant dışıdır ve 40 m 7.200'de durur
        [2] 420–450 MHz demektir ve 40 m 7.300'e kadar gider
       *[3] 430–450 MHz demektir ve 80 m 3.900'de durur
    }. Hemen etkinleşir ve bu istasyondaki her radyoya uygulanır.
settings-rx-grid-reported = Bu radyo üzerinden gelen alımlar, kendi lokatörünüzden değil { $grid } konumundan bildirilir.
settings-rx-grid-invalid = { $grid } bir Maidenhead lokatörü değil, bu yüzden bu radyonun duyduğu hiçbir şey bildirilmez. Dört ya da altı karakter: iki harf, iki rakam ve isteğe bağlı olarak iki harf daha.
settings-bypassed = atlandı
# $state, settings-bypassed, settings-use ya da settings-every-stage-off değerlerinden biridir.
settings-tx-audio-chain = Zincir { $state }.
settings-gateway-via = { " " }üzerinden { $via }
settings-auto-upload-to = Her yeni QSO'yu { $service } hizmetine otomatik yükle
settings-tle-updated = Güncellendi: { $n } { $n ->
        [one] abonelik
       *[other] abonelik
    }: { $total } uydu.
settings-tle-updated-failed = { $n } içinden { $ok } güncellendi; { $failed } başarısız — yukarıdaki satırlara bakın.
settings-panadapter-of = Bu radyo, { $radio } radyosunun panadaptörüdür
settings-radio-n = radyo { $n }
settings-on-station = { $station } üzerinde
settings-add-radio-on-station = { $station } üzerine bir radyo ekle — istasyon onu hemen sunar
settings-panadapter-receiver-for = { $radio } için panadaptör alıcısı
settings-hang-up-stays-on = Bağlantıyı kes. Radyo { $station } üzerinde kalır.
settings-close-on-station = { $radio }, { $station } üzerinde kapatılsın mı? Yapılandırması o makinede kalır; buradaki sekme onunla birlikte kapanır, herkesinkiyle birlikte de.
settings-close-it-on = { $station } üzerinde kapat
settings-locator-hint = lokatör

## Ayarlar penceresi, Uyarılar (src/app/settings/alerts.rs)

settings-alerts-audible-alerts = Sesli uyarılar
settings-alerts-sound-alarm-when-decode-matters = Bir çözümleme önemli olduğunda alarm çal
settings-alerts-plays-over-own-audio-output = Kendi ses çıkışından çalar; bu yüzden bant, bu ekranla farklı bir hoparlördeyken bile duyulur.
settings-alerts-volume = Ses düzeyi
settings-alerts-output = Çıkış
settings-alerts-test = Dene
settings-alerts-what-sound = Neyin sesi çalsın
settings-alerts-each-station-quiet-while-after = Her istasyon bir uyarıdan sonra bir süre sessiz kalır, böylece yoğun bir bant her zaman dilimini çaldırmaz. Ses ve Ton + ses, sesli duyuru sesiyle okunur — Ayarlar → UI bölümünden açın.

## Ayarlar penceresi, Amplifikatör (src/app/settings/amp.rs)

settings-amp-amplifier = Amplifikatör
settings-amp-waiting-station-s-amplifier-configuration = İstasyonun amplifikatör yapılandırması bekleniyor…
settings-amp-hobbypcb-hardrock-500-hardrock-50 = USB seri bağlantı noktasındaki bir HobbyPCB Hardrock-500 ya da Hardrock-50. SDRoxide, metrelerini okur; bandını, modunu ve antenini değiştirir ve tuner'ını AMP penceresinden çalıştırır. Amplifikatör, daha önce olduğu gibi hâlâ radyonun PTT hattıyla anahtarlanır.
settings-amp-enable = Etkinleştir
settings-amp-model = Model
settings-amp-untested-panel-has-only-been = Test edilmemiş: bu panel yalnızca bir Hardrock-500 ile test edildi. Bu model için metre sınırları Hardrock-500'den ölçeklenmiştir ve yanlış olabilir. Amplifikatörün kendi ekranını izleyin.
settings-amp-port = Bağlantı noktası
settings-amp-choose = — seçin —
settings-amp-baud = Baud
settings-amp-must-match-amplifier-s-menu = Amplifikatörün menüsüyle eşleşmelidir. HobbyPCB'nin kendi denetim paneli 115200 ister.
settings-amp-follow-radio-s-band = Radyonun bandını izle
settings-amp-sends-transmit-frequency-amplifier-every = Her bant değişiminde yayın frekansını amplifikatöre gönderir; böylece filtreleri kendi CAT kablosu olmadan radyoyu izler. Bir yayın sırasında asla gönderilmez.
settings-amp-show-power-swr-tx-meter = Gücünü ve SWR'sini TX metresinde göster
settings-amp-only-where-radio-does-not = Yalnızca radyonun kendisinin ölçmediği yerde ve yalnızca amplifikatör hatta olduğunda.
settings-amp-off = Kapalı
settings-amp-connecting = Bağlanıyor…
settings-amp-status = Durum
settings-amp-apply = {" "}UYGULA{" "}
settings-amp-save-re-open-amplifier-s = Amplifikatörün bağlantı noktasını kaydet ve yeniden aç

## Ayarlar penceresi, Denetimler (src/app/settings/controls.rs)

settings-controls-keyboard = Klavye
settings-controls-click-shortcut-rebind-then-press = Yeniden atamak için bir kısayola tıklayın, sonra tuş kombinasyonuna basın (Esc iptal eder). Bir metin alanına yazarken atamalar yok sayılır.
settings-controls-shortcut = Kısayol
settings-controls-does = Ne yapar
settings-controls-step-mode = Adım / mod
settings-controls-accel = Hızlandırma
settings-controls-on = Açık
settings-controls-press-key = bir tuşa basın…
settings-controls-remove-binding = Bu atamayı kaldır
settings-controls-add-shortcut = + Kısayol ekle
settings-controls-restore-defaults = Varsayılanları geri yükle
settings-controls-bind-hold-talk-space = Basılı tutarak konuşmayı Space'e ata
settings-controls-hold-space-transmit-releasing-losing = Yayın için Space'i basılı tutun; bırakmak — ya da pencere odağını kaybetmek — tuşu serbest bırakır
settings-controls-unkey-held-ptt-after = Basılı PTT'yi şu süreden sonra serbest bırak
settings-controls-backstop-against-stuck-key-controller = Takılı kalmış bir tuşa ya da bildirmeyi kesen bir denetleyiciye karşı emniyet. 0 kapatır.
settings-controls-panadapter-mouse = Panadaptör faresi
settings-controls-wheel = Tekerlek
settings-controls-wheel-shift = Tekerlek + Shift
settings-controls-tune-step = Ayar adımı
settings-controls-zoom-rate = Yakınlaştırma hızı
settings-controls-click-tune-rounding = Tıklayarak ayarda yuvarlama
settings-controls-invert-wheel-direction = Tekerlek yönünü ters çevir
settings-controls-left-drag-tunes-well-pans = Sol sürükleme hem ayarlar hem kaydırır
settings-controls-off-makes-left-drag-pan = Kapalıyken sol sürükleme, sağ sürükleme gibi yalnızca görünümü kaydırır.
settings-controls-scroll-digit-frequency-readout-tune = Ayarlamak için frekans göstergesinde bir rakamı kaydırın
settings-controls-both-wheel-actions-are-tune = İki tekerlek eylemi de Ayar — geriye yakınlaştırmanın hiçbir yolu kalmıyor.
settings-controls-restore-mouse-defaults = Fare varsayılanlarını geri yükle
settings-controls-mouse-buttons = Fare düğmeleri
settings-controls-left-right-buttons-are-reserved = Sol ve sağ düğmeler ayarlama ve kaydırma için ayrılmıştır; orta ve ekstra düğmeler serbesttir. PTT için basılı tutulan bir yan düğme, bir ayak anahtarı gibi davranır.
settings-controls-add-mouse-button = + Fare düğmesi ekle
settings-controls-f1-always-opens-manual-even = F1 yazarken bile her zaman bu kılavuzu açar; bu yüzden yeniden atanamaz.
settings-controls-midi-controller = MIDI denetleyicisi
settings-controls-midi-controllers-need-native-app = MIDI denetleyicileri yerel uygulama gerektirir — tarayıcı istemcisinin MIDI erişimi yoktur.
settings-controls-any-class-compliant-midi-surface = Sınıf uyumlu her MIDI yüzeyi çalışır: bir DJ denetleyicisinin jog tekerleği güzel bir VFO düğmesi olur, pedleri PTT ve bant düğmeleri olur, kaydırıcıları kazanç denetimleri olur.
settings-controls-enable = Etkinleştir
settings-controls-controller = Denetleyici
settings-controls-feedback = Geri bildirim
settings-controls-rescan-ports = Bağlantı noktalarını yeniden tara
settings-controls-not-connected = Bağlı değil.
settings-controls-move-control-see-here = Burada görmek için bir denetimi hareket ettirin.
settings-controls-control = Denetim
settings-controls-reads = Okuma biçimi
settings-controls-led = LED
settings-controls-move = hareket ettirin…
settings-controls-click-then-move-control-you = Tıklayın, sonra atamak istediğiniz denetimi hareket ettirin
settings-controls-speed-sensitivity-spin-faster-tune = Hız hassasiyeti: daha hızlı ayarlamak için daha hızlı çevirin
settings-controls-send-current-value-back-light = LED'i yakmak ya da bir kaydırıcıyı hareket ettirmek için mevcut değeri geri gönderin
settings-controls-add-midi-control = + MIDI denetimi ekle
settings-controls-clear-all = Hepsini temizle
settings-controls-endless-jog-encoders-send-relative = Sonsuz (jog) kodlayıcılar, küçük hareketlerde birbirine benzeyen üç kodlamadan biriyle, bir konum yerine göreli bir adım gönderir. ÖĞREN saat yönündeki bir çevirmeden tahmin eder; düğme ters yönde ayarlıyorsa "rev" kutusunu işaretleyin.
settings-controls-none = — yok —

## Ayarlar penceresi, Genel (src/app/settings/general.rs)

settings-general-system-default = Sistem varsayılanı
settings-general-remote-access = Uzaktan erişim
settings-general-signed-in-station = Oturum açılmış istasyon
settings-general-sign-out = {" "}OTURUMU KAPAT{" "}
settings-general-sign-out-forgets-device = Bu cihazın oturumunu unutur. İstasyon bir sonraki bağlantıda yeniden soracak; bu bağlantı siz kapatana kadar açık kalır.
settings-general-password-kept-no-cookie = { $stations } için bu tarayıcıda bir parola hatırlanıyor, çünkü oturum çerezi verilmedi. Eski istasyonlar böyle yapar; aynı şey değildir.
settings-general-what-remote-client-browser-page = Uzak bir istemcinin — tarayıcı sayfasının ya da --connect ile başlatılmış başka bir sdroxide'ın — bu istasyonun işletmesine izin vermeden önce sağlaması gerekenler. Sunucu modunda (--server) geçerlidir; bir sonraki oturum açmada değişiklik, yeniden başlatmadan alınır.
settings-general-username = Kullanıcı adı
settings-general-password = Parola
settings-general-clients-must-give-password-leaving = İstemciler parolayı vermelidir. Kullanıcı adını boş bırakmak sorun değildir.
settings-general-clients-must-sign = İstemciler oturum açmalıdır.
settings-general-empty-anyone-who-can-reach = ⚠ Boş: sunucunun bağlantı noktasına erişebilen herkes, çağrı işaretinizle bu radyoyu işletebilir. Bağlantı noktasını yönlendirmeden önce bir parola ayarlayın.
settings-general-stored-clear-config-toml-like = sdroxide'ın tuttuğu diğer parolalar gibi config.toml içinde açık olarak saklanır.
settings-general-receive-audio-gain = Alma ses kazancı
settings-general-t-0-db = 0 dB
settings-general-extra-gain-everything-radio-sends =
    Bu radyonun hoparlörlere gönderdiği her şeye, ses düzeyi denetiminin üstüne ek kazanç. Radyo tam ses düzeyinde sessizse dışında 0 dB'de bırakın: ses düzeyi çubuğunun en üstü geldiği haliyle sestir, bu yüzden bir radyoyu yalnızca kısabilir, yükseltemez; bazı telsizlerin USB ses çıkışı da tam ölçeğin çok altındadır.

    6 dB'lik adımlarla yükseltin. Fazlası kırpar — ses tam ölçekte sınırlanır, sarılmaz; bu yüzden fazla yapmak yüksek değil, sert duyulur. Kayıtlar bundan önce alınır ve etkilenmez.
settings-general-reload-band-plan = BANT PLANINI YENİDEN YÜKLE
settings-general-re-read-bandplan-json-machine = Radyonun bağlı olduğu makinedeki bandplan.json dosyasını yeniden okuyup uygulayın — bant kenarları, alt segmentler ve skimmer pencereleri — yeniden başlatmadan.
settings-general-running-station-s-own-band = İstasyonun kendi bant planında çalışıyor.
settings-general-running-built-iaru-tables = Yerleşik IARU tablolarında çalışıyor.
settings-general-settings-file = Ayarlar dosyası
settings-general-settings-are-machine-radio-attached = Ayarlar, radyonun bağlı olduğu makinededir. Onları orada dışa aktarın.
settings-general-export = DIŞA AKTAR…
settings-general-write-every-setting-station-radios = Bu istasyondaki her ayarı — radyolar, modlar, sunucular, hafızalar, bant planı — başka bir kuruluma taşıyabileceğiniz tek bir dosyaya yazın. Günlüğünüz ve kayıtlı sunucu parolaları burada kalır.
settings-general-import = İÇE AKTAR…
settings-general-replace-station-s-settings-ones = Bu istasyonun ayarlarını, başka bir kurulumdan dışa aktarılmış bir dosyadakilerle değiştirin. Sonrasında sdroxide'ı yeniden başlatın.
settings-general-import-overwrites-what-here-file = İçe aktarma, buradakilerin üzerine dosya dosya yazar ve sdroxide bir sonraki başlatıldığında etkinleşir — aksi halde bellekteki ayarlar hemen üstüne geri yazılırdı. Dosyanın değinmediği her şey olduğu gibi kalır; bu yüzden tek radyolu bir istasyonun paketi buradaki ikinci bir radyoyu kaldırmaz. Günlüğünüz hiçbir zaman dosyada değildir: onu da taşımak isterseniz LOG penceresinden ADIF olarak dışa aktarın.
settings-general-sdroxide-settings = sdroxide ayarları
settings-general-swr-guard = SWR koruması
settings-general-stop-transmitting-high-swr = Yüksek SWR'de yayını durdur
settings-general-trip = Şu değerde kes
settings-general-stops-transmission-when-radio-reports =
    Radyo bu değerde ya da üstünde bir SWR bildirdiğinde yayını durdurur ve siz onaylayana dek yayını kilitli tutar. Kopmuş bir anteni, arızalı bir besleme hattını ya da yanlış porta alınmış bir anahtarı yakalar.

    Ayarlama farklı ele alınır, çünkü uyumsuzluğu beslemek onun amacıdır: bir anten tuner'ı iki katı sınırı alır ve koruma hiç devreye girmeden önce yaklaşık beş saniye. Bundan uzun süren elle bir tuner, oturum için korumanın kapatılmasını ister.

    CAT üzerinden SWR bildiren bir radyo gerektirir. Her yayının ilk beşte birlik kısmını yok sayar ve yüksek güç için beklemez.
settings-general-per-mode-settings = Mod başına ayarlar
settings-general-reset-every-mode = HER MODU SIFIRLA
settings-general-forget-every-mode-s-remembered = Her modun hatırlanan AGC, susturma, gürültü azaltma, çentik ve stereo anahtarlarını unutun ve her modun kendi varsayılanlarını geri koyun.
settings-general-selecting-mode-lays-mode-s = Bir mod seçmek, o modun AGC, susturma, gürültü azaltma, çentik ve stereo anahtarları için kendi başlangıç değerlerini alıcıya yerleştirir; birini değiştirmek bunu yalnızca o mod için hatırlar. Alıcının filtre/gürültü satırının sonundaki dairesel ok simgesi, değiştirilenleri listeler ve geçerli modu eski haline getirir.
settings-general-your-audio-speakers-microphone = Sesiniz (hoparlörler / mikrofon)
settings-general-output = Çıkış
settings-general-channels = Kanallar
settings-general-which-speaker-channel-s-received = Alınan sesin hangi hoparlör kanalına ulaştığı. Sol ya da sağ, tek kanala indirilmiş her şeyi yalnızca o tarafa koyar.
settings-general-input = Giriş

## Ayarlar penceresi, ağ hizmetleri (src/app/settings/net.rs)

settings-net-operator = Operatör
settings-net-callsign-grid-are-set-general = Çağrı işareti ve lokatör Genel sekmesinde ayarlanır.
settings-net-set-your-callsign-grid-general = ⚠ Çağrı işaretinizi ve lokatörünüzü Genel sekmesinde ayarlayın.
settings-net-freedv-reporter = FreeDV Reporter
settings-net-enable = Etkinleştir
settings-net-connects-whenever-enabled-your-station = Etkinleştirildiğinde her zaman bağlanır. İstasyonunuz, yalnızca radyo RADE modundayken başkalarına gösterilir — diğer herhangi bir modda bağlı kalırsınız ama gizli olursunuz.
settings-net-station = İstasyon
settings-net-message = İleti
settings-net-receive-only-i-cannot-transmit = Yalnızca alma (yayın yapamam)
settings-net-server = Sunucu
settings-net-host = Ana makine
settings-net-port = Bağlantı noktası
settings-net-tls-wss = TLS (wss://)
settings-net-not-yet-implemented-freedv-gui = Henüz uygulanmadı — FreeDV GUI de düz ws:// kullanır.
settings-net-reporting = Bildirim
settings-net-report-stations-i-decode = Çözdüğüm istasyonları bildir
settings-net-sends-rx-report-each-callsign = Bir RADE Over Sonu çerçevesinden kurtarılan her çağrı işareti için bir rx_report gönderir.
settings-net-show-other-reporter-stations-spots = Diğer bildirici istasyonları spot olarak göster
settings-net-adds-them-panadapter-overlay-world = Bunları FREEDV filtresi altında panadaptör katmanına, dünya haritasına ve SPOTS penceresine ekler.
settings-net-set-your-callsign-grid-general-2 = ⚠ Çağrı işaretinizi ve lokatörünüzü Genel sekmesinde ayarlayın. İkisi de olmadan bağlantı yalnızca görüntüleme içindir: diğer istasyonları görürsünüz ama kendiniz görünmezsiniz.
settings-net-apply = {" "}UYGULA{" "}
settings-net-persist-re-connect = Kaydet ve (yeniden) bağlan
settings-net-longwave-shortwave-stations-labelled-waterfall = Şelalede etiketlenen uzun dalga ve kısa dalga istasyonları. Kısa dalga programı, geçerli yayın mevsimi için eibispace.de'den indirilir ve mevsim değiştiğinde yeniden indirilir.
settings-net-season = Mevsim
settings-net-downloading = indiriliyor…
settings-net-using-built-copy = yerleşik kopya kullanılıyor
settings-net-your-own-stations-corrections-go = Kendi istasyonlarınız ve düzeltmeleriniz bu dosyaya gider. sdroxide onu asla yazmaz ve buradaki bir girdi, aynı ad ve frekansa sahip zamanlanmış girdinin yerini alır.
settings-net-reload = Yeniden yükle
settings-net-re-read-your-station-file = Düzenledikten sonra istasyon dosyanızı yeniden okuyun
settings-net-download-schedule-now = Programı şimdi indir
settings-net-fetch-season-s-schedule-again = Bu mevsimin programını yeniden al, önbellekteki kopyanın yerine
settings-net-broadcast-stations-labelled-waterfall-come = Şelalede etiketlenen yayın istasyonları, bu derlemeye gömülü listeden gelir. Onları düzenlemek masaüstü uygulamasını gerektirir.
settings-net-ask-service-whether-these-credentials = Hizmete bu kimlik bilgilerinin çalışıp çalışmadığını sorun. Önce yukarıdaki ayarları uygular ve hiçbir şey günlüğe yazmaz: yalnızca okur.
settings-net-checking = denetleniyor…

## Ayarlar penceresi: değerleri doldurulmuş iletiler.

settings-amp-connected-to = { $model } modeline bağlanıldı
settings-controls-last-midi = Son ileti: { $message }  değer { $value }
settings-general-band-plan-file = Her bant kenarı ve alt segmenti { $path } dosyasından gelir; dosya ilk seferinde yerleşik IARU tablolarından yazılır ve sonrasında düzenlemek sizindir — bir bandı lisansınıza daraltırsanız sdroxide onun dışında yayın yapmayı reddeder. Frekanslar MHz cinsindendir; varsayılanların yeni bir kopyası için dosyayı silin. Bu bölgesel tahsistir, lisansınız değildir: kendi koşullarınız daha dar olabilir ve ulusal planlar bir bölge içinde farklılık gösterir.
settings-general-exported = { $note } dışa aktarıldı — kaydedileceği yeri seçin.
settings-general-export-failed = Dışa aktarma başarısız: { $error }
settings-general-import-failed = İçe aktarma başarısız: { $error }
settings-general-imported = { $summary } — kullanmak için sdroxide'ı yeniden başlatın.
settings-general-import-skipped = { $path } atlandı: { $why }
settings-general-swr-tripped = ⚠ Şu anda { $swr }:1 değerinde kesildi — yayın kilitli.
settings-net-call-grid-set-on-general = { $call } / { $grid } — Genel sekmesinde ayarlandı
settings-net-reporting-as = { $call } / { $grid } olarak bildiriliyor — SDRoxide { $version }
settings-net-schedule-download-failed = ⚠ İndirme başarısız: { $error }. Yerleşik program kullanılıyor.
settings-net-test-service = { $service } testi

## Ayarlar penceresi (src/app/settings/mod.rs), devamı

settings-transmit-off-while-converter-set = Bir dönüştürücü ayarlıyken yayın kapalıdır — onu yeniden açmak için yukarıdaki yayın hattında ne olduğunu belirtin.
settings-transmit-goes-through-same-converter = Yayın aynı dönüştürücüden geçer; böylece radyo iki yönde de skaladan aynı uzaklıkta çalışır.
settings-transmit-not-converted-radio-transmits = Yayın dönüştürülmez: radyo skalanın gösterdiği frekansta yayın yapar, alma ise dönüştürücüden gelir.
settings-transmit-takes-own-offset-separate = Yayın, alma kaymasından ayrı olarak kendi kaymasını alır.
settings-no-enabled-subscriptions-update = Güncellenecek etkin abonelik yok.
settings-add-radio-here-station-you = Bir radyo ekleyin — burada ya da bağlı olduğunuz bir istasyonda

## Ayarlar penceresi, Genel, devamı

settings-general-every-band-edge-sub-segment = Her bant kenarı ve alt segmenti, radyonun bağlı olduğu makinedeki bandplan.json dosyasından gelir. Bu bölgesel tahsistir, lisansınız değildir: kendi koşullarınız daha dar olabilir ve ulusal planlar bir bölge içinde farklılık gösterir.

## Ayarlar penceresi, resim yükleme (src/app/settings/picup.rs)

settings-picup-received-pictures-ftp = Alınan resimler → FTP
settings-picup-uploads-every-received-sstv-picture = Kaydedildiği anda alınan her SSTV resmini (ve işaretliyse her hava haritasını) yükler; QSSTV'nin yaptığı biçimde düzenlenir, böylece QSSTV için yazılmış web sayfaları değişmeden çalışır.
settings-picup-upload-received-pictures = Alınan resimleri yükle
settings-picup-server = Sunucu
settings-picup-port = Bağlantı noktası
settings-picup-security = Güvenlik
settings-picup-user = Kullanıcı
settings-picup-password = Parola
settings-picup-certificate = Sertifika
settings-picup-trusted-when-computer-s-operating = Bu bilgisayarın işletim sistemi ona güvendiğinde güvenilir; tıpkı buradaki bir tarayıcının yapacağı gibi. Aşağıya bir özel CA eklenebilir (PEM).
settings-picup-extra-ca = Ek CA
settings-picup-only-certificate-fingerprint-accepted-way = Yalnızca bu parmak izine sahip sertifika kabul edilir — kendinden imzalı bir sunucuya güvenmenin yolu budur. TEST, sunucunukini gösterir.
settings-picup-any-certificate-accepted-link-encrypted = ⚠ Herhangi bir sertifika kabul edilir. Bağlantı şifrelidir, ama yol üzerindeki herkes sunucu gibi davranıp parolayı okuyabilir.
settings-picup-plain-ftp-sends-password-pictures = ⚠ Düz FTP, parolayı ve resimleri şifresiz gönderir.
settings-picup-sstv = SSTV
settings-picup-sstv-dir = SSTV dizini
settings-picup-weather-fax = Hava faksı
settings-picup-wefax-dir = WEFAX dizini
settings-picup-empty-login-directory = Boş bırakılırsa giriş dizinidir.
settings-picup-file-names = Dosya adları
settings-picup-format = Biçim
settings-picup-add-extension = uzantı ekle
settings-picup-image1-png-rather-than-qsstv = QSSTV'nin çıplak image1 yerine image1.png
settings-picup-caption-bar = Başlık çubuğu
settings-picup-time-frequency-mode-sender-s = Zaman, frekans, mod ve göndericinin FSK kimliği üstte, QSSTV'nin çizdiği gibi. Yalnızca yüklenen kopya işaretlenir.
settings-picup-fit-960-768 = 960 × 768'e sığdır
settings-picup-qsstv-s-heartbeat-tiny-picture = QSSTV'nin nabzı: yüksekliği dakikayı gösteren küçük bir resim; her birinin yanında yüklenir ve program kapandığında iki piksel genişliğe getirilir.
settings-picup-nat-workaround = NAT geçici çözümü
settings-picup-ignore-private-address-server-s = Sunucunun pasif mod yanıtındaki özel adresi yok sayın ve sunucunun kendi adresini kullanın — kendi genel adresini bilmeyen, NAT arkasındaki bir sunucu için.
settings-picup-apply = {" "}UYGULA{" "}
settings-picup-test = TEST
settings-picup-connect-settings-above-applied-not = Yukarıdaki ayarlarla — uygulanmış ya da uygulanmamış olsun — bağlanın, oturum açın ve her yükleme dizinine küçük bir dosya yazıp silin.
settings-picup-checking = denetleniyor…
settings-picup-trust-certificate = Bu sertifikaya güven
settings-picup-pin-switch-pinned-fingerprint-one = Sabitleyin: bunla birlikte "Sabitlenmiş parmak izi" seçeneğine geçin. Önce sunucuyla karşılaştırın — bu, sunucunun söylediği şeydir, kim olduğunun kanıtı değil. Sonra UYGULA.
settings-picup-last-upload = Son yükleme

## Ayarlar penceresi, Profiller (src/app/settings/profiles.rs)

settings-profiles-profiles-station-s-saved-setups = Profiller: istasyonun kayıtlı kurulumları
settings-profiles-profile-captures-whole-working-setup = Bir profil, bütün çalışma kurulumunu yakalar — skalalar ve VFO'lar, mod ve filtreler, kazançlar, sürüş ve antenler, çağrı işaretiniz ve ileti şablonları ve bant başına kayıtlar — ve bunu tek tıklamayla geri uygular. Donanım (arka uç, ses aygıtları, dönüştürücüler) bilerek dokunulmadan bırakılır: profil, radyoyu değil istasyonu nasıl işlettiğinizdir.
settings-profiles-save-current-setup = Geçerli kurulumu kaydet
settings-profiles-profile-name = profil adı
settings-profiles-save = Kaydet
settings-profiles-applies-whatever-sdroxide-attached-dials = sdroxide'ın bağlı olduğu her şeye uygulanır: skalalar ve VFO'lar, mod ve filtreler, seviyeler, kazançlar, sürüş, antenler, dijital kimlik ve bant yığınları.
settings-profiles-no-profiles-saved-yet-name = Henüz kayıtlı profil yok — yukarıya bir ad yazın ve Kaydet'e basın.
settings-profiles-saved-profiles = Kayıtlı profiller
settings-profiles-apply = Uygula

## Ayarlar penceresi, T/R anahtarı (src/app/settings/relay.rs)

settings-relay-transmit-receive-switch = Yayın/alma anahtarı
settings-relay-waiting-station-s-t-r = İstasyonun T/R anahtarı yapılandırması bekleniyor…
settings-relay-closes-contact-while-station-transmits = Bu istasyon yayın yaparken bir kontağı kapatır — SDR'nin anten girişini ayırıp topraklamak ve bununla sırayla bir amplifikatörü ya da harici bir T/R rölesini anahtarlamak için. Ucuz USB röle kartlarıyla (LCUS, KMtronic, Numato), PTT kapanışı isteyen herhangi bir arayüze seri bir RTS/DTR hattıyla ve bir komut kancasıyla başka her şeyle çalışır.
settings-relay-hardware = Donanım
settings-relay-board = Kart
settings-relay-lcus-four-byte-a0-command = LCUS, bir düzine adla satılan dört baytlık A0 komutlu karttır; KMtronic ve Numato'ya da hangi duruma ayarlı olduklarını sorulabilir
settings-relay-port = Bağlantı noktası
settings-relay-choose = — seçin —
settings-relay-device = Aygıt
settings-relay-gpio-pin = GPIO pini
settings-relay-pin-3-every-homebrew-plan = Her ev yapımı planda ve her ticari kartta 3. pin — paketin sonunda bir telin bağlanabileceği pin budur. Röle tıklamıyorsa kart, bu pinleri hiç uygulamayan bir kopyadır.
settings-relay-gpio-chip = GPIO çipi
settings-relay-linux-only-gpiodetect-lists-chips = Yalnızca Linux. `gpiodetect` makinedeki çipleri listeler.
settings-relay-transmit = Yayında
settings-relay-receive = Alımda
settings-relay-nothing-switched = Hiçbir şey anahtarlanmaz.
settings-relay-starting-program-costs-unpredictable-amount = Bir program başlatmak, öngörülemeyen bir süre alır ve bunun hiçbiri öncü süreye sayılmaz — bu yüzden burada bir röle kartı için olacağından daha geniş bir pay bırakın. Kontak tablosundaki sayılar bu bağlantı için bir anlam ifade etmez: komutlarınız tuşa basıldığında ve bırakıldığında birer kez çalışır.
settings-relay-contacts = Kontaklar
settings-relay-contact-1-rts-contact-2 = Kontak 1 RTS, kontak 2 DTR'dir.
settings-relay-no = No.
settings-relay-gpio = GPIO
settings-relay-name = Ad
settings-relay-job = İş
settings-relay-tx-closes = Yayında kapanır
settings-relay-lead = Öncü
settings-relay-hold = Tutma
settings-relay-line-s-offset-chip-bcm = Hattın bu çipteki ofseti — bir Raspberry Pi başlığındaki BCM numarası, fiziksel pin numarası değildir
settings-relay-sdr-antenna = SDR anteni
settings-relay-decides-default-timings-what-log = Varsayılan zamanlamaları ve günlüğün ona ne dediğini belirler — ve bir bant çözücü için kontağın aşağıdaki bant tablosunu izlediğini
settings-relay-transmitting-energises-coil-off-transmitting =
    Açık: yayın bobini enerjilendirir. Kapalı: yayın onu serbest bırakır.

    Bu, bir ayardan önce bir kablolama kararıdır — aşağıdaki nota bakın.
settings-relay-how-long-before-rf-contact = Bu kontak, RF'den ne kadar önce bandının RX sözcüğünden TX sözcüğüne geçer. Bir ayarlama onu hemen taşır; bu, tuşa basıldığında içindir, güç gelmeden önce yalnızca yayın filtresinin devrede olması gerektiğinde.
settings-relay-closed-long-before-rf-small = RF'den bu kadar önce kapatılır. Küçük bir koaksiyel röle 5 ila 15 ms'de atar. Yayın, bunların en uzununu bekler; bu yüzden büyük bir değer alma sesinde duyulabilir bir boşluktur; 250 ms'nin üstündekiler yok sayılır.
settings-relay-how-long-after-rf-stops = RF durduktan ne kadar sonra bu kontak, TX sözcüğünden RX sözcüğüne geri döner.
settings-relay-opened-long-after-rf-stops = RF durduktan bu kadar sonra açılır. Bilerek öncüden uzun: anteni biraz geç geri bağlamanın maliyeti yoktur, erken geri bağlamanın ise bir ön uca maliyeti vardır. Sıfır gerçek bir cevaptır ve bir amplifikatörün anahtar hattı için doğru olanıdır.
settings-relay-test = {" "}DENE{" "}
settings-relay-not-while-station-air = İstasyon havadayken değil
settings-relay-close-contact-half-second-so = Bu kontağı yarım saniye kapatın; böylece röleyi duyabilir ve verici soğukken bağlantıyı kontrol edebilirsiniz
settings-relay-remove = Kaldır
settings-relay-add-contact = + kontak ekle
settings-relay-if-will-not-answer = Yanıt vermezse
settings-relay-switch-exists-protect-receiver-not = Var olma amacı bir alıcıyı korumak olan bir anahtar, sessizce korumayı bırakıyorsa değmez — bu yüzden reddetmek varsayılandır. Gevşek bir USB kablosunun yarışmanızı bitirdiği durumun daha kötü sonuç olduğu yerde diğerini seçin.
settings-relay-order-timings-longest-lead-closes = Sıra zamanlamalardır: en uzun öncü önce kapanır, en uzun tutma en son açılır. Bu yüzden anten rölesine daha uzun öncü ve daha uzun tutma verin, amplifikatöre ise her birinden daha kısa olanı — böylece anten, amplifikatör anahtarlanmadan önce her zaman değiştirilir ve amplifikatör, anten geri gelmeden önce her zaman tuşu bırakır.
settings-relay-transmit-sense-input = Yayın algılama girişi
settings-relay-when-transceiver-keys-itself-own = Bir telsiz kendini anahtarladığında — kendi mikrofon düğmesiyle, ayak anahtarıyla, VOX ya da keyer'ıyla — sdroxide bunu CAT üzerinden sorarak öğrenir ve bu yanıt, yayının birkaç yüz milisaniye içinde gelir. Telsizin SEND hattını, bir optik izolatör üzerinden, aynı porttaki bir el sıkışma girişine bağlarsanız, bu birkaç milisaniye içinde görülür. Metre ve yayın kilidi bununla birlikte hızlanır.
settings-relay-line = Hat
settings-relay-transmitting = Yayında olmak
settings-relay-high-line = yüksek hat
settings-relay-most-opto-isolated-interfaces-pull = Çoğu optik izoleli arayüz, telsiz anahtarlandığında hattı aşağı çeker; bu yüzden genellikle kapalıdır
settings-relay-belongs-radio = Bağlı olduğu radyo
settings-relay-which-radio-tab-sensed-transceiver = Algılanan telsizin hangi radyo sekmesinde olduğu. Tek radyolu bir istasyonda bu 1'dir ve hiçbir şey buna bağlı değildir.
settings-relay-needs-serial-port-handshake-lines = Bir seri bağlantı noktası gerektirir — el sıkışma hatları onun üzerindedir.
settings-relay-wiring-only-fail-safe-survives = Bu programdan sağ kurtulan tek emniyet, kablolamadır.
settings-relay-choose-tx-closes-so-de = “TX closes” seçin; böylece kart çekildiğinde, bilgisayar kapalıyken ya da sdroxide çalışmazken istediğiniz durum *enerjisiz* kontak olsun. Enerjisiz olmanın topraklı anlamına gelecek şekilde kablolandığında, ölü bir röle SDR'yi sağır bırakır — rahatsız edici ama güvenli. Diğer yönde kablolandığında ölü bir röle ve tek bir yayın, ölü bir ön uç demektir. Arkasında gerçek bir amplifikatör varsa ilkini alın.
settings-relay-cannot-protect-first-instant-over = Bu, radyodan başlattığınız bir yayının ilk anını koruyamaz. Telsizi sdroxide'dan anahtarlarsanız kontaklar her zaman RF'nin önüne geçer; yukarıdaki algılama girişini kablolarsanız birkaç milisaniye içinde takip ederler. Gerçekten korunmaya değer bir ön uç için RF algılamalı bir donanım T/R anahtarı da kullanın — bir bilgisayardaki hiçbir program tek başına yeterli bir çözüm olamaz.
settings-relay-contacts-transmit = YAYINDA kontaklar
settings-relay-not-open = Açık değil.
settings-relay-status-unknown-press-apply = Durum bilinmiyor — UYGULA'ya basın.
settings-relay-station-air-apply-will-take = İstasyon havada — UYGULA bu yayından sonra etkinleşecek.
settings-relay-band-decoder-outputs = Bant çözücü çıkışları
settings-relay-one-control-word-per-band = Bant başına bir denetim sözcüğü: bit 0 kontak 1, bit 1 kontak 2 ve böyle devam eder; yukarıdaki numaralandırmayla eşleşir. RX, alma skalasının bandını izler; TX ise hangisi anahtarladıysa onun bandını, her kontağın öncüsünde RF'den önce devreye girerek ve tutmasından sonra geri çıkarak. RF'nin hangi yöne gittiği önemsemeyen bir filtre için ikisine aynı değeri verin; yalnızca alma yapan bir ön yükseltici atlatması ya da yalnızca yayın yapan bir LPF bankası için farklı değerler verin. 0'da bırakılan bantlar hiçbir şey etkinleştirmez. Uygula / yeniden bağlan ile etkinleşir.
settings-relay-band = Bant
settings-relay-rx = RX
settings-relay-tx = TX
settings-relay-outputs-asserted = Çıkışlar etkin
settings-relay-other = Diğer
settings-relay-everywhere-outside-amateur-bands-short = Amatör bantların dışında her yerde — kısa dalga dinleme ve bir transverter skalasının hiçbir bandın kapsamadığı yere düştüğü her şey.
settings-relay-apply = {" "}UYGULA{" "}
settings-relay-persist-re-open-switch = Kaydet ve anahtarı (yeniden) aç

## Ayarlar penceresi, Uzak (src/app/settings/remote.rs)

settings-remote-connect-server = Bir sunucuya bağlan
settings-remote-drive-sdroxide-running-somewhere-else = Başka bir yerde çalışan bir sdroxide'ı — istasyon makinesini, uzak bir siteyi — ağ üzerinden yönetin. Orada --server ile başlatın ve adresini buraya girin. Radyo, çözücüler ve günlük o makinede kalır; bu ekran şelaleyi ve sesi alır, kullandığınız hoparlör ve mikrofon de bunlardır.
settings-remote-address = Adres
settings-remote-host-name-ip = ana makine adı ya da IP
settings-remote-machine-running-sdroxide-server-host = sdroxide --server çalıştıran makine: bir ana makine adı, bir IPv4 ya da IPv6 adresi ya da sunucu bir ters vekilin arkasındaysa tam bir ws:// (ya da wss://) URL'si — bir URL yazıldığı gibi kullanılır ve aşağıdaki bağlantı noktasını ve anahtarı yok sayar.
settings-remote-port = Bağlantı noktası
settings-remote-port-server-listens-server-port = Sunucunun dinlediği bağlantı noktası — config.toml içindeki server_port. --port verilmediyse 4950.
settings-remote-secure = Güvenli
settings-remote-wss-tls = wss:// (TLS)
settings-remote-tick-where-server-sits-behind = Sunucu, HTTPS'i onun yerine sonlandıran bir şeyin arkasındaysa bunu işaretleyin — 443 numaralı bağlantı noktasındaki bir ters vekil; bir istasyona açık internet üzerinden ulaşmanın olağan yolu budur. sdroxide'ın kendi sunucusu düz ws:// konuşur ve bunu istemez.
settings-remote-connect = {" "}BAĞLAN{" "}
settings-remote-open-station-radio-tab-own = Bu istasyonu kendi radyo sekmesi olarak aç
settings-remote-client-cannot-hold-second-connection = Bu istemci ikinci bir bağlantı tutamaz.
settings-remote-station-appears-tab-beside-radios =
    İstasyon, burada zaten açık olan radyoların yanında bir sekme olarak görünür; pencerenin üstündeki sekme şeridi aralarında geçiş yapar (⊞ ikisini yan yana koyar). Onu Radio sekmesinin üstündeki listeden kapatın — bu bağlantıyı keser; sunucudaki hiçbir şey değişmez.

    Sunucu bir kullanıcı adı ve parola isterse, oturum açma ekranı yeni sekmede açılır. Düz bir ws:// bağlantısı bunların hiçbirini şifrelemez; bu yüzden açık internet üzerinden ya bir VPN ya da SSH tüneli kullanın ya da sunucunun önüne bir HTTPS sonlandırıcı koyup yukarıdaki Güvenli seçeneğini işaretleyin.

## Ayarlar penceresi, Sunucular (src/app/settings/servers.rs)

settings-servers-wsjt-x-udp-broadcast = WSJT-X UDP yayını
settings-servers-waiting-station-s-wsjt-x = İstasyonun WSJT-X yayın yapılandırması bekleniyor…
settings-servers-sends-decodes-station-status-logged = Çözümlemeleri, istasyon durumunu ve günlüğe yazılan QSO'ları WSJT-X'in yaptığı gibi gönderir; böylece GridTracker, JTAlert, N1MM+ ve Log4OM sdroxide ile değişmeden çalışır. Yalnızca çıkış — bu soketteki hiçbir şey radyoya dokunamaz.
settings-servers-enable = Etkinleştir
settings-servers-send = Gönderilecek adres
settings-servers-t-127-0-0-1-reaches = 127.0.0.1 bu makinedeki istemcilere ulaşır; bir LAN adresi ya da çok noktalı yayın grubu (224.0.0.1) diğerlerine ulaşır
settings-servers-port = Bağlantı noktası
settings-servers-t-2237-port-every-client-defaults = 2237, her istemcinin varsayılan olarak kullandığı bağlantı noktasıdır
settings-servers-identify = Kendini şöyle tanıt
settings-servers-name-clients-see-some-loggers = İstemcilerin gördüğü ad. Bazı günlükleyiciler yalnızca kendini WSJT-X olarak tanıtan trafiği kabul eder.
settings-servers-n1mm-contactinfo-broadcast = N1MM+ contactinfo yayını
settings-servers-same-news-n1mm-s-words = Aynı haber N1MM'nin dilinde: kaydedilen her temas için bir XML datagramı; WSJT-X'i değil N1MM'yi dinleyen günlükleyiciler için — Dünya Radyo Ligi'nin köprüsü ikisini de alır. Yalnızca temaslar; bu protokolde bir çözümleme akışı yoktur. Bir istasyon ikisini de isteyebileceği için kendi anahtarı ve kendi bağlantı noktası vardır.
settings-servers-t-127-0-0-1-reaches-2 = 127.0.0.1 bu makinedeki istemcilere ulaşır. N1MM'nin bir yarışma ağı için kendi önerisi bu alt ağın yayın adresidir — 192.168.1.n ağı için 192.168.1.255 — ve bu her konuma aynı anda ulaşır.
settings-servers-t-12060-port-n1mm-s-documentation = 12060, N1MM'nin belgelerinin önerdiği bağlantı noktasıdır
settings-servers-station-name = İstasyon adı
settings-servers-what-n1mm-calls-stationname-name = N1MM'nin StationName dediği şey: paketi gönderen bilgisayarın adı. Günlükleyiciler, bir işletim konumunu diğerinden ayırt etmek için bunu gösterir.
settings-servers-decoded-cw-over-udp = UDP üzerinden çözülen CW
settings-servers-cw-panel-s-copy-plain = CW panelinin kopyası düz metin olarak, oturduğu her parça için bir UDP datagramı — bir istasyon denetleyicisi, bir günlükleyicinin CW penceresi ya da bir betik için. Protokol yoktur: bir dinleyici, gelenleri sadece ekler (nc -ul 9999 deneyin). Yalnızca oturmuş metin gönderilir; çözücünün hâlâ düzeltmekte olduğu kuyruk asla gönderilmez. Her radyonun kendi anahtarı vardır.
settings-servers-t-127-0-0-1-reaches-3 = 127.0.0.1 bu makinedeki programlara ulaşır; bir LAN adresi ya da bu alt ağın yayın adresi diğerlerine ulaşır.
settings-servers-t-9999-default = Varsayılan olarak 9999
settings-servers-apply = {" "}UYGULA{" "}
settings-servers-persist-re-open-broadcast-sockets = Kaydet ve yayın soketlerini (yeniden) aç
settings-servers-hamlib-rigctld-server = Hamlib rigctld sunucusu
settings-servers-waiting-station-s-rigctld-configuration = İstasyonun rigctld yapılandırması bekleniyor…
settings-servers-listen = Dinlenecek adres
settings-servers-t-127-0-0-1-machine = 127.0.0.1 (bu makine)
settings-servers-t-0-0-0-0-whole = 0.0.0.0 (tüm ağ)
settings-servers-t-4532-port-every-rigctld-client = 4532, her rigctld istemcisinin varsaydığı bağlantı noktasıdır
settings-servers-rig-name = Telsiz adı
settings-servers-reported-clients = istemcilere bildirilen
settings-servers-max-clients = En fazla istemci
settings-servers-allow-clients-transmit = İstemcilerin yayın yapmasına izin ver
settings-servers-off-refuses-every-key-request = Kapalıyken her anahtarlama isteği reddedilir ve bir yayın aralığı ilan edilmez; böylece Hamlib kendisi anahtarlamayı reddeder.
settings-servers-rigctld-has-no-authentication-0 = ⚠ rigctld kimlik doğrulaması yapmaz. 0.0.0.0 üzerinde, bu bağlantı noktasına erişebilen herkes verici ayarlayabilir ve anahtarlayabilir.
settings-servers-t-1-client = 1 istemci
settings-servers-not-running = Çalışmıyor.
settings-servers-status-unknown-press-apply = Durum bilinmiyor — UYGULA'ya basın.
settings-servers-persist-re-bind-server = Kaydet ve sunucuyu (yeniden) bağla
settings-servers-lets-any-hamlib-capable-program = Hamlib destekli herhangi bir programın bu radyoyu yönetmesine izin verir: frekans, mod, PTT, split, RIT ve güç. WSJT-X, fldigi ya da CQRLOG'da “Hamlib NET rigctl” telsizini (model 2) seçip bu adrese yönlendirin; GPredict ve N1MM'de ana makine ve bağlantı noktasını doğrudan girin. TCI sunucusunun aksine yalnızca denetim taşır — ses yok, IQ yok.
settings-servers-antenna-rotator = Anten döndürücü
settings-servers-waiting-station-s-rotator-configuration = İstasyonun döndürücü yapılandırması bekleniyor…
settings-servers-points-motorized-antenna-satellite-you = Motorlu bir anteni, kilitlendiğiniz uyduya, bir Hamlib rotctld servisi üzerinden yöneltir (örneğin bir GS-232B döndürücü için “rotctld -m 603 -r /dev/ttyUSB0”, ya da donanım olmadan denemek için “rotctld -m 1”) — tek bir servis Hamlib'in sürdüğü her döndürücüyü kapsar — ya da Setup'ında UDP Control etkinleştirilmiş PSTRotator üzerinden.
settings-servers-program = Program
settings-servers-host = Ana makine
settings-servers-t-4533-rotctld-s-default-pstrotator = 4533, rotctld'nin varsayılanıdır. PSTRotator'ın UDP denetim bağlantı noktası varsayılan olarak 12000'dir ve konum sorgularını bir üst bağlantı noktasında, 12001'de yanıtlar — bu makinede boş olmalıdır.
settings-servers-min-elevation = En düşük yükseklik
settings-servers-below-rotator-parks-instead-tracking = Bunun altında döndürücü izlemek yerine park eder — yerel ufkunuza ya da çatı hattınıza göre ayarlayın
settings-servers-azimuth-offset = Azimut kayması
settings-servers-added-every-command-rotator-whose = Kuzeyi kayık bir döndürücü için her komuta eklenir
settings-servers-min-movement = En az hareket
settings-servers-steps-smaller-than-are-not = Bundan küçük adımlar gönderilmez — derecenin onda birlerini kovalamak motorları boşuna yıpratır
settings-servers-park-position = Park konumu
settings-servers-drive-position-when-idle = boştayken bir konuma sür
settings-servers-off-leaves-antenna-wherever-last = Kapalıyken anten, son geçişin bittiği yerde kalır
settings-servers-not-connected = Bağlı değil.
settings-servers-persist-re-connect-daemon = Kaydet ve servise (yeniden) bağlan
settings-servers-waiting-station-s-tci-server = İstasyonun TCI sunucu yapılandırması bekleniyor…
settings-servers-built-tci-server = Yerleşik TCI sunucusu
settings-servers-device-name = Aygıt adı
settings-servers-off-leaves-control-receive-streams = Kapalıyken denetim ve alma akışları çalışmaya devam eder, ama her anahtarlama isteği reddedilir.
settings-servers-tci-has-no-authentication-0 = ⚠ TCI kimlik doğrulaması yapmaz. 0.0.0.0 üzerinde, bu bağlantı noktasına erişebilen herkes verici ayarlayabilir ve anahtarlayabilir.
settings-servers-tci-client = TCI İstemcisi
settings-servers-when-you-tune-send-radio = Ayar yaptığınızda bu radyonun VFO'sunu başka bir TCI sunucusuna gönderir; böylece o telsiz birlikte ayarlanır. Yalnızca denetim — ses yok, IQ yok ve verici anahtarlanmaz.
settings-servers-server = Sunucu
settings-servers-same-address-server-above-client = Bu, yukarıdaki sunucuyla aynı adres. İstemci yalnızca o sunucu dinleyen olmadığında bağlanır — 590Ctrl bu bağlantı noktasını kullanırken kapalı bırakın ya da başka bir bağlantı noktasına alın.
settings-servers-tci-client-connected = ● TCI İstemcisi bağlı
settings-servers-tci-client-connecting = TCI İstemcisi bağlanıyor…
settings-servers-tci-client-press-apply = TCI İstemcisi — UYGULA'ya basın.
settings-servers-persist-re-bind-server-connect = Kaydet ve sunucuyu (yeniden) bağla, TCI istemcisini bağla
settings-servers-server-lets-tci-capable-programs = Sunucu, TCI destekli programların bu radyoyu yönetmesine izin verir: frekans ve mod denetimi, geniş bant IQ, alma sesi ve yayın sesi. WSJT-X'te SunSDR (TCI) telsiz türünü seçip bu adrese yönlendirin ve PTT'yi TCI olarak ayarlayın. TCI İstemcisi ise diğer yöndedir: sdroxide'ın skalası, yukarıdaki adresteki 590Ctrl'ye (ya da herhangi bir TCI sunucusuna) bir VFO komutu olarak gönderilir. Ses geçirmez.

## Ayarlar penceresi, TLE (src/app/settings/tle.rs)

settings-tle-satellites-element-sets-frequencies = Uydular: eleman setleri ve frekanslar
settings-tle-waiting-station-s-satellite-configuration = İstasyonun uydu yapılandırması bekleniyor…
settings-tle-tracker-already-fetches-celestrak-s = İzleyici, CelesTrak'ın amatör grubunu zaten kendi başına alır. Bu, geri kalan her şey içindir: NOAA hava uyduları, grupta yer almak için çok yeni bir cubesat ya da gelen elemanlardan daha taze bir eleman seti. Listeler, radyo motorunun çalıştığı yerde alınır ve tutulur; bu yüzden uygulama nerede açık olursa olsun uydu kümesi aynıdır.
settings-tle-subscriptions = Abonelikler
settings-tle-listings-fetched-kept-current-same = Listeler, amatör kümesiyle aynı altı saatlik aralıkla alınır ve güncel tutulur. Güneş penceresi açıkken ve buradaki GÜNCELLE ile yenilenir.
settings-tle-fetch-track-listing = Bu listeyi al ve izle
settings-tle-remove-subscription = Bu aboneliği kaldır
settings-tle-orbits = Yörüngeler
settings-tle-which-satellites-listing-get-orbit = Bu listedeki hangi uyduların bir yörünge halkası ve etiketi alacağı. Bütün bir grup “küratörlü” ister: aynı anda doksan halka okunmaz, hiç olmaması ise doksan adsız nokta bırakır.
settings-tle-nothing-listing-sdroxide-s-curated = Bu listedeki hiçbir şey sdroxide'ın küratörlü listesinde değil — o liste on amatör uydudur, bu yüzden bu tam olarak “hiçbiri” gibi davranırdı.
settings-tle-all-satellites = bütün uydular
settings-tle-catalogue-numbers-keep-comma-separated = Tutulacak katalog numaraları, virgülle ayrılmış. Boş bırakılırsa listenin taşıdığı her şey izlenir.
settings-tle-not-fetched-yet = henüz alınmadı
settings-tle-subscription = + Abonelik
settings-tle-update-now = {" "}ŞİMDİ GÜNCELLE{" "}
settings-tle-ask-radio-engine-fetch-every = Radyo motorundan etkin her aboneliği şimdi almasını isteyin
settings-tle-celestrak-groups = CelesTrak grupları
settings-tle-pasted-element-sets = Yapıştırılmış eleman setleri
settings-tle-one-off-these-do-not = Tek seferlik içindir. Bunlar kendi kendilerini güncellemez ve SGP4, bir eleman setinin epoch'undan bir hafta sonra yayılmayı durdurur — tutmak istediğiniz her şey için bunun yerine abone olun.
settings-tle-track-one = Bunu izle
settings-tle-show-two-element-lines = İki eleman satırını göster
settings-tle-remove = Kaldır
settings-tle-paste-two-three-line-element = Buraya iki ya da üç satırlık eleman setleri yapıştırın
settings-tle-add-pasted = + Yapıştırılanı ekle
settings-tle-nothing-paste-box-looked-like = Yapıştırma kutusundaki hiçbir şey bir eleman setine benzemiyordu.
settings-tle-clear-box = Kutuyu temizle
settings-tle-frequencies = Frekanslar
settings-tle-shown-under-pass-table-solar = Güneş penceresindeki geçiş tablosunun altında gösterilir. Buradaki bir girdi, o katalog numarası için yerleşik olanın yerini tamamen alır; bu yüzden geri kalanı bırakmak istemiyorsanız bir kopyasından başlayın.
settings-tle-remove-satellite-s-entry = Bu uydunun girdisini kaldır
settings-tle-link = BAĞLANTI
settings-tle-downlink = AŞAĞI BAĞLANTI
settings-tle-uplink = YUKARI BAĞLANTI
settings-tle-mode = MOD
settings-tle-note = NOT
settings-tle-fm-repeater = FM tekrarlayıcı
settings-tle-fm = FM
settings-tle-ctcss-67-0-hz = CTCSS 67.0 Hz
settings-tle-link-2 = + Bağlantı
settings-tle-copy-built = Yerleşiği kopyala
settings-tle-frequency-either-one-number-145 = Bir frekans ya da tek bir sayıdır (145.800) ya da 145.950-145.970 olarak yazılmış bir transponder bant geçişidir. Bir işaret için yönü boş bırakın.
settings-tle-norad = NORAD
settings-tle-satellite = + Uydu
settings-tle-frequency-entry-needs-satellite-s = Bir frekans girişi, uydunun NORAD katalog numarasını gerektirir.

## Ayarlar penceresi, Wavelog (src/app/settings/wavelog.rs)

settings-wavelog-address = Adres
settings-wavelog-api-key = API anahtarı
settings-wavelog-your-wavelog-s-front-page = Wavelog'unuzun ön sayfa adresi — https://log.example.org ya da http://192.168.1.10/wavelog — ve API sayfasından bir anahtar. Yükleme okuma/yazma anahtarı gerektirir.
settings-wavelog-plain-http-address-internet-api = İnternetteki bir adrese düz http://: API anahtarı şifrelenmeden gider. Kendi ağınızın içinde sorun değildir; internet üzerinden https:// kullanın.
settings-wavelog-trusted-cert = Güvenilen sertifika
settings-wavelog-forget-certificate = Bu sertifikayı unut
settings-wavelog-go-back-ordinary-checking-against = Genel sertifika yetkililerine karşı olağan denetime geri dönün — artık düzgün bir sertifikası olan bir sunucu için.
settings-wavelog-connection-check-runs-machine-radio = Bağlantı denetimi, radyonun bağlı olduğu makinede çalışır — onu test etmek ve istasyon konumlarını listelemek için oradaki ayarları kullanın.
settings-wavelog-server-presented-certificate-sha-256 = Sunucu bu sertifikayı sundu (SHA-256):
settings-wavelog-compare-your-server-s-e = Sunucunuzunkiyle karşılaştırın — örneğin `openssl x509 -noout -fingerprint -sha256 -in cert.pem` — ve yalnızca eşleşirse güvenin. Bundan sonra bu tam sertifika kabul edilir ve diğer herhangi biri reddedilir.
settings-wavelog-trust-certificate = Bu sertifikaya güven
settings-wavelog-station-location = İstasyon konumu
settings-wavelog-none = — yok —
settings-wavelog-press-test-above-list-station = Wavelog'unuzda tanımlı istasyon konumlarını listelemek için yukarıdaki Test'e basın.
settings-wavelog-default = Varsayılan
settings-wavelog-where-qsos-go-when-operating = İşletim profili bir konum belirtmediğinde QSO'ların gittiği yer.
settings-wavelog-profile = Bu profil
settings-wavelog-use-default = Varsayılanı kullan
settings-wavelog-not-set = ayarlanmadı
settings-wavelog-no-station-location-chosen-uploads = Hiçbir istasyon konumu seçilmedi — biri seçilene kadar yüklemeler başarısız olur.
settings-wavelog-qsos-not-wavelog = Wavelog'da olmayan QSO'lar
settings-wavelog-every-qso-logbook-wavelog = Günlükteki her QSO Wavelog'da.
settings-wavelog-everything-logbook-wavelog-has-not = Wavelog'un onaylamadığı günlükteki her şey — otomatik yüklemeden önceki temaslar dahil. Zaten sahip olduklar yinelenen olarak geri gelir ve yalnızca gönderildi olarak işaretlenir.
settings-wavelog-set-address-key-station-location = Önce adresi, anahtarı ve bir istasyon konumunu ayarlayın.
settings-wavelog-yes-send-them = Evet, gönder
settings-wavelog-cancel = İptal
settings-wavelog-download-into-logbook = Bu günlüğe indir
settings-wavelog-downloading-runs-where-radio-attached = İndirme, radyonun bağlı olduğu yerde, o makinenin günlüğüne çalışır — orada kurun.
settings-wavelog-download-qsos-start-every = QSO'ları başlangıçta ve şu aralıkla indir
settings-wavelog-downloaded-qsos-join-logbook-so = İndirilen QSO'lar günlüğe katılır; böylece FT8 ve spot listeleri, diğer programlardan ve telsizlerden neyle çalıştığınızı bilir. Her indirme yalnızca yeni olanı alır.
settings-wavelog-press-test-above-choose-which = Hangi istasyon konumlarının indirileceğini seçmek için yukarıdaki Test'e basın.
settings-wavelog-sync-now = Şimdi eşitle
settings-wavelog-fetch-what-wavelog-has-added = Son indirmeden bu yana Wavelog'un eklediklerini alın. Önce yukarıdaki ayarları uygular.
settings-wavelog-full-resync = Tam yeniden eşitleme…
settings-wavelog-download-ticked-locations-whole-compare = İşaretli konumları bütünüyle indirin ve günlükle karşılaştırın: Wavelog'da düzenlenen ya da silinen QSO'lar ya da artık işaretli olmayan bir konumdan gelenler hizaya getirilir. Herhangi bir şey olmadan önce değişiklikleri görürsünüz.
settings-wavelog-full-resync-changes-make = Tam yeniden eşitleme — yapılacak değişiklikler
settings-wavelog-qsos-logged-here-imported-file = Burada kaydedilen ya da bir dosyadan içe aktarılan QSO'lar hiçbir zaman değiştirilmez ya da kaldırılmaz — yalnızca Wavelog'dan indirilenler.
settings-wavelog-apply = Uygula
settings-wavelog-discard = At
settings-wavelog-live-radio-status = Canlı radyo durumu
settings-wavelog-send-frequency-mode-wavelog-they = Değiştikçe frekansı ve modu Wavelog'a gönder
settings-wavelog-radio-name = Radyo adı
settings-wavelog-pick-radio-wavelog-s-live = Wavelog'un canlı kaydında bu radyoyu seçin; formu skalayı izler. Okuma/yazma anahtarı gerektirir. Yalnızca istasyonun ana radyosundan.

## Ayarlar penceresi, resim yükleme: değerli iletiler

settings-picup-server-fingerprint = Sunucu sertifikası SHA-256: { $fingerprint }

## Ayarlar penceresi, Profiller: değerli iletiler

settings-profiles-apply-hint = İstasyonu yeniden “{ $name }” profiline geri koyun: skalalar, VFO'lar, mod, filtreler, kazançlar, sürüş, antenler, kimlik ve bant yığınları.
settings-profiles-delete = “{ $name }” sil

## Ayarlar penceresi, T/R anahtarı: değerli iletiler

settings-relay-not-usable = Henüz kullanılamaz: { $why }
settings-relay-receiving = alıyor
settings-relay-rx-tx-words = { $rx } / TX { $tx }

## Ayarlar penceresi, Sunucular: değerli iletiler

settings-servers-n-clients = { $n ->
        [one] { $n } istemci
       *[other] { $n } istemci
    }
settings-servers-listening = ● { $addr } üzerinde dinleniyor — { $clients }
settings-servers-failed = Başarısız: { $error }
settings-servers-rotator-at = ● Bağlı — anten { $az }° az, { $el }° el
settings-servers-tci-client-error = TCI İstemcisi: { $error }

## Ayarlar penceresi, TLE: değerli iletiler

settings-tle-subscription-age = { $count } uydu · { $age } eski
settings-tle-subscribed = { $name } aboneliği başlatıldı. Almak için GÜNCELLE'ye basın.
settings-tle-norad-too-stale = NORAD { $norad } · { $age } eski — yayılım için fazla eski
settings-tle-norad-age = NORAD { $norad } · { $age } eski
settings-tle-norad-n = NORAD { $norad }
settings-tle-added = { $n } eleman { $n ->
        [one] seti
       *[other] seti
    } eklendi.
settings-tle-added-refreshed = { $added } eklendi ve { $r } eleman { $r ->
        [one] seti
       *[other] seti
    } yenilendi.
settings-tle-links = { $n } { $n ->
        [one] bağlantı
       *[other] bağlantı
    }
settings-tle-replace-links = Bu bağlantıları { $name } için yerleşik olanlarla değiştir

## Ayarlar penceresi, Wavelog: değerli iletiler

settings-wavelog-call-mismatch = “{ $location }” konumu { $station_call }, ama { $call } olarak kaydediyorsunuz — Wavelog bu QSO'ları reddedecek.
settings-wavelog-grid-mismatch = “{ $location }” konumu { $station_grid } içinde, ama lokatörünüz { $grid } — Wavelog bu QSO'ları reddedecek.
settings-wavelog-location-n = konum #{ $id }
settings-wavelog-saved-with-callsign = Ayarlar → Profiller içinde çağrı işaretiyle ({ $call }) kaydedildi; böylece taşınabilir bir profil taşınabilir bir konuma kaydedebilir.
settings-wavelog-owed = Otomatik yükleme açıldığından beri kaydedilen ve hâlâ gönderilmeyi bekleyen { $n } { $n ->
        [one] QSO
       *[other] QSO
    } — her 5 dakikada bir yeniden denenir.
settings-wavelog-send-unsent = Wavelog'da henüz olmayan { $n } { $n ->
        [one] QSO
       *[other] QSO
    } gönderilsin mi…
settings-wavelog-send-confirm = { $n } QSO { $location } konumuna gönderilsin mi? Ayarlar önce uygulanır.
settings-wavelog-plan-add = • Günlüğe eklenecek Wavelog'daki { $n } QSO
settings-wavelog-plan-update = • Güncellenecek, Wavelog'da değişen { $n } indirilmiş QSO
settings-wavelog-plan-remove = • Kaldırılacak, artık Wavelog'da olmayan (ya da işaretsiz bir konumdan gelen) { $n } indirilmiş QSO
settings-wavelog-plan-mark = • Wavelog'da olduğu işaretlenecek { $n } kendi QSO'nuz

## Ayarlar penceresi, Radyo sekmesi (src/app/settings/radio.rs)

settings-radio-antenna = Anten
settings-radio-which-socket-back-radio-using =
    Radyonun arkadaki hangi soketi kullandığı — kendi ANT komutu; ön paneldeki ANT düğmesiyle aynı ayar.

    Hemen uygulanır ve burada tutulan bir kopya değil, radyonun kendi ayarıdır: radyonun üzerinde olduğu soket, oturum açıldığında geri okunur.

    Seçim bant başına hatırlanır. 2 m'de anteni, 40'ta teli seçin; skala o banda her geçtiğinde ve sdroxide her başladığında ikisi de geri gelir. Burada saklanan bir hafıza kanalı, soketini de taşır.
settings-radio-receive-antenna = Alma anteni
settings-radio-radio-s-separate-receiving-antenna =
    Radyonun ayrı alma anteni — arkadaki RX ANT — alma yoluna ya da alma yolundan çıkarılır. Kendi ayarı; radyonun RX ANT düğmesiyle aynı.

    Ana soketteki anten, her iki durumda da yayında kalır: bu, alma için ek bir girişdir, bir döngü ya da bir beverage; bir soket seçimi değildir.

    Yukarıdaki antenin aksine bu burada *hatırlanmaz*. Radyo bunu bant başına kendisi tutar; bu yüzden sdroxide her bant değişiminden sonra bunu geri okur ve radyonun ne dediğini gösterir; bu kutuyu işaretlemek onu hareket ettiren tek şeydir.
settings-radio-receive-path = Alma yolunda
settings-radio-radio-power = Radyo gücü
settings-radio-switch-radio-itself-off-back =
    Radyonun kendisini kontrol bağlantısı üzerinden kapatıp yeniden açın. Programdaki tek gerçek açma/kapatma budur — sdroxide'ın kendi LINK anahtarı değildir; o yalnızca sdroxide'ın tarafını kapatır ve radyo çalışmaya devam eder.

    Yeniden açma anahtarının bir şeye ulaşması için, radyonun kontrol ucu kapalıyken uyanık kalmalıdır. Ağ üzerinden bunu Network Control yapar. Seri kabloyla, bu Icom'un CI-V bağlantı noktasıdır; fişten çekilmemiş ama ön panelden kapatılmış bir cihazda şebekeden beslenmeye devam eder. sdroxide, açma komutundan önce Icom'un kendi belgelerinin istediği uyandırma dizisini gönderir.

    Kapatmak sesi ve metreleri sonlandırır — arkalarında radyo yoktur — ve sdroxide, yeniden açma anahtarının gidecek bir yeri olsun diye kontrol bağlantısını açık tutar.
settings-radio-on = Açık
settings-radio-off = Kapalı
settings-radio-waiting-configuration-machine-radio-attached = Radyonun bağlı olduğu makinenin yapılandırması bekleniyor.
settings-radio-sound-format = Ses biçimi
settings-radio-invert-spectrum = Spektrumu ters çevir
settings-radio-swap-i-q = I/Q'yu değiştir
settings-radio-mirror-panadapter-about-tuned-frequency =
    Ses kartında I ve Q'yu ters taşıyan bir telsiz için, panadaptörü ayarlı frekansın etrafında aynalar. Bunu ele veren şey, skalanın yanlış tarafında olan ama inandırıcı görünen sinyallerle dolu bir şelaledir; SSB karşı yan bantta çıkar — ses kartındaki iki kabloyu değiştirmek de aynı şekilde düzeltir.

    Yalnızca alma: yayın, tersine çevrilecek yan bandı olmayan gerçek bir ses sinyali verir.
settings-radio-i-q-sample-rate = I/Q örnekleme hızı
settings-radio-how-fast-radio-s-i =
    Radyonun I/Q ses kartının ne kadar hızlı çalıştığı — ve bu yüzden panadaptörün ne kadar bant gösterdiği, çünkü kuadratür bir akış tüm örnekleme hızını kaplar: 48 kHz skalanın iki yanında ±24 kHz verir, 192 kHz ise ±96.

    Son sözü kart söyler. Buradan seçilen hızı yapamayan bir kart, yapabildiği en yakın hızda açılır ve panadaptör o kadar dar olur — bu olduğunda günlük başlangıçta bunu söyler.

    Daha hızlı bir kart daha fazla iştir: makinenin onu boşaltmak için orantılı olarak daha az zamanı vardır ve yetişemeyen bir kart örnekleri düşürür. Bu, şelale hâlâ kusursuz görünürken sesin kesilmesi olarak ortaya çıkar ve bu da günlüğe yazılır. Bunu görürseniz bir adım geri inin.
settings-radio-t-48-khz-24-khz = 48 kHz (±24 kHz)
settings-radio-t-96-khz-48-khz = 96 kHz (±48 kHz)
settings-radio-t-192-khz-96-khz = 192 kHz (±96 kHz)
settings-radio-t-384-khz-192-khz = 384 kHz (±192 kHz)
settings-radio-i-q-centre-offset = I/Q merkez kayması
settings-radio-how-far-radio-s-i =
    Radyonun I/Q çıkışının kendi skalasının ne kadar üstünde merkezlendiği; alma I.F.'si sıfırdan uzağa taşınmış bir telsiz için — bir Elecraft'ta, MENU:RX SHFT, NOR yerine 8.0'a ayarlandığında, bu skalayı karıştırıcının DC tepesinden uzaklaştırır (ve yakındaki güçlü bir SSB/AM istasyonunun AM olarak algılanmasını durdurur).

    Bir dönüştürücü değildir: radyo hâlâ gerçek frekansta gösterir ve yayın yapar, buradaki hiçbir şey ona gönderilmez. Bu yalnızca ses kartındaki örneklerin nerede durduğunu söyler; akış gelirken skalaya geri kaydırılır.

    Böyle bir menü girdisini açmadıysanız 0'da bırakın. Sinyaller kayma kadar iki katı uzaklığa düşüyorsa işaret ters demektir.
settings-radio-iq-correction = IQ düzeltme
settings-radio-cancel-mirror-image-every-signal =
    Her sinyalin ayna görüntüsünü ve şelalenin ortasındaki DC tepesini iptal eder.

    Bir telsizin I/Q çıkışı, kazançları hiçbir zaman tam eşit olmayan ve 90° tam olarak ayrık olmayan iki analog yoldur; bunun bıraktığı şey, her sinyalin ayarlı frekansa göre yansıyan bir kopyasıdır, genellikle 30-40 dB aşağıda — bir istasyon yokmuş gibi görünecek ve öyle çözülecek kadar güçlü. Ön paneldeki denge trimmer'ları olan telsizler tam olarak bunu ayarlar; burada alınan gürültüden ölçülür ve bir ayar gerektirmez.

    Açık bırakın. Taşıyıcı tam üzerinde ayarlanmış AM dinliyorsanız kapatın — taşıyıcı DC'dir ve tepeyle birlikte gider — ya da bir sinyalin gerçek olup olmadığını, onun kalıp kalmadığına bakarak denetlemek için kapatın.
settings-radio-cancel-mirror-images = Ayna görüntülerini iptal et
settings-radio-dc-notch = DC çentiği
settings-radio-widen-hole-taken-out-middle =
    Genişliği ortasından çıkarılan deliği genişletin; merkez tepesi, altındaki kaymadan daha geniş olan bir telsiz için.

    0, sıradan engelleyiciyi bırakır — birkaç onlarca hertz genişliğindedir ve kaymayı başka hiçbir şeye dokunmadan kaldırır. Onu yukarı çevirirseniz genişliğin alt ucu da onunla gider: birinci dereceden bir yüksek geçiren, buradaki değerde 3 dB aşağı ve altında daha da fazla.

    Telsizin I/Q'sunun merkezlendiği yere merkezlenir; bu, yukarıdaki kayma onu hareket ettirmedikçe skaladır — yani oraya ayarlanan her şey de gider. 600 Hz'deki bir CW notası, 600 Hz'lik bir ayarın içindedir.
settings-radio-panadapter-bw = Panadaptör BW
settings-radio-cat-family = CAT ailesi
settings-radio-rigctld-address = rigctld adresi
settings-radio-host-port-running-hamlib-rigctld =
    Çalışan bir Hamlib rigctld'nin ana makine:bağlantı noktası — 127.0.0.1:4532, bu makinedeki varsayılanıdır.

    Birini örneğin `rigctld -m 2028 -r /dev/ttyUSB0 -s 38400` ile başlatın; burada -m, radyonuzun Hamlib model numarasıdır (`rigctl -l` listeler).

    Bu, her şeyi kapsayan genel seçenektir: frekans, mod, PTT, güç, S-metre ve SWR'ye ulaşır, başka hiçbir şeye değil. Yukarıdaki yerel ailelerden biri radyonuza uyuyorsa daha fazlasını yapar — telsizin kendi metin tamponundan anahtarlama, alma filtresi ve model başına metre ölçekleri.
settings-radio-flrig-address = flrig adresi
settings-radio-host-port-running-flrig-127 =
    Çalışan bir flrig'in ana makine:bağlantı noktası — 127.0.0.1:12345, bu makinedeki varsayılanıdır. flrig çalışırken her zaman XML-RPC sunar; bağlantı noktası Config → Setup → Server altındadır.

    Hamlib seçeneği gibi, radyoyu değil bir servisi sürer; ama flrig'in kendi model başına sürücüsü üzerinden — birçok telsizde güç ve filtre işlemi ikisinden daha sadıktır. Frekans, mod, PTT, yayın gücü (tam watt olarak), alma bant genişliği, S-metre, SWR ve çıkış gücüne ulaşır. CW, flrig'in kendi cwio bağlantı noktasından anahtarlanır; bu, flrig'in içinde ayarlanmalıdır.
settings-radio-serial-port = Seri bağlantı noktası
settings-radio-select = — seçin —
settings-radio-baud = Baud
settings-radio-data-bits = Veri bitleri
settings-radio-parity = Eşlik
settings-radio-stop-bits = Durma bitleri
settings-radio-force-rts = RTS'yi zorla
settings-radio-force-dtr = DTR'yi zorla
settings-radio-ptt-method = PTT yöntemi
settings-radio-how-transmit-keyed = Yayının nasıl anahtarlandığı.
settings-radio-how-transmit-keyed-network-link = Yayının nasıl anahtarlandığı. Bir ağ bağlantısının denetim hatları yoktur, bu yüzden DTR ve RTS burada hiçbir şeyi anahtarlamaz — servisten telsizi anahtarlamasını isteyen CAT'i ya da VOX'u kullanın.
settings-radio-mode-control = Mod denetimi
settings-radio-digimode-mode = Dijital mod modu
settings-radio-what-put-radio-mode-sdroxide =
    sdroxide'ın ses kartı üzerinden modüle ettiği bir mod için telsizin neye alınacağı. “DIGI”, telsizin DATA/PKT konumunu seçer; bu, yayın sesini USB ya da ACC girişinden alır ve mikrofon yolunun konuşma işlemesini dışarıda bırakır. “USB”, düz yan bantta bırakır — SSB'yi veri girişinden modüle etmek üzere zaten ayarlanmış ya da hiç DATA konumu olmayan bir radyo için.

    Bu, her dijital modu — FT8, FT4, PSK, RTTY, SSTV ve diğerleri — ve “Sound card” olarak gönderilen CW'yi kapsar ve bunlar için Mod denetimini geçersiz kılar. Yan bandı banda göre değişen SSTV'de bu, o yan bant üzerindeki DATA konumu demektir: 40 m'nin üstünde USB-D, bunda ve altında LSB-D. SSTV-FM bunun parçası değildir — bir FM taşıyıcının seçilecek bir yan bandı yoktur.
settings-radio-cw-keying = CW anahtarlama
settings-radio-how-cw-panel-s-keyer =
    CW panelinin keyer'ı nasıl yayın yapar. “Rig keyer”, radyoyu CW'ye alır ve gönderilecek metni kendi keyer'ına verir. Panelin WPM'inden ayarlanan telsizin keyer hızını kullanır ve Yaesu'da keyer hafıza 1 üzerinden gönderir; orada ne saklıysa üzerine yazar.

    Radyoda break-in'in açık olması gerekir; sdroxide, Icom, Yaesu ve Kenwood'da her iletiyle bunu kendisi açar — break-in kapalı olan bir telsiz iletiyi alır ve hiçbir şey yayınlamaz. Sonrasında açık bırakılır ve tam break-in hiçbir zaman yarıya indirilmez.

    “Sound card”, bunun yerine anahtarlanan yan sesi ses olarak gönderir (MCW), skalanın artı perdede bir tonu — ve CW'deki bir telsiz ses kartını tamamen yok sayacağı için CW seçmek, telsizi CW'ye çevirmek yerine Dijital mod ayarını (USB, DATA ya da Radio controlled) izler. Veri konumu CAT üzerinden komut alamayan telsizler (bir Xiegu'nun U-D'si) için orada “Radio controlled” seçin ve FT8'de olduğu gibi telsizi onda kendiniz park edin.
settings-radio-poll-rate = Sorgulama hızı
settings-radio-how-often-radio-asked-what =
    Radyoya ne yaptığının sorulma sıklığı — skalası, modu ve metreleri. Bu, kontrol bağlantısının radyodan sdroxide'a giden yarısıdır: telsizin kendi VFO düğmesini çevirin ya da ön panelden modunu değiştirin; okuma, bant ve panadaptör bir sorgulama içinde izler.

    Bu, bu ucun ürettiği kontrol trafiğinin tamamıdır ve modern bir Icom'da bu trafik bedava değildir: CI-V bağlantı noktası ve ses kartı telsizin içinde tek bir USB hub'ın arkasındadır; bu yüzden burada istenen kareler, sesin alamadığı bus zamanıdır — DSP arızası gibi görünen ve öyle olmayan kesintiler olarak duyulur. Varsayılan 2 Hz, düğmenin yarım saniye gerisindedir ve sesin yolundan çekilecek kadar sessizdir. Kontrol bağlantı noktası kendi aygıtıysa (ayrı bir USB-seri adaptör, ağ rigctld'si) yükseltin; radyonun sesi kesiliyorsa düşürün.

    Kendi menüsünde CI-V Transceive açık olan bir Icom, skala için bunların hiçbirine ihtiyaç duymaz: düğme hareket ettiği anda bildirir ve sdroxide bunu gördüğü anda skala sorgulamasını durdurur — ve radyo bunu bildirmeden hareket ederse yeniden başlatır.

    Hız skalanın hızıdır. Mod, yalnızca her dördüncü sorgulamada eşlik eder; çünkü bu, bir düğmeyi izleyen değil, bir akşamda birkaç kez değişen bir ayardır.
settings-radio-send-command = Gönderme komutu
settings-radio-which-transceiver-generation-keys-rig =
    PTT yöntemi “CAT” için telsizi hangi telsiz kuşağının anahtarladığı. İkisi TX parametresinin ne anlama geldiği konusunda anlaşmazdır ve kabloda ikisini ayırt edecek hiçbir şey yoktur; bu yüzden radyonuzla eşleşeni seçin.

    “TS-2000 style (TX;)” — TS-480, TS-570, TS-870, TS-2000: sıradan gönderme, ana bantta.

    “TS-590 style (TX1;)” — TS-590S/SG, TS-890, TS-990: DATA SEND, ACC2/USB ses girişi canlıyken anahtarlar. Bu telsizlerde düz gönderme bunun yerine mikrofonu seçer ve sdroxide'ın yayınladığı sesi susturur.

    Yanlış ayarlanırsa bir TS-590 sessizlik yayınlar — bir TS-2000 ise alt bantta yayın yapar, bu tamamen başka bir banttır.
settings-radio-radio = Radyo
settings-radio-k3-k3s-kx3-kx2-k4 = K3 · K3S · KX3 · KX2 · K4
settings-radio-one-profile-covers-family-k3 =
    Tek bir profil aileyi kapsar: KX3, KX2 ve K4'ün hepsinin yanıt verdiği K3 komut kümesi. Burada seçilecek bir şey yoktur — Sürüş kaydırıcısının kaç watt kapsadığı (çıplak bir KX2 ya da KX3'te 12, bir KPA3 ya da KXPA100 ile 110) bağlantı noktası açıldığında telsizin kendi opsiyon modülü sorgusundan okunur.

    Yukarıdaki baud hızlarına dikkat edin: K3, K3S, KX3 ve KX2 38400'den hızlı gitmez ve buradaki hızın altına ayarlanmış bir telsiz hiçbir şeye yanıt vermez.
settings-radio-transmit-input = Yayın girişi
settings-radio-where-radio-takes-transmit-audio =
    Telsizin yayın sesini nereden aldığı — telsizin TI komutu; ön panelde menü 32 “TX IN”. Bağlantı noktası açıldığında etkinleştirilir.

    “USB audio”, bu arayüzü çalıştıran ayardır: FDM-DUO, sdroxide'ın USB ses kartına koyduğu şeyi yayınlar. “Microphone”da bırakılmış bir telsiz bunun yerine odayı yayınlar ve ekranda bunu söyleyen hiçbir şey yoktur.

    “Auto”, telsizin seçmesine izin verir — mikrofondaki bir PTT basışı için mikrofon, bir CAT ya da RTS tuşlamasında USB bağlantı noktası.

    “Leave as set on the radio”, hiç TI göndermez; bunu ön panelden ayarlayan bir operatör içindir.
settings-radio-which-two-sockets-back-receiver =
    Alıcının arkadaki iki soketten hangisini dinlediği — telsizin AN komutu; ön panelde menü 31 “ANTENNAS” ve ekranındaki “ANT 1 2” göstergesi.

    “RTX”, yayın da yapan M tipi soket üzerinde tek bir antenin iki işi birden yapmasıdır. “RX only”, almayı ikinci sokete taşır ve yayını RTX'te bırakır — bir alma anteni, bir döngü ya da bir beverage, huzmenin hâlâ verici üzerinde olduğu.

    Hemen uygulanır ve bağlantı noktası açıldığında telsizden geri okunur: burada tutulan bir kopya değil, telsizin kendi ayarıdır.
settings-radio-one-profile-covers-both-there =
    Tek bir profil ikisini de kapsar. Burada seçilecek bir şey yoktur — telsiz, bağlantı noktası açıldığında kendini adlandırır.

    Bu yalnızca CAT yarısıdır: skalayı, modu, PTT'yi, S-metre'yi, SWR'yi ve yayın gücünü telsizin CAT USB bağlantı noktası üzerinden sürer ve sesi USB Audio bağlantı noktasından alır. FDM-DUO'nun üçüncü USB bağlantı noktası geniş bant I/Q taşır; bu arayüz onu okumaz — bunun için yukarıdaki ELAD arayüzünü seçin.

    Baud hızına dikkat edin: telsizdeki menü 70 “CAT BAUD” eşleşmelidir ve fabrikadan 38400 gelir.

    CW, telsizin kendi tuşu ya da paletiyle anahtarlanır. FDM-DUO metin kabul eden bir komuta sahip değildir, bu yüzden CW paneli onu CAT üzerinden anahtarlayamaz.
settings-radio-one-profile-covers-range-qrp =
    Tek bir profil bu aralığı kapsar: QRP Labs'ın kendi komut kümesi; Kenwood TS-480'in bir alt kümesi, ama epey eklentili. Burada seçilecek bir şey yoktur — telsiz, bağlantı noktası açıldığında kendini ve yazılım sürümünü adlandırır ve sürüm, SWR koruma okumasının hiç istenip istenmeyeceğine karar verir.

    Kenwood profili değildir: bir QMX'te PC komutu güç denetimi değil güç METER'dır; bu yüzden Kenwood gibi sürülen bir telsizin metre okuması, bir ayarmış gibi yazılır ve MD8 — bir Kenwood modu — burada SWR Tune'dur.

    Yayın gücü telsizde ayarlanır. Bunun için bir CAT komutu yoktur, bu yüzden Sürüş kaydırıcısı yalnızca ses kartına giden seviyeye etki eder.

    Alma filtresi de telsizin kendisinindir: modunun gerektirdiği genişliği bildirir (Digi'de 3,2 kHz, CW'de 300 Hz) ve onu değiştirecek hiçbir şey sunmaz.
settings-radio-baud-rate-ignored-port-usb = Baud hızı yok sayılır — bağlantı noktası USB
settings-radio-qmx-serves-own-virtual-com =
    Bir QMX, kendi sanal COM bağlantı noktalarını USB üzerinden sunar; bu yüzden yukarıda ayarlanan hızın iki uçta da bir etkisi yoktur. En fazla üçünü sunar; telsiz hiçbir şeye yanıt vermiyorsa en olası neden yanlış olanıdır.

    Bağlantı noktası hakkında bir şey gerçekten önemlidir: ona gönderilen bir satır başı, telsizi oturumun geri kalanı için terminal moduna alır. sdroxide hiçbir zaman göndermez — ama aynı bağlantı noktasında açık bırakılmış bir terminal programı gönderir ve CAT, bunun olduğu anda çalışmayı durdurur.
settings-radio-i-q-mode-switched-radio = I/Q modu telsizde açılır
settings-radio-radio-s-sound-card-carries =
    Telsizin ses kartı, ya demodüle edilmiş sesi ya da ADC'sinin gördüğü ham I/Q'yu taşır; sdroxide, bağlantı noktası açıldığında yukarıdaki Ses biçiminin istediğini etkinleştirir (telsizin Q9 komutu, “IQ mode” menü girdisi).

    Yukarıdaki merkez kayması ve örnekleme hızı, bir QMX'in ihtiyacı olan değerlere ayarlanmıştır: alıcısı 12 kHz A.F.'li bir süperheterodindir, bu yüzden sentezleyici skalanın 12 kHz altında durur ve kartındaki her şey bandın ortasının 12 kHz üstündedir. Kart 48 kHz'de çalışır, bu da panadaptörü 48 kHz genişliğinde yapar.

    CW'de telsiz bu A.F.'yi bir miktar daha, yaklaşık 700 Hz kaydırır; tek bir rakam bunu izleyemez — telsiz CW'deyken panadaptörü çalıştırıyorsanız elle ekleyin. Telsizin genellikle I/Q ön ucu olarak durduğu Digi'de eklenecek bir şey yoktur.

    QRP Labs, I/Q modunun demodüle ses bekleyen WSJT-X ve diğer programlar için uygun olmadığını not eder; burada demodülasyon bu tarafta yapıldığı için uygundur.
settings-radio-rs-hfiq-5-w-hf = RS-HFIQ (5 W HF telsiz)
settings-radio-hobbypcb-s-rs-hfiq-there =
    HobbyPCB'nin RS-HFIQ'su. Burada seçilecek bir şey yoktur: profil, telsizin komut kümesinin tamamıdır ve yazılım sürümü, bağlantı noktası açıldığında günlüğe yazılır.

    Kontrol bağlantısı iki şey taşır, daha fazlasını değil — osilatörün nereye konacağı ve yayın yapılıp yapılmayacağı. Mod, güç, filtre, susturma ya da metre komutu yoktur, çünkü telsizde bunların hiçbiri yoktur. Geri kalan her şey sdroxide'ındır: ses kartından çıkan şey skalaya merkezlenmiş karmaşık tabandır, bu yüzden mod, filtre ve modülasyonun hepsi bu tarafta olur.

    Frekans komutu 3–30 MHz'i kapsar. Bunun dışında bir şey isterseniz telsiz reddeder ve olduğu yerde kalır; günlük bunu söyler.

    CW, yayın zinciri üzerinden ses olarak anahtarlanır — bu telsizin metin alan bir keyer'ı yoktur. (Dahili bir CW üreteci vardır; sdroxide onu hiç kullanmaz, yazılımın kendi tavsiyesi üzerine.)
settings-radio-sound-format-baud-ptt-are = Ses biçimi, baud ve PTT telsizinkidir, ayar değil
settings-radio-all-three-have-been-filled =
    Üçü de doldurulmuştur ve hiçbiri bir tercih değildir:

    • Ses biçimi I/Q (stereo)'dur — kart karmaşık taban bandı taşır ve başka hiçbir şeyi taşımaz.
    • 57600 baud, 8N1 — yazılımın tek hızı, değiştirecek bir menü yoktur. Başka herhangi bir hız sessiz bir bağlantıdır, bu yüzden bağlantı noktası açıldığında sabitlenir.
    • CAT üzerinden PTT — *X komutu, arayüzün sahip olduğu tek yayın anahtarıdır.

    Merkez kayması sıfıra ayarlanmıştır: sentezleyici bir kuadratür dedektörüne dört kez skalada çalışır, bu yüzden bandın ortası skaladır. Yukarıdaki örnekleme hızını ses kartınızın gerçekte çalıştığı hıza ayarlayın; panadaptörü olduğu kadar geniş yapan budur.
settings-radio-dl2man-pe1nnz-s-pocket-qrp =
    DL2MAN/PE1NNZ'nin cep QRP telsizi ve ondan büyüdüğü açık uSDX yazılımı. Bir Kenwood TS-480'i taklit eder, ama alt küme incedir: skala, mod, PTT ve RIT/XIT temizleme yanıtladığı her şeydir. S-metre yok, SWR yok, güç denetimi yok, VFO B yok, split yok ve CAT keyer yok; bu yüzden Sürüş kaydırıcısı ve metreler CAT üzerinden hiçbir şey yapmaz — ses seviyesi tek yayın denetimidir.

    CW, yan bant üzerinden ses olarak (MCW) ya da telsizdeki tuş ya da paletle anahtarlanır; metin alan bir komut yoktur.
settings-radio-audio = Ses
settings-radio-tr-usdx-has-no-sound =
    Bir (tr)uSDX'in kendi ses kartı yoktur ve duyulmasının iki yolu vardır; hangisinin doğru olduğu telsizle değil, shack'inizle ilgili bir gerçektir.

    “One cable”, sesi CAT seri bağlantısının kendisinin içinde alır — telsizin kendi 8 bitlik akışı, denetimle aynı USB kablosu üzerinden, takılacak başka bir şey olmadan. Yakalama, yazılımdadır: akışı çalışırken bir CAT komutu alamaz (akışa gönderilen bir komut onu duraklatmaz, öldürür), bu yüzden bu mod hiç sorgulamaz. Ayarlama ve mod değişiklikleri, siz yaptıkça gönderilir; her biri akışı bir an duraklatıp yeniden başlatır, bu yüzden her değişiklikte kısa bir boşluk bekleyin; siz yalnızca dinlerken hiçbir şey gönderilmez. Telsizin kendi skalası ve modu da izlenmez — onları izlemek için okuma yoktur — bu yüzden buradan sürün.

    “USB sound card”, sesi telsizin 3,5 mm hoparlör/mikrofon jakına bağlı harici bir ses kartından alır ve diğer her CAT telsizi gibi davranır: skala sorgulaması çalışır, bu yüzden telsizin kendi düğmesi ve modu izlenir. Aşağıdaki Radyo sesi altından kartı seçin.

    İki mod da bağlantı noktasını DTR yüksekken açar: seri adaptörün DTR'si telsizin reset'ine bağlıdır, bu yüzden bağlantı noktasını açmak onu yeniden başlatır ve ayağa kalkarken ilk bir ya da iki saniye sessizdir. DTR asla anahtarlamak için kullanılmaz.
settings-radio-radio-model = Radyo modeli
settings-radio-which-icom-two-things-ci =
    Hangi Icom — CI-V'nin hepsinde aynı biçimde yapmadığı iki şey için.

    Aşağıdaki telsiz adresi bundan doldurulur: her model farklı bir adresle gelir ve yanlış adrese gönderilen bir çerçeve sessizce yok sayılır — hiçbir yerde nedenini söyleyen bir hata olmadan yanıt vermeyen bir telsiz.

    Ayrıca sdroxide'ın DATA modunu seçip seçemeyeceğine karar verir. CI-V'de USB ve USB-DATA aynı mod baytıdır ve ayrı bir komut onları ayırt eder; bu olmadan dijital bir mod yayını, telsizin konuşma işlemesi ve SSB filtresi yolda olan mikrofon girişinden çıkar.

    “Other”, adresi size bırakır ve hiçbir DATA modu komutu göndermez.
settings-radio-radio-id-hex = Radyo kimliği (hex)
settings-radio-show-radio-s-spectrum-scope = Telsizin spektrum ekranını göster
settings-radio-scope-span = Ekran aralığı
settings-radio-how-wide-sweep-radio-keeps = Ne kadar geniş taranacağı. Telsiz, kendi ekranında en son seçilen aralığı tutar — genellikle birkaç kHz. Burada bir aralık ayarlamak, ekranı ortadan-merkez moduna da alır; böylece skalayı izler. Telsizin kendi ekranını da değiştirir; “As set on the radio” onu olduğu gibi bırakır.
settings-radio-press-apply-reconnect-switch-without = Yeniden başlatmadan geçmek için “Apply / reconnect”e basın.
settings-radio-radio-audio-sound-card = Radyo sesi (ses kartı)
settings-radio-waiting-sound-cards-machine-radio = Radyonun takılı olduğu makinedeki ses kartları bekleniyor.
settings-radio-radio-rx = Radyodan (RX)
settings-radio-radio-tx = Radyoya (TX)
settings-radio-apply-reconnect = Uygula / yeniden bağlan
settings-radio-reopen-cat-rig-these-sound = CAT telsizini bu ses kartlarıyla yeniden aç — yeniden başlatma yok
settings-radio-reconnects-radio-without-restarting = Radyoyu yeniden başlatmadan yeniden bağlar.
settings-radio-receive-radio-pc = Alma (radyo → PC)
settings-radio-sound-card-radio-s-own = Radyonun kendi sesinin geldiği ses kartı — bilgisayarın line ya da mikrofon girişine giden kulaklık soketi. Panadaptördeki ve çözücülerdeki her şey buraya gelir.
settings-radio-transmit-pc-radio-mic = Yayın (PC → radyo mikrofonu)
settings-radio-sound-card-carries-audio-radio = Radyonun yayınlaması gereken sesi taşıyan ses kartı — bilgisayarın çıkışı, radyonun mikrofon soketine. Radyo, buraya ses geldiği anda VOX ile kendini anahtarlar; bu yüzden radyo yayın yapılmadığında bu aygıt sessizdir.
settings-radio-radio-has-no-control-cable = Bu radyonun kontrol kablosu yoktur — VOX'tan kendini anahtarlar, bu yüzden komut verilecek bir PTT yoktur. Yayın, radyonun mikrofonuna giden sestir.
settings-radio-reopen-radio-these-sound-cards = Radyoyu bu ses kartlarıyla yeniden aç — yeniden başlatma yok
settings-radio-devices = Aygıtlar
settings-radio-discover = Keşfet
settings-radio-no-devices-press-discover = aygıt yok — Keşfet'e basın
settings-radio-manual-ip = Elle IP
settings-radio-optional-e-g-192-168 = isteğe bağlı, örn. 192.168.1.50
settings-radio-sample-rate = Örnekleme hızı
settings-radio-receiver-ddc = Alıcı (DDC)
settings-radio-protocol-2-board-carries-several = Bir Protocol 2 kartı, tek bir bağlantı üzerinde birden çok bağımsız ayarlanabilir alıcı (DDC) taşır — bu radyoyu DDC1'de, aynı adresteki başka bir radyoyu DDC2'de çalıştırın. Verici DDC1 radyosuna aittir. Protocol 1 kartlarında yalnızca DDC1 vardır.
settings-radio-lna-gain = LNA kazancı
settings-radio-front-end-gain-hermes-lite = Bir Hermes-Lite 2'nin ön uç kazancı. Hemen etkinleşir — yeniden bağlanmadan — ve radyonun başladığı seviye olarak hatırlanır. Çok yüksek olursa ADC kırpar ve tüm bant bozuk görünür; çok düşük olursa alıcı sağır olur.
settings-radio-overload-protection = Aşırı yük koruması
settings-radio-back-lna-gain-off-itself =
    Kart ADC'sinin taştığını bildirdiği sürece LNA kazancını kendiliğinden geri çeker ve taşma durunca yeniden yükseltir.

    Bir Hermes-Lite 2, 0-38 MHz'in tamamını, önünde karıştırıcı ve ön seçici olmadan tek bir 12 bitlik dönüştürücüye örnekler; bu yüzden bir bant uzaktaki bir yayın istasyonu onu taşmaya sokabilir, siz baktığınız bant ise hiçbir sorun göstermezken — gürültü tabanı yükselir, her şey intermodülasyon yapar ve çözücüler durur. Kart bunu bilir ve bu, buna tepki veren şeydir.

    Varsayılan olarak kapalı: sizin ayarladığınız bir denetimi hareket ettirir. Yayın yaparken hiçbir şey olmaz.
settings-radio-wind-gain-back-when-adc = ADC taştığında kazancı geri sar
settings-radio-applies-apply-reconnect-main-window = Uygula / yeniden bağlan ile uygulanır. Ana pencerenin Kazanç rayı, kartın gerçekte çalıştırdığı kazancı okur; bu yüzden döngüyü izler ve ne yaptığını izleyebilirsiniz; yukarıdaki kaydırıcı radyonun başladığı seviyedir, döngü onu yeniden yazmaz.
settings-radio-step = {"  "}Adım
settings-radio-how-far-gain-moves-each = Kazancın her seferinde dB cinsinden ne kadar hareket ettiği. Bir kartın kendi kazanç kaydedicisinin adımı birdir.
settings-radio-attack-decay = {"  "}Atak / bırakma
settings-radio-how-often-gain-may-come =
    Dönüştürücü taşarken kazancın ne sıklıkla düşebileceği ve taşma durduktan sonra ne sıklıkla yeniden yükselebileceği.

    İkisi bilerek yüz kat uzaktadır. Hemen geri çekilin: taşmanın her milisaniyesi, intermodülasyonla dolu bir alıcı demektir. Yavaş dönün: onu tetikleyen her ne ise — bir komşunun anahtarlaması, akşam üzeri açılan bir yayıncı — genellikle gitmemiştir ve geri çekildiği kadar hızlı kurtulan bir döngü, akşamı eşiğin üstünde salınarak geçirirdi. Desibel başına 100 ms ve 10 s, hem PowerSDR'nin Auto S-Att'ının hem de N1GP'nin HermesIntf'inin kullandığı değerlerdir.
settings-radio-range = {"  "}Aralık
settings-radio-lowest-highest-gain-loop-may = Döngünün kullanabileceği en düşük ve en yüksek kazanç, dB cinsinden. Tavan, alıcınızın ne kadar hassas olması gerektiğine karar vermesini engeller; taban ise bir noktanın altındaki taşmanın başkasının sorunu olduğunu ve cevabın yirmi desibel daha değil bir filtre olduğunu söylediğiniz yerdir.
settings-radio-filter-board = Filtre kartı
settings-radio-accessory-board-hermes-lite-2 = Hermes-Lite 2'nin J16 başlığındaki aksesuar kartı (ya da başka herhangi bir openHPSDR kartının açık kollektörlü çıkışları — Protocol 2 dahil). “N2ADR”, bant başına bir röle seçer; “Alex / Hermes band code”, bant numarasını 1-4 çıkışlarına koyar; bu, bir ANAN'ın Alex kartının, bir Zeus SDR'nin, bir HiQSDR'nin ve Quisk'in beklediği şeydir. “Custom”, yedi çıkışı bant bant, alma ve yayında kendiniz belirttiğiniz aşağıdaki bir tabloyu açar — bir anten anahtarı, bir amplifikatörün bant çözücüsü ya da iki ön ayarın da uymadığı herhangi bir şey için; her iki ön ayar da başlangıç noktası olarak buraya dökülebilir. Gerçekten takılı bir şey yoksa bunu “None” bırakın: bu yedi pim genel amaçlı açık kollektörlü çıkışlardır ve operatörler onları amplifikatör PTT'sine, anten rölelerine ve transverter anahtarlamasına da bağlar. Bant verisiyle sürmek, bağlı olan her şeyi çalıştırmaya başlar. Uygula / yeniden bağlan ile etkinleşir.
settings-radio-io-board-rx-input = IO kartı RX girişi
settings-radio-where-n2adr-hl2ioboard-takes-receive = Bir N2ADR HL2IOBoard'un alma sinyalini nereden aldığı. Kart kendisi otomatik bulunur ve bir ayar gerektirmez; bu ayar yalnızca kendi SMA jaklarını bağlamış operatörler içindir. Aksi halde “Radio's own input”ta bırakın — IO kartının J9'unu, ona hiçbir şey bağlı değilken seçmek alıcıyı sağır bırakır. Uygula / yeniden bağlan ile etkinleşir. J9 seçeneklerinden birine ayarlandığında, girişler ANT denetiminde de görünür ve bant başına hatırlanır.
settings-radio-power-amplifier = Güç amplifikatörü
settings-radio-use-hermes-lite-2-s = Hermes-Lite 2'nin kartüstü PA'sını kullan
settings-radio-default-what-you-want-unless = Varsayılan olarak açıktır ve harici bir amplifikatör kartın düşük güçlü RF1 çıkışından sürülmediği sürece istediğiniz budur. Kapalıyken radyo yine anahtarlanır — T/R rölesi çalışır ve her aksesuar kartı izler — ama anten jakı hiç güç üretmez ve röle bilerek alma konumunda tutulur. Hermes-Lite dışındaki kartlarda yok sayılır.
settings-radio-mirror-board-s-spectrum-about = Kartın spektrumunu, ayarlı frekansın etrafında aynalar; yalnızca alma değil yayında da. Varsayılan olarak açıktır: bir Hermes-Lite 2 bunu gerektirir. Yalnızca sinyaller skalanın yanlış tarafında görünüyor ve hiçbir şey çözülmüyorsa kapatın — ele veren şey, inandırıcı izlerle dolu bir şelale iken SSB'nin yanlış yan banda düşmesi ve FT8'in hiç çözüm döndürmemesidir.
settings-radio-frequency-correction = Frekans düzeltme
settings-radio-crystal-tcxo-error-ppm-applied = Kristal/TCXO hatası ppm cinsinden, RX ve TX'e uygulanır. Hemen etkinleşir.
settings-radio-ppm = {" "}ppm
settings-radio-transmit-buffer = Yayın arabelleği
settings-radio-how-far-ahead-real-time = sdroxide yavaşlayıp eşleşmeden önce, yayın sesi gerçek zamandan ne kadar ileride ağ üzerinden karta beslenir. Kartın kendisi böyle bir arabellek tutmaz — bu yalnızca sdroxide'ın kendi payını genişletir. Düşük varsayılan (doğrudan kablolu bağlantı için doğru), bir WiFi bağlantısında ya da bir VPN'de jitter'a karşı yeterli pay sağlamaz ve yayınlanan ses ya da PTT takılır; bu durumda yükseltin. Yüksek değer yayın gecikmesi maliyetidir. APPLY ile etkinleşir ve kartla yeniden bağlanır.
settings-radio-puresignal = PureSignal
settings-radio-adaptive-predistortion-linearise-transmitter-sample = Uyarlamalı ön bozma: verici, gerçekten yaydığının bir örneğinden doğrusallaştırılır; aynı gücde yirmi küsur desibel daha az intermodülasyon için. Alıcı geri besleme yoludur — kart, bir yayın boyunca almaya devam eder — bu yüzden yönlü bir kuplaj ve bir zayıflatıcı, amplifikatör çıkışının bir örneğini, yayında T/R anahtarının almadığı bir girişe koymalıdır. Bir Hermes-Lite 2'de bu, IO kartının PureSignal jakı demektir ve yukarıdaki alma girişi buna ayarlanır; bunu içermeyen bir kartta, bir ANAN'ın RX portu ya da kuplajı yayın süresince devreye sokan harici bir röle. Okunan, bu alıcıdır — ikinci bir ADC'li bir kart, kendi yazılımı onu nasıl yönlendirirse yönlendirsin, RX2'yi geri besleme tapı olarak kullanamaz. Hiçbir şey kuplaj yapılmadığında döngü hiç kilitlenmez ve verici tam olacağı gibi bırakılır. Uygula / yeniden bağlan ile etkinleşir ve yalnızca vericinin sahibi olan radyo için (DDC1).
settings-radio-correct-transmitter-receiver-s-own = Vericiyi alıcının kendi geri beslemesinden düzelt
settings-radio-table-steps = {"  "}Tablo adımları
settings-radio-how-finely-amplifier-s-curve = Amplifikatörün eğrisinin ne kadar ince modellendiği. Daha fazla adım daha keskin bir dirseği izler ve doldurulması daha uzun sürer; 32 makul bir başlangıçtır.
settings-radio-adaptation = {"  "}Uyarlama
settings-radio-how-fast-correction-follows-what = Düzeltmenin kuplajın bildirdiğine ne kadar hızlı uyduğu. Yavaş doğrudur — bu, gürültülü bir geri besleme yolundan hareket etmeyen bir amplifikatör eğrisinin ortalamasını alır.
settings-radio-hold = Tut
settings-radio-stop-adapting-keep-correction-where = Uyarlamayı durdurun ve düzeltmeyi olduğu yerde tutun — ölçüm için ve öğrendiğinden memnun bir operatör için.
settings-radio-manual-ip-overrides-discovery-press = Elle IP, keşfin yerine geçer. Yeniden başlatmadan geçmek için “Apply / reconnect”e basın.
settings-radio-open-collector-outputs-band = Banda göre açık kollektörlü çıkışlar
settings-radio-one-control-word-per-band = Donanımınızın belgelerinin belirttiği gibi, bant başına bir denetim sözcüğü: bit 0 çıkış 1, bit 6 çıkış 7. RX, o bantta alırken; TX ise verici anahtarlıyken etkinleştirilir — bir filtre için aynı değeri, yalnızca anahtarlamanın bir tarafına ait olan şeyler için (bir amplifikatörün anahtar hattı, bir alma ön yükseltecinin atlatması) farklı değerler verin. 00'da bırakılan bantlar hiçbir şey etkinleştirmez. Uygula / yeniden bağlan ile etkinleşir.
settings-radio-alex-band-code = ALEX BAND CODE
settings-radio-replace-table-what-preset-would = Tabloyu, bu ön ayarın her bantta göndereceği şeyle değiştirin; düzenlemek için bir başlangıç noktası olarak. Yapılandırmanın başka hiçbir yeri değişmez.
settings-radio-clear = TEMİZLE
settings-radio-every-output-off-every-band = Her bantta her çıkış kapalı — “None”un yaptığı budur.
settings-radio-band = Bant
settings-radio-rx = RX
settings-radio-tx = TX
settings-radio-outputs-asserted = Çıkışlar etkin
settings-radio-other = Diğer
settings-radio-everywhere-outside-amateur-bands-short = Amatör bantların dışında her yerde — kısa dalga dinleme ve bir transverter skalasının hiçbir bandın kapsamadığı yere düştüğü her şey.
settings-radio-dongle = Dongle
settings-radio-rescan = Yeniden tara
settings-radio-re-list-usb-bus-no = USB veriyolunu yeniden listele. Hiçbir aygıt açılmaz, bu yüzden alım sırasında basmak güvenlidir.
settings-radio-no-dongles-press-rescan = dongle yok — Yeniden tara'ya basın
settings-radio-first-one-found = — bulunan ilk —
settings-radio-rtl2832u-s-resampler-reaches-225 = RTL2832U'nun yeniden örnekleyicisi 225–300 kHz ve 900 kHz–3,2 MHz'e ulaşır, aradakinin hiçbiri değil. Uygula ile etkinleşir.
settings-radio-often-drops-samples = {"  "}(çoğu zaman örnek düşürür)
settings-radio-agc = AGC
settings-radio-manual-setting-measurement-weak-signal = Elle ayar, ölçüm ve zayıf sinyal dijital modları içindir. Tuner ve demodülatörün bağımsız otomatik döngüleri vardır.
settings-radio-tuner-gain = Tuner kazancı
settings-radio-applies-immediately-no-reconnect-tuner = Hemen uygulanır — yeniden bağlanmadan. Tuner'ın 29 ayrık adımı vardır, bu yüzden değer üretebildiği en yakın adıma oturur. Tuner AGC çalışırken yok sayılır.
settings-radio-crystal-error-parts-per-million = Milyonda kısım cinsinden kristal hatası. RUST_LOG=sdroxide_rtlsdr=debug ile çalıştırın; günlük, yaklaşık 20 saniye sonra ölçülen saat hatasını yazar — girilecek sayı budur. Hemen uygulanır.
settings-radio-hf-reception = HF alımı
settings-radio-tuner-itself-starts-24-mhz =
    Tuner kendisi 24 MHz'de başlar. Bir RTL-SDR Blog V4 bunun altını donanımda yukarı dönüştürür; diğer dongle'lar HF'e yalnızca ADC'yi doğrudan örnekleyerek, V3'ün HF portu üzerinden ulaşır. Otomatik, 24 MHz'in altındaki her şeyi ADC'ye, üstündekini tuner'a verir.

    Doğrudan örnekleme, 17 m ve 15 m dahil her HF bandını kapsar — ADC'nin ikinci Nyquist bölgesinde, doğru yönde gelirler. Ancak ADC'nin girişini hiçbir şey filtrelemez; bu yüzden 28,8 MHz eksi skala neyse, onlarla birlikte gelir: 17 m'nin altında 10,7 MHz, 15 m'nin altında 7,726 MHz.

    Modları değiştirmek akışı kısaca keser.
settings-radio-removes-dongle-s-own-dc =
    Dongle'ın kendi DC tepesini aralığın ortasından ve her sinyalin etrafında yansıyan ayna görüntüsünü, örneklerin kendisindeki dengesizliği ölçerek kaldırır — kalibrasyon yok ve hemen uygulanır. Tuner'ın bir ofset-ayarlama modu yoktur, bu yüzden ortayı temizlemenin tek yolu budur.

    Skalaya tam ayarlanmış bir AM taşıyıcısı da DC'dedir, bu yüzden tepeyle birlikte gider: ondan bir kilohertz uzağa ayarlayın ya da bunu kapatın.
settings-radio-remove-centre-spike-mirror-image = Merkez tepesini ve ayna görüntüsünü kaldır
settings-radio-bias-tee = Bias tee
settings-radio-feed-4-5-v-dc = Koaksiyel kabloya ~4,5 V DC besle
settings-radio-bias-tee-never-connect-transceiver = Bias tee AÇIK. Bu etkinken bir telsiz, topraklı bir anten ya da başka yerden beslenen bir ön yükselteç bağlamayın — DC doğrudan besleme hattından iner.
settings-radio-receive-only-dongle-sample-rate = Yalnızca alma. Dongle ve örnekleme hızı Uygula ile etkinleşir; diğer her şey değiştirdikçe uygulanır.
settings-radio-server-address = Sunucu adresi
settings-radio-where-rtl-tcp-listening-address =
    rtl_tcp'nin dinlediği yer: bir adres ya da bir adres ve bağlantı noktası. Bağlantı noktası varsayılan olarak 1234'tür; bu, rtl_tcp'nin kendi varsayılanıdır.

    Uzak uçta, `rtl_tcp -a 0.0.0.0` ile başlatın — -a olmadan yaptığı gibi 127.0.0.1'e bağlanırsa yalnızca aynı makineden gelen bağlantıları kabul eder.

    Uygula ile etkinleşir.
settings-radio-host-host-port-e-g = ana makine ya da ana makine:bağlantı noktası, örn. raspberrypi.local:1234
settings-radio-requested-server-same-resampler-limits =
    Sunucudan istenir; yerel bir dongle ile aynı yeniden örnekleyici sınırlarıyla — uzak uçtaki aynı silikon olduğu için.

    Her hızın yanındaki rakam, bağlantıda ne maliyeti olduğudur: örnekler sıkıştırılmadan gönderilir ve ağın taşıyamayacağı bir hız, rtl_tcp'nin kaliteyi düşürmek yerine bağlantıyı kesmesine yol açar. Uygula ile etkinleşir.
settings-radio-wired-link = {"  "}(kablolu bağlantı)
settings-radio-runs-server-s-dongle-manual = Sunucunun dongle'ında çalışır. Elle, ölçüm ve zayıf sinyal dijital modları içindir.
settings-radio-applies-immediately-no-reconnect-sent = Hemen uygulanır — yeniden bağlanmadan. Onda bir dB cinsinden gönderilir ve sunucu, tuner'ında bulunan bir adıma yakalar; protokolün yanıtı yoktur, bu yüzden üzerinde durduğu değer geri okunamaz ve bu kaydırıcı istenen değeri göstermeye devam eder. Tuner AGC çalışırken yok sayılır.
settings-radio-crystal-error-server-s-dongle =
    Sunucunun dongle'ının kristal hatası, milyonda kısım cinsinden — o donanımın bir özelliği olduğu için burada ayarlanır, bu makinedeki dongle'larda değil. Hemen uygulanır.

    USB arabiriminin yazdırdığı ölçülen saat hatası burada mevcut değildir: bir ağ üzerinden bu ölçümün gördüğü şey kristal değil arabelleklemedir ve binlerce ppm yanlıştır. Dongle'ı bir kez USB üzerinden kalibre edin ve sayıyı taşıyın ya da bilinen frekanslı bir yayın istasyonuna ayarlayıp skalaya oturana dek ayarlayın.
settings-radio-tuner-starts-24-mhz-below =
    Tuner 24 MHz'de başlar; bunun altında uzak uçun yardıma ihtiyacı vardır. Bir Blog V4 kendi başına yukarı dönüştürür ve Otomatik onu olduğu gibi bırakır — protokol yalnızca tuner çipini bildirir, başka hiçbir şeyi bildirmez ve bir V4, kabloda düz bir R828D'den ayırt edilemez; bu yüzden yapabileceği tek şey budur.

    Bir V3'te ya da başka bir dongle'da Otomatik, tuner'ın kendi 24 MHz tabanının altında sunucuyu doğrudan örneklemeye geçirir — bu, 28,8 MHz eksi skalanın üstüne katlanmış, ADC'nin ikinci Nyquist bölgesinde gelen 17 m ve 15 m dahil her HF bandını kapsar. HF'de hiçbir şey duymayan düz bir R828D için Doğrudan örnekleme'yi açıkça seçin. Geçiş akışı kısaca keser.
settings-radio-removes-dongle-s-dc-spike =
    Dongle'ın DC tepesini ve ayna görüntüsünü burada, örnekler geldikleri anda kaldırır — bunlar donanımın yapay ürünleridir, bu yüzden ağ üzerinden diğer her şeyle birlikte giderler. Hemen uygulanır.

    Skalaya tam ayarlanmış bir AM taşıyıcısı da DC'dedir, bu yüzden tepeyle birlikte gider: ondan bir kilohertz uzağa ayarlayın ya da bunu kapatın.
settings-radio-powers-preamp-far-end-s = Bir ön yükselteci uzak ucun dongle'ından besler. Eski sunucular komutu uygulamaz ve sessizce yok sayar — protokolün hayır demenin bir yolu yoktur — bu yüzden açılmayan bir bias tee, mutlaka bu uçtan kaynaklanmaz.
settings-radio-feed-4-5-v-dc-2 = Uzak koaksiyel kabloya ~4,5 V DC besle
settings-radio-bias-tee-hardware-somewhere-else = Bias tee AÇIK ve donanım başka bir yerde. Bu besleme hattının diğer ucundaki her şey — bir telsiz, topraklı bir anten, zaten beslenen bir ön yükselteç — kontrol etmek için karşınızda değildir.
settings-radio-sdrplay-server-rsp-tcp = SDRplay sunucusu (rsp_tcp)
settings-radio-sdrplay-server-greets-exactly-like = Bir SDRplay sunucusu tıpkı bir dongle gibi selamlar, bu yüzden bunlar bir tane algılandığında değil her zaman gösterilir. Sıradan bir rtl_tcp sunucusuna karşı yok sayılırlar — protokolün yanıtı yoktur ve tanımadığı komutları atar. Aygıt sekmesi, kendini tanıttığı anda sunucuyu rsp_tcp olarak adlandırır.
settings-radio-input = Giriş A
settings-radio-input-b = Giriş B
settings-radio-hi-z = Hi-Z
settings-radio-lna-state = LNA durumu
settings-radio-step-index-not-db-figure = Bir dB rakamı değil, bir adım indeksi: her adımın ne kadar değerli olduğu RSP modeline ve banda bağlıdır. 0 en fazla kazançtır.
settings-radio-if-gain-reduction = IF kazanç azaltma
settings-radio-reduction-so-more-less-signal = Bir azaltma, yani daha fazlası daha az sinyal demektir. Yalnızca RSP'nin AGC'si kapalıyken uygulanır.
settings-radio-enable = Etkinleştir
settings-radio-dbfs = {" "}dBfs
settings-radio-level-rsp-s-agc-aims = RSP'nin AGC'sinin tutmayı hedeflediği seviye.
settings-radio-notches = Çentikler
settings-radio-am = AM
settings-radio-fm = FM
settings-radio-dab = DAB
settings-radio-rf = RF
settings-radio-reference-out = Referans çıkışı
settings-radio-t-24-mhz-clock-out = 24 MHz saat çıkışı
settings-radio-rsp2-rspduo-only = Yalnızca RSP2 ve RSPduo.
settings-radio-receive-only-address-sample-rate = Yalnızca alma. Adres ve örnekleme hızı Uygula ile etkinleşir; diğer her şey değiştirdikçe uygulanır. Kopan bir bağlantı kendiliğinden yeniden denenir; bu yüzden yeniden başlatılan bir sunucu, buradaki hiçbir şeye dokunmadan geri döner.
settings-radio-where-spyserver-listening-address-address =
    spyserver'ın dinlediği yer: bir adres ya da bir adres ve bağlantı noktası. Bağlantı noktası varsayılan olarak 5555'tir; bu, spyserver'ın kendi varsayılanıdır.

    Uzak uçta, yapılandırma dosyasının diğer makinelerin ulaşabileceği bir adrese bağlandığını denetleyin — 127.0.0.1'e bağlıysa yalnızca aynı makineden gelen bağlantıları kabul eder.

    Uygula ile etkinleşir.
settings-radio-host-host-port-e-g-2 = ana makine ya da ana makine:bağlantı noktası, örn. raspberrypi.local:5555
settings-radio-i-q-bandwidth = I/Q bant genişliği
settings-radio-how-wide-slice-band-arrives =
    Bandın I/Q olarak ne kadar geniş bir dilimi gelir — ve bu yüzden bağlantının ne kadarını kullandığı. Bu *demodüle edilmiş* penceredir: skalayı izler ve ondan geniş olan her şey, panadaptörün üstündeki şeritteki sunucunun FFT'sidir.

    Otomatik, geniş FM dahil buradaki her modu taşıyan ve yine de bir hücresel yukarı bağlantıdan geçen yaklaşık 96 kHz'i hedefler. Uygula ile etkinleşir.
settings-radio-which-stage-server-s-own =
    Sunucunun kendi hız merdiveninin hangi basamağının isteneceği. Her alıcının farklı bir merdiveni vardır — maksimum hızı basamak basamak yarılanarak elde edilir — bu yüzden bu, hertz cinsinden bir rakam değil basamak olarak saklanır ve aynı ayar, farklı bir sunucuya yöneltildiğinde de bir anlam taşır.

    Otomatik, yaklaşık 1 Msps'i hedefler. Bu sunucuda basamakların ne olduğunu görmek için Bağlantıyı test et'e basın. Uygula ile etkinleşir.
settings-radio-automatic = Otomatik
settings-radio-sample-format = Örnek biçimi
settings-radio-bits-per-component-wire-so =
    Kabloda bileşen başına bit, ve bu yüzden bağlantıda ne maliyeti olduğu: 16 bit, 8 bitin iki katıdır; 32 bit kayan nokta ise alıcının ADC'sinin sahip olduğundan fazla bilgi olmadan dört katıdır.

    8 bit, uzak bir alıcının bir ev internet bağlantısı üzerinden çalışmasını sağlayan şeydir ve neredeyse her şey için doğrudur. Bir sunucu bunu geçersiz kılabilir — kimileri tek bir biçimde ısrar edecek şekilde yapılandırılmıştır — ve bunu yaptığında günlükte söyler. Uygula ile etkinleşir.
settings-radio-gain = Kazanç
settings-radio-server-s-gain-stage-index =
    Sunucunun kazanç basamağı, bir indeks olarak — desibel sayısı olarak değil. Her indeksin ne anlama geldiği uzak uçtaki alıcıya aittir ve banda göre değişir; protokol bunu hiçbir zaman söylemez, bu yüzden buradaki hiçbir şey, bir rakam uydurmadan onu dB'ye çeviremez.

    Gerçek aralık sunucuya aittir ve yalnızca bağlanıldığında bilinir; ondan büyük bir indeks sınırlanır. Hemen uygulanır — yeniden bağlanmadan.
settings-radio-index = indeks{" "}
settings-radio-digital-gain = Dijital kazanç
settings-radio-how-far-server-scales-samples =
    Sunucunun, kabloya göndermeden önce nicemlemeden önce örneklerini ne kadar yukarı ölçeklediği. Otomatik gelen şeyi izler ve tepeyi tam ölçeğin yarısında tutar; ölü bir bant ile kalabalık bir bantta da doğru olabilecek tek şey budur — ikisi otuz desibel ayrıdır ve sabit bir rakam ikisine de hizmet etmez.

    Yalnızca 8 bitte önemlidir; tam ölçeğin çok altındaki bir sinyal, alt bitlerini nicemleyiciye kaybeder ve üstündekiler düz kırpılır. Daha geniş biçimlerin yeri bol olur ve diğer her istemcinin hesapladığı rakamı, kazanç indeksinden ve azaltma basamağından alır. Hemen uygulanır.
settings-radio-full-band-strip = Tüm bant şeridi
settings-radio-server-s-own-fft-whole =
    Sunucunun tüm bandın kendi FFT'si, panadaptörün üstündeki şeritte çizilir. Bu arayüzde bulunan tek bant görünümüdür — panadaptörün kendisi yalnızca alınan I/Q kadar geniştir.

    Kapatmak, alıcıyı zaten ayarlı olmadığı hiçbir şeyi görememeye bırakır. Hemen uygulanır; sunucu gönderdiğini değiştirirken sesde kısa bir kesintiyle.
settings-radio-ask-server-low-rate-fft =
    Sunucudan I/Q'nun yanında tüm bandın düşük hızlı bir FFT'sini isteyin ve panadaptörün üstündeki şeritte çizin.

    Neredeyse hiçbir şey maliyeti yoktur — çerçeve başına birkaç kilobayt, saniyede bir düzine kez — ve demodüle edilen dilimi değil, tüm alıcıyı gösterir. Yalnızca her baytın sayıldığı bir bağlantıda kapatmaya değer. Hemen uygulanır; sesde kısa bir kesintiyle.
settings-radio-show = Göster
settings-radio-whole-band = tüm bant
settings-radio-how-much-receiver-strip-covers = Şeridin alıcının ne kadarını kapsadığı. Tüm bant, mevcut en geniş görünümdür; daraltmak, aynı sayıda kutuyu daha az spektrume yayar — daha küçük bir aralık üzerinde daha ince ayrıntı.
settings-radio-strip-db-window = Şerit dB penceresi
settings-radio-server-quantises-fft-into-window =
    Sunucu, göndermeden önce FFT'sini bu pencereye nicemler — kutu başına bir bayt; bu yüzden bunlar yalnızca şeridin nasıl çizildiğini değil, ne kadar ince ölçüldüğünü belirler. Şeridin *gösterildiği* taban ve tavan, motorun kendi otomatik ayarlamasıyla belirlenir ve ayrı bir şeydir.

    Varsayılan 150 dB tüm protokol aralığıdır ve dikkat gerektirmez. Gürültü tabanı çevresinde daraltmak, tüm sinyalleri ölçeğin küçük bir kısmında olan bir alıcıda çözünürlük kazandırır. Hemen uygulanır.
settings-radio-top = üst{" "}
settings-radio-range-2 = aralık{" "}
settings-radio-i-q-correction = I/Q düzeltme
settings-radio-remove-dc-spike-mirror-image = DC tepesini ve ayna görüntüsünü, bu tarafta DSP'de kaldır. Uzak uçtaki alıcının bunu gerektirip gerektirmediği ne olduğuna bağlıdır — bir Airspy HF+ gerektirmez, bir RTL-SDR gerektirir — ve protokol hangisiyle konuşulduğunu söylemez. Hemen uygulanır.
settings-radio-enabled = Etkin
settings-radio-test-connection = Bağlantıyı test et
settings-radio-connect-read-what-server-says = Bağlanın, sunucunun alıcısı hakkında ne söylediğini okuyun ve yeniden bağlantıyı kesin — akış başlatmadan, bu yüzden başka birinin kullandığı bir sunucuya karşı basmak güvenlidir.
settings-radio-receive-only-panadapter-narrow-i = Yalnızca alma. Panadaptör, skalayı izleyen dar I/Q penceresidir; bant görünümü, onun üstündeki şeritteki sunucunun FFT'sidir. Yeniden başlatmadan geçmek için “Apply / reconnect”e basın.
settings-radio-receive-only-wideband-i-q = Yalnızca alma. Geniş bant I/Q, herhangi bir yerel SDR ile aynı. Yeniden başlatmadan geçmek için “Apply / reconnect”e basın.
settings-radio-receiver-address = Alıcı adresi
settings-radio-where-receiver-serves-web-page =
    Alıcının web sayfasını sunduğu yer: bir adres ya da bir adres ve bağlantı noktası. Bağlantı noktası varsayılan olarak 8073'tür; bu, bir KiwiSDR'ın kendi varsayılanıdır.

    Proje vekili üzerinden erişilen bir alıcının (something.proxy.kiwisdr.com — kamuya açık olanların neredeyse yarısı) bunun yerine 80 numaralı bağlantı noktasında yanıt verdiğini unutmayın; bu yüzden açıkça belirtin. “Public SDRs”tan bir alıcı seçmek bunu doğru doldurur.

    Uygula ile etkinleşir.
settings-radio-host-host-port-e-g-3 = ana makine ya da ana makine:bağlantı noktası, örn. kiwi.example.org:8073
settings-radio-password = Parola
settings-radio-receiver-s-user-password-where =
    Alıcının *kullanıcı* parolası, operatörü bir tane ayarlamışsa. Neredeyse her kamuya açık alıcıda boştur. Yönetici parolası değildir.

    Uygula ile etkinleşir.
settings-radio-announce = Şu adla duyur
settings-radio-name-connection-shows-up-under =
    Bu bağlantının görüneceği ad; alıcının sahibi ve onu dinleyen herkes görebilir.

    Boş bırakılırsa çağrı işaretiniz olur; bu ağın kendi kuralıdır — kimliğini bildirmeyen istemcileri engelleyen operatörler de vardır. Uygula ile etkinleşir.
settings-radio-blank-station-callsign = boş = istasyon çağrı işareti
settings-radio-band-view = Bant görünümü
settings-radio-ask-receiver-own-waterfall-well =
    I/Q'sunun yanında alıcıdan kendi şelalesini de isteyin ve panadaptörün üstündeki şeritte çizin.

    Değerlidir: I/Q yalnızca yaklaşık 12 kHz genişliktedir, bu olmadan ayarlanacak *bir şey* yoktur. Tam hızda saniyede yaklaşık 20 kB taşıyan ikinci bir bağlantıdır; I/Q'nun 44 kB/s'ine karşı.

    Uygula ile etkinleşir.
settings-radio-show-receiver-s-0-30 = alıcının 0-30 MHz şelalesini göster
settings-radio-band-view-speed = Bant görünümü hızı
settings-radio-how-often-receiver-sends-waterfall =
    Alıcının bir şelale satırını ne sıklıkla gönderdiği, 1 (en yavaş) ile 4 arasında. Burada çalışırken bağlantının maliyetini değiştiren tek ayar budur; bu yüzden ölçülen bir bağlantıda düşürün.

    Hemen uygulanır.
settings-radio-band-view-span = Bant görünümü aralığı
settings-radio-how-much-band-receiver-s =
    Alıcının şelalesinin ne kadar bandı kapsadığı. Kaydırıcının solunda tam 0-30 MHz'i, sağa her adımda yarıya inerek.

    Pencere ne kadar geniş olursa olsun şelale her zaman 1024 kutu genişliktedir, bu yüzden bu gerçekte bir çözünürlük denetimidir: tüm bant kutu başına 29 kHz'dir ve bu, bir şeyin resmi değil bir bant *haritasıdır*; birkaç yüz kilohertz ise bir yayın bandında tek tek istasyonları gösterir. Yakınlaştırıldığında pencere sizin skalanızı izler.

    Tüm bant varsayılandır ve şeridi ayarlanacak bir şey yapan budur; altındaki panadaptör ise yaklaşık 12 kHz'lik I/Q'dur. Hemen uygulanır.
settings-radio-receiver-agc = Alıcı AGC
settings-radio-receiver-s-agc-which-far =
    Alıcının AGC'si, bağlantının uzak tarafındadır ve I/Q'dan önce gelir — bu yüzden sdroxide'ın yaptığı her şeyden önce devreye girer.

    Buradaki her yerel SDR'nin aksine varsayılan olarak açıktır, çünkü ölçülen alternatif daha kötüydü: canlı bir alıcıda elle kazanç aralığı boyunca tekdüze değildi ve üst ucu I/Q'yu tam ölçekte kırpıyordu; AGC ise 7 dB boşlukla yaklaşık -24 dBFS'de tutuyordu.

    Bu, örnek genliğinin bir sinyal seviyesi olmadığı anlamına gelir; bu yüzden S-metre bunun yerine alıcının kendi rakamından okunur. Hemen uygulanır.
settings-radio-let-receiver-ride-own-gain = alıcının kendi kazancını izlemesine izin ver
settings-radio-manual-gain = Elle kazanç
settings-radio-fixed-gain-when-receiver-s = Alıcının AGC'si kapalıyken sabit kazanç, kendi 0-90 ölçeğinde. Belirtilmiş herhangi bir şeyin desibeli değil — protokol buna manGain der ve daha fazlasını söylemez, ve tekdüze olmadığı ölçülmüştür; bu yüzden bir kalibrasyon değil, iyi bir nokta bulmak için bir kadran gibi ele alın. Hemen uygulanır.
settings-radio-read-receiver-s-status-page = Alıcının durum sayfasını okuyun: ne olduğunu, neyi kapsadığını, kaç kanalın boş olduğunu ve operatörünün bir tarayıcı dışındaki uygulamalardan bağlantıya izin verip vermediğini. Kanallarından hiçbirini almaz.
settings-radio-host-port-e-g-127 = ana makine:bağlantı noktası, örn. 127.0.0.1:50001
settings-radio-iq-sample-rate = IQ örnekleme hızı
settings-radio-receiver = Alıcı
settings-radio-rig-two-receivers-sunsdr2dx-can = İki alıcılı bir telsiz (SunSDR2DX) tek bir bağlantıdan iki radyo sekmesi sunabilir — bu radyoyu RX1'de, başka bir radyoyu RX2'de çalıştırın. Verici RX1 radyosuna aittir.
settings-radio-stream-delay = Akış gecikmesi
settings-radio-how-long-rig-s-iq = sdroxide yeni bir merkeze taşıdıktan sonra telsizin IQ'sunun ona ulaşması ne kadar sürer. Bu, ağ değil telsizin kendi DSP hattıdır — bu makinedeki bir sunucunun da bir tane vardır — ve panadaptör tam uzaklaştırılmış halde sürüklenirken, bu kadar yanlış olmak, en yeni şelale satırlarını geçmişe göre yanlara, hatanın sürükleme hızıyla çarpımı kadar kaydırır. Çok yüksek olması, çok düşük olmasıyla tam aynı kadar yerinden eder, yalnızca ters yönde. `cargo run --release -p sdroxide-tci --example retune_latency` ile ölçün; varsayılan, 192 kHz'de ExpertSDR3 üzerinde bir SunSDR2DX'tir.
settings-radio-wideband-iq-receive-audio-transmit = Geniş bant IQ alma, ses yayını. Yeniden başlatmadan geçmek için “Apply / reconnect”e basın.
settings-radio-radio-address = Radyo adresi
settings-radio-address-shown-radio-under-set = Radyoda SET > Network altında gösterilen adres. Orada Network Control açık olmalı ve radyoda bir ağ kullanıcı adı ile parola ayarlanmış olmalıdır.
settings-radio-host-ip-e-g-192 = ana makine ya da IP, örn. 192.168.1.50
settings-radio-control-port = Kontrol bağlantı noktası
settings-radio-t-50001-unless-has-been-changed = Radyoda değiştirilmedikçe 50001.
settings-radio-network-user = Ağ kullanıcısı
settings-radio-stored-clear-radio-json-protocol = radio.json içinde açık olarak saklanır. Protokol bunu kabloda geri döndürülebilir biçimde karıştırır, yani buradaki hiçbir şey onu sır yapmaz.
settings-radio-receive = Alma kaynağı
settings-radio-af-radio-demodulates-sdroxide-shows = AF: radyo demodüle eder ve sdroxide ses bandını gösterir. 12 kHz IF: radyo bunun yerine DRM ara frekansını gönderir ve sdroxide demodüle eder; bu, kendi filtrelerini, gürültü azaltmasını ve çözücülerini yaklaşık ±12 kHz üzerinde işe koşar. Her iki durumda da geniş şelale radyonun kendi ekranıdır — hiçbir Icom I/Q çıkarmaz. Bir IF tercihi, WFM gibi geniş modlar için geçici olarak AF kullanır, sonra uyumlu modlar için IF'i geri yükler. Bir AF tercihi AF olarak kalır. HD Radio bu bağlantıda kullanılamaz.
settings-radio-audio-sample-rate = Ses örnekleme hızı
settings-radio-t-12-khz-if-needs-48000 = 12 kHz IF, 48000 Hz akışı gerektirir — bu hızda radyonun demodüle edilmiş sesi kullanılır.
settings-radio-if-spectrum = IF spektrumu
settings-radio-which-way-round-radio-s = Radyonun 12 kHz IF'inin hangi yönde çalıştığı. “Automatic”, modelin ne yaptığı bilinen şeyi kullanır — IC-7760'ta ters, diğer her Icom'da normal. SSB yanlış yan bantta çıkıyorsa elle ayarlayın: ele veren şey, bant LSB çalışırken USB seçmek zorunda kalmanızdır; radyonun kendi mod göstergesi ise sdroxide ile baştan sona aynı fikirdedir.
settings-radio-normal = Normal
settings-radio-mirrored = Ters
settings-radio-displayed-bandwidth = Gösterilen bant genişliği
settings-radio-width-audio-band-panadapter-cat = CAT telsizinde olduğu gibi ses bandı panadaptörünün genişliği. Dijital modlarda ve radyonun ekranı kapalıyken kullanılır; aksi halde panadaptör ekrandır ve Ekran aralığı genişliğini ayarlar.
settings-radio-how-cw-panel-s-keyer-2 = CW panelinin keyer'ının nasıl yayın yaptığı. “Rig keyer”, radyoyu CW'ye alır ve gönderilecek metni kendi keyer'ına verir (CI-V 0x17). “Sound card”, anahtarlanan yan sesi bunun yerine LAN üzerinden ses olarak gönderir (MCW), skalanın artı perdede bir tonu — ve CW'deki bir telsiz modülatör girişini yok sayacağı için CW seçmek, radyoyu düz USB'de tutar; dijital modların bindiği aynı mod.
settings-radio-how-much-audio-radio-holds = Radyonun modüle etmeden önce tuttuğu ses miktarı. Daha fazlası kötü bir ağa dayanır, yayın gecikmesi pahasına.
settings-radio-streams-radio-s-own-475 = Radyonun kendi 475 kutulu taramasını akıtır. AF'de bu panadaptördür — radyonun gönderdiği ses, filtresinden geçen şeydir, bandın bir resmi değildir — ve 12 kHz IF'de bunun üstündeki tüm bant şelalesidir. Her iki durumda da sdroxide'ın DSP'si değil, radyonun resmidir: bunu hesaplayacak bir I/Q yoktur.
settings-radio-how-wide-sweep-only-wide = Ne kadar geniş taranacağı. Bir Icom'un sahip olduğu tek geniş görünüm budur ve radyo, kendi ekranında en son seçilen aralığı tutar — genellikle birkaç kHz, bu yüzden şerit altındaki panadaptörden daha geniş gelemez. Burada bir aralık ayarlamak, ekranı ortadan-merkez moduna da alır; böylece skalayı izler. Telsizin kendi ekranını da değiştirir; “As set on the radio” onu olduğu gibi bırakır.
settings-radio-switch-modulation-input-lan = Modülasyon girişini LAN'a geçir
settings-radio-transmit-audio-only-heard-when = Yayın sesi, yalnızca radyonun MOD girişi LAN'a ayarlıysa duyulur. sdroxide, menü numaralandırmasını bildiği bir modelde bunu yazabilir; başka herhangi birinde bunu söyler ve menüyü olduğu gibi bırakır. Bu bir ödünçtür: orada tutulan her şey önce okunur ve oturum sona erdiğinde geri konur; böylece sonradan tek başına kullanılan bir telsiz hâlâ kendi mikrofonunu duyar.
settings-radio-copy-diagnostic-report = Tanı raporunu kopyala
settings-radio-copies-radio-s-last-session = Bu radyonun son oturumunu — el sıkışması ve CI-V izini — bir hata raporu için panoya kopyalar. İstasyonun değil bu radyonun: LAN'da iki Icom varsa her sekme kendi adresi hakkında yanıt verir.
settings-radio-control-audio-radio-s-own = Kontrol, ses ve radyonun kendi ekranı tek bir ağ bağlantısı üzerinden — seri kablo ve ses kartı yok. Yeniden başlatmadan geçmek için “Apply / reconnect”e basın.
settings-radio-testing = Test ediliyor…
settings-radio-was-only-check-press-apply = Bu yalnızca bir denetimdi — bu radyoyu kullanmaya başlamak için aşağıdaki Apply / reconnect'e basın.
settings-radio-radios = Radyolar
settings-radio-asks-network-iio-devices-also = Ağdan IIO aygıtlarını ister ve 192.168.2.1'i de doğrudan dener — bir USB kablosunun ucundaki bir Pluto, adresi çalışsa bile çoğu zaman çok noktalı yayınla ulaşılamaz.
settings-radio-no-radios-press-discover = radyo yok — Keşfet'e basın
settings-radio-address = Adres
settings-radio-overrides-selection-above-usb-cable = Yukarıdaki seçimi geçersiz kılar. USB kablosu Pluto'yu bir seri bağlantı noktası değil bir ağ adaptörü olarak sunar; bu yüzden radyo masanızda olsa bile bu bir IP adresidir.
settings-radio-pluto-revision-c-pluto-unlocked = Bir Pluto+ ya da 2R2T'ye kilitlenmiş bir revizyon-C Pluto, tek bir kutudan iki radyo sekmesi sunabilir — bu radyo RX1'de, başka biri RX2'de, her biri kendi anteninde. İki zincir tek osilatörü paylaşır; bu yüzden herhangi bir radyoyu yeniden ayarlamak ikisini de hareket ettirir; RX2'nin kazandırdığı şey aynı spektrumda ikinci bir anten (çeşitlilik), ikinci bir bant değildir. Verici RX1 radyosuna aittir. Standart 1R1T bir Pluto, bağlandığında RX2'yi reddeder.
settings-radio-width-spectrum-sdroxide-receives-ad9361 = sdroxide'ın aldığı spektrumun genişliği. AD9361 61,44 Msps'ye ulaşır; USB ağ bağlantısı ulaşmaz ve bu liste ona göre ölçeklenmiştir. En düşük hızlar AD9361'e yüklenmiş bir filtre yapılandırması gerektirir, sdroxide bunu yapmaz — standart bir Pluto onları bunun yerine yaklaşık 2,084 Msps'de çalıştırır ve bağlandığında bunu söyler. Uygula ile etkinleşir.
settings-radio-more-than-usb-2-will = {"  "}(USB 2'nin taşıyabileceğinden fazla)
settings-radio-stock-pluto-runs-2-084 = {"  "}(standart bir Pluto 2.084'te çalışır)
settings-radio-analog-filter = Analog filtre
settings-radio-ad9361-s-baseband-filter-leave = AD9361'in taban bant filtresi. Otomatik için 0'da bırakın; bu, örnekleme hızının dokuzda dokuzuna açar — kasıtlı olarak geniş, çünkü alıcı, sinyalleri DC tepesinden uzak tutmak için osilatörünü aralığın dörtte biri kadar skaladan uzağa park eder ve dar bir filtre tam da onları taşıdığı kısmı keserdi. Uygula ile etkinleşir.
settings-radio-ad9361-has-four-modes-not = AD9361'in açık/kapalı bir anahtarı değil, dört modu vardır. Yavaş atak SSB ve CW'ye uygundur; hızlı atak patlamalı sinyallere; elle ise ölçüm ve zayıf sinyal dijital modları içindir. Hemen uygulanır.
settings-radio-rx-gain = RX kazancı
settings-radio-applies-immediately-no-reconnect-ignored = Hemen uygulanır — yeniden bağlanmadan. AGC elle ayarlanmadıkça yok sayılır; bu, sdroxide'ın değil AD9361'in kendi davranışıdır.
settings-radio-tx-gain = TX kazancı
settings-radio-negative-because-ad9361-expresses-transmit = AD9361 yayın seviyesini zayıflatma olarak ifade ettiği için negatif: 0 dB tam çıkıştır. Hemen uygulanır. Bu değer bağlanırken uygulanmadan önce verici en sessiz konuma getirilir; böylece önceki programın bıraktığı hiçbir şey canlı olmaz.
settings-radio-reference-error-parts-per-million = Milyonda kısım cinsinden referans hatası. RUST_LOG=sdroxide_pluto=debug ile çalıştırın; günlük yaklaşık 20 saniye sonra ölçülen saat hatasını yazar — girilecek sayı budur. sdroxide tarafından uygulanır, radyonun kendi kalıcı ayarına yazılmaz. Hemen uygulanır.
settings-radio-full-duplex = Tam çift yön
settings-radio-keep-receiving-while-transmitting = Yayın yaparken almaya devam et
settings-radio-leave-off-pluto-reached-over =
    USB kablosu üzerinden erişilen bir Pluto'da bunu kapalı bırakın. Bu bağlantı, saniyede bir megaörneği iki yönde birden taşıyamaz ve yayın arabelleğini aç bırakan bir yayın parçalı çıkar — bu yüzden varsayılan olarak bir yayın süresince alma durur.

    Yer olduğu için her ikisine de gerçek Ethernet üzerindeki bir kart için açın (bir LibreSDR, bir gigabit adaptördeki bir Pluto): o zaman alıcıyı kendi yayınınız boyunca duyarsınız; bir QO-100 istasyonunun kendi aşağı bağlantısını dinlemesi böyledir.

    Bir yayın, örnekleme hızının iki katı kadar bağlantı ister — 2,5 Msps her yönde 10 MB/s demektir — bu yüzden günlük bağlantının bunu taşımadığını söylemeye başlarsa hızı düşürün. Şelale, bir yayın sırasında yine yayınlanan sinyali gösterir; devam eden ses odur.

    Uygula ile etkinleşir.
settings-radio-duplex = Çift yön
settings-radio-whether-ad9361-runs-both-directions =
    AD9361'in iki yönü aynı anda mı (FDD — bir Pluto'nun açılış biçimi ve sdroxide'ın onu hep bıraktığı durum) yoksa tek tek mi (TDD) çalıştırdığı.

    PTT pimlerini istemedikçe bunu FDD'de bırakın — TDD, onların anahtarladığı moddur ve yukarıdaki Tam çift yönü dışlar. Uygula ile etkinleşir.
settings-radio-ptt-pins = PTT pimleri
settings-radio-pluto-s-four-gpo-test =
    Pluto'nun dört GPO test noktası, bir harici güç amplifikatörünü, LNA'yı ya da alma-verme anahtarını tek başlarına anahtarlayabilir. Bir çift seçin: radyo alırken bir pim yüksek, yayın yaparken öteki yüksek olur — döngüde hiçbir ana bilgisayar yazılımı ve kablolanacak bir seri PTT hattı yoktur.

    Duplex satırı ne derse desin bu radyoyu TDD'ye alır, çünkü pimler AD9361'in etkinleştirme hatlarını izler ve FDD'de ikisi de oturum boyunca etkindir. Tam çift yön onunla birlikte kapanır; bu yüzden uydularla çalışıyorsanız ve kendi aşağı bağlantınızı duymanız gerekiyorsa ikisini de varsayılanlarında bırakın.

    Analog Devices'ın kendi notu harici bir LNA'yı GPO0/GPO1'e koyar; kartınız öyle kablolanmışsa GPO2/GPO3'ü kullanın. Pimler birkaç miliamperde yaklaşık 1,3 V verir: bunlarla bir transistör ya da bir optik izolatör sürün, asla bir röle bobinini değil.

    Uygula ile etkinleşir.
settings-radio-buffer-size = Arabellek boyutu
settings-radio-how-much-device-holds-before =
    Aygıtın her aktarımdan önce, karmaşık örnek cinsinden ne kadar tuttuğu. Varsayılan 32768, 2 Msps'de yaklaşık 16 ms'dir: aktarım başına gidiş-dönüşün darboğaz olmayacağı kadar uzun, bir yeniden ayarın gözle görülür biçimde geç kalmayacağı kadar kısa.

    Günlük alma soketinin değiştirildiğini bildirirse yarıya indirin. Sınırda bir bağlantı — yüksek bir örnekleme hızında USB kablosundan erişilen bir Pluto bunun tipik örneğidir — bir aktarımın ortasında takılır ve daha küçük bir aktarım hem ortada yakalanma olasılığı daha düşük hem de sonradan düzeltilmesi daha hızlıdır. Yeniden ayar gecikmesine karşılık daha az gidiş-dönüş için yükseltin.

    Uygula ile etkinleşir.
settings-radio-samples = {" "}örnek
settings-radio-rx-tx-port = RX / TX bağlantı noktası
settings-radio-ad9361-s-rf-port-select = AD9361'in rf_port_select değeri. Standart bir Pluto her birinden birer tane kablolar, bu yüzden bunu öyle yapmayan bir kartınız yoksa boş bırakın. Uygula ile etkinleşir.
settings-radio-opens-radio-reads-what-says = Radyoyu açar, kendisi hakkında söylediklerini okur ve bu belirli kartın bildirdiği ayar aralığını raporlar. Bir akış başlatmaz.
settings-radio-copies-last-session-s-protocol = Son oturumun protokol izini bir hata raporu için panoya kopyalar.
settings-radio-wideband-iq-receive-transmit-half = Geniş bant IQ alma ve yayın. Yarı çift yön — USB ağ bağlantısı ikisini birden taşımadığı için bir yayın süresince alma durur. SoapySDR ve libiio gerekmez.
settings-radio-not-yet-verified-against-real = Henüz gerçek donanımda doğrulanmadı. Düzgün çalışmazsa lütfen tanı raporunu gönderin — radyoyla değiş tokuş edilen her komutu ve örnek akışının ilk baytlarını içerir.
settings-radio-flexradio-announces-itself-local-network = Bir FlexRadio, yerel ağda yaklaşık saniyede bir kendini duyurur. Bir yönlendirici ya da VPN üzerinden erişilen bir radyo size hiç yayın yapmaz — onun yerine aşağıya adresini girin.
settings-radio-another-gui-client-has-radio = Başka bir GUI istemcisi bu radyoya sahip ve multiFLEX kapalı. O istemciyi bağlantıdan çıkarın ya da radyoda multiFLEX'i açın.
settings-radio-overrides-selection-above-use-radio = Yukarıdaki seçimi geçersiz kılar. Başka bir alt ağdaki, bir VPN arkasındaki ya da standart dışı bir bağlantı noktasındaki bir radyo için bunu kullanın.
settings-radio-width-spectrum-sdroxide-receives-192 = sdroxide'ın aldığı spektrumun genişliği. 192 kHz, bir DAX IQ akışı için radyonun maksimumudur ve bu arayüzün gösterebileceği en geniş aralıktır.
settings-radio-t-192-khz-widest-dax-iq = 192 kHz, bir FLEX'in göndereceği en geniş DAX IQ akışıdır ve radyo panadaptörün aralığını ona bağlar — yani tüm aralık budur ve buradaki hiçbir ayar onu genişletmez.
settings-radio-dax-iq-channel = DAX IQ kanalı
settings-radio-radio-has-four-change-only = Radyonun dört tane vardır. Bunu yalnızca ağdaki başka bir şey zaten kanal 1'i kullanıyorsa değiştirin — radyo bir kanalı iki kez reddeder.
settings-radio-station-name = İstasyon adı
settings-radio-shown-against-session-radio-s = Radyonun istemci listesinde bu oturuma karşı gösterilir. Radyo bir istemciyi bununla da hatırlar, bu yüzden yeniden adlandırmak radyonun sdroxide'ı yeni biri gibi ele almasına yol açar.
settings-radio-gui-client-id = GUI istemci kimliği
settings-radio-identity-radio-restores-session-s = Radyonun bu oturumun dilimlerini ve panadaptörlerini geri yüklediği kimlik. Boş bırakılırsa istasyon adından türetilir — kararlıdır, ama varsayılan adı tutan her sdroxide'da aynıdır ve radyo bir çiftlemeyi, önceki istemciyi düşürerek çözer. sdroxide bunu ağda fark eder ve tek oturumluk bir kimliğe geçer, bu da geri yüklemeye mal olur; kendi UUID'nizi buraya koyarak bunu koruyun.
settings-radio-optional-e-g-uuid-your = isteğe bağlı, örn. kendi UUID'niz
settings-radio-mirror-radio-s-i-q =
    Radyonun I/Q'sunu panadaptörün merkezi etrafında aynalar. Varsayılan olarak kapalıdır; bir FLEX-6600 bu şekilde doğrulandı.

    Alma sesi hem USB'de *hem* LSB'de anlaşılmazsa ve hiçbir şey çözülmezken şelale tamamen inandırıcı görünüyorsa deneyin — aynalanmış bir akış böyle görünür ve başka hiçbir belirtisi olmayan tek arızadır (sorun #368). Değilse, bunu açmak onu ince olmaktan çıkarıp bariz kılar. Uygula / yeniden bağlan ile etkinleşir.
settings-radio-network-mtu = Ağ MTU
settings-radio-largest-datagram-radio-may-send = Radyonun gönderebileceği en büyük datagram. 1450, SmartSDR'nin kendisinin istediği değerdir. Parçaların düşürüldüğü ve hiç spektrum gelmediği, daha küçük bir MTU'lu bir yol — bir VPN ya da tünel — üzerinde düşürün.
settings-radio-b = {" "}B
settings-radio-checks-radio-answers-without-registering = Radyonun yanıt verdiğini, GUI istemcisi olarak kaydolmadan denetler — bu yüzden zaten çalışan bir SmartSDR oturumunu bozmaz.
settings-radio-wideband-iq-receive-over-dax = DAX üzerinden geniş bant IQ alma, radyonun modüle ettiği ses yayını. Yeniden başlatmadan geçmek için “Apply / reconnect”e basın.
settings-radio-not-yet-verified-against-real-2 = Henüz gerçek donanımda doğrulanmadı. Düzgün çalışmazsa lütfen tanı raporunu gönderin — radyoyla değiş tokuş edilen her protokol satırını içerir.
settings-radio-devices-soapysdr-can-see = SoapySDR'nin görebildiği aygıtlar
settings-radio-waiting-machine-radio-attached-where = Radyonun takılı olduğu makine bekleniyor; modüller orada kurulu. Açılan aygıt, o makinenin config.toml dosyasındaki `device_args` ya da --device'tır.
settings-radio-ask-every-installed-soapysdr-module = Kurulu her SoapySDR modülünün taramasını isteyin. Hiçbir şey açılmaz, bu yüzden alım sırasında güvenlidir — ama bir an sürebilir.
settings-radio-not-enumerated-yet-press-rescan = Henüz listelenmedi — Yeniden tara'ya basın.
settings-radio-no-soapysdr-devices-found-check = SoapySDR aygıtı bulunamadı. Radyonuzun modülünün kurulu olduğunu ve aygıta erişebileceğinizi denetleyin.
settings-radio-not-radio = radyo değil
settings-radio-sound-card-listed-above-if = Yukarıda bir ses kartı, bir SDR imiş gibi listelenmiş. Her frekansı kabul eder ve yok sayar; bu yüzden açıldığında bant yerine ses kartının girişini gösterir. sdroxide bunları kendiliğinden seçmez — ama birini adlandıran bir device_args satırı yine de uygulanır.
settings-radio-which-one-opens-set-device = Hangisinin açılacağı --device ya da config.toml içindeki device_args ile ayarlanır; ikisi de yoksa listelenen ilk radyo kazanır.
settings-radio-no-device = aygıt yok
settings-radio-rig-has-no-software-adjustable = Bu telsizin yazılımla ayarlanabilir kazancı yoktur.
settings-radio-rx-gains = RX kazançları
settings-radio-tx-gains = TX kazançları
settings-radio-antennas = Antenler
settings-radio-remembered-next-start = Bir sonraki başlatma için hatırlanır.
settings-radio-these-controls-appear-once-soapysdr = Bu denetimler bir SoapySDR aygıtı açılınca görünür — aygıtın kendi denetimleridir ve ona sorulması gerekir.
settings-radio-stream = Akış
settings-radio-follow-app-setting = Uygulama ayarını izle
settings-radio-msps = ya da Msps olarak
settings-radio-baseband-filter = Taban bant filtresi
settings-radio-let-driver-choose = Sürücünün seçmesine izin ver
settings-radio-changing-either-reopens-radio = İkisini de değiştirmek radyoyu yeniden açar.
settings-radio-driver-publishes-no-settings-own = Bu sürücünün kendi ayarı yayımlanmaz.
settings-radio-applied-once-remembered-next-start = Hemen uygulanır ve bir sonraki başlatma için hatırlanır.
settings-radio-no-rx-888-found-press = RX-888 bulunamadı — Yeniden tara'ya basın
settings-radio-adc-clock = ADC saati
settings-radio-t-129-6-msps-needs-superspeed = 129,6 Msps bir SuperSpeed bağlantısı ve hızlı bir ana makine gerektirir; 64,8 güvenli varsayılandır. Değiştirmek alıcıyı otomatik olarak yeniden açar; bu bir an sürer ama yeniden başlatma gerektirmez.
settings-radio-panadapter-width = Panadaptör genişliği
settings-radio-hf-only = {"  "}· yalnızca HF
settings-radio-usable-both-front-ends = İki ön uçta da kullanılabilir.
settings-radio-below-vhf-crossover-ordinary-width = VHF geçişinin altında bu sıradan bir genişliktir. Üstünde ise tuner'ın 8 MHz'lik IF'i bu kadar geniş bir pencereye ortalanamaz; canlı spektrum panadaptörün bir yanına, yanında hiçbir şey olmadan oturur.
settings-radio-dial-above-crossover-now-so = Skala şimdi geçişin üstünde, bu yüzden bu genişlik tuner'ın 8 MHz'ini merkez dışında gösteriyor ve yanında ölü bir spektrum var. Panadaptörü doldurmak için genişliği daraltın.
settings-radio-vga-gain = VGA kazancı
settings-radio-ad8370-variable-gain-amplifier-ahead = ADC'den önce AD8370 değişken kazançlı yükseltici.
settings-radio-attenuator = Zayıflatıcı
settings-radio-pe4304-step-attenuator-0-5 = PE4304 adım zayıflatıcı, 0,5 dB adımlarla.
settings-radio-adc-range = ADC aralığı
settings-radio-wide-2-25-vp-p = Geniş (2,25 Vp-p)
settings-radio-selects-adc-s-wider-input = ADC'nin daha geniş giriş aralığını seçer: güçlü yayın sinyalleri için daha fazla pay, zayıf olanlar için daha az sayım. Kapalıyken daha hassas 1,5 Vp-p aralığı seçilir.
settings-radio-dither = Dither
settings-radio-adds-small-dither-signal-ahead = ADC'den önce küçük bir dither sinyali ekler: gürültü tabanına biraz maliyet, sahte içermeyen dinamik aralık kazandırır.
settings-radio-randomiser = Rastgeleleştirici
settings-radio-adc-scrambles-output-so-digital = ADC çıkışını karıştırır; böylece dijital veriyolu ön uca yayın yapmayı bırakır. Sürücü onu çözer. Hata ayıklamıyorsanız açık bırakın. Yeniden bağlanınca uygulanır.
settings-radio-dc-hf-antenna-port = HF anten portunda DC
settings-radio-powers-active-antenna-preamp-down = Aktif bir anteni ya da ön yükselteci koaksiyel kablodan besler.
settings-radio-vhf-tuner-gain = VHF tuner kazancı
settings-radio-r828d-rf-gain-used-above = R828D RF kazancı; otomatik HF/VHF geçişinin üstünde kullanılır. 29 ayrık adım; en yakın olanı kullanılır.
settings-radio-auto = Otomatik
settings-radio-let-tuner-run-own-lna = Tuner'ın kendi LNA ve karıştırıcı döngülerini çalıştırmasına izin verin.
settings-radio-bias-tee-vhf = Bias tee (VHF)
settings-radio-dc-vhf-antenna-port = VHF anten portunda DC
settings-radio-clock-trim = Saat ayarı
settings-radio-corrects-reference-oscillator-applied-when = Referans osilatörünü düzeltir. Değer bırakıldığında uygulanır — bir sürüklemenin her pikselinde yeniden açmak alıcıyı yüzlerce kez yeniden başlatırdı.
settings-radio-receive-only-below-adc-s = Yalnızca alma. ADC'nin Nyquist sınırının altında anten doğrudan örneklenir ve yeniden ayar anlıktır, çünkü donanımda bir aşağı dönüştürücü yoktur — ADC'nin tüm akışı ana bilgisayarda tabanbanda çevrilir. Bunun üstünde alıcı, R828D tuner'ına ve VHF SMA'ya otomatik olarak geçer; bu yüzden iki anten de bağlı olmalıdır. VHF, tuner'ın IF'inin sığması için 32,4 Msps ya da daha yüksek bir ADC saati gerektirir ve 48 Msps'nin altındaki saatlerde iki aralık arasında hiçbir şeyin ulaşamadığı bir boşluk vardır. Buradaki her ayar hemen uygulanır — basılacak bir Uygula düğmesi yoktur.
settings-radio-device = Aygıt
settings-radio-no-devices-press-rescan = aygıt yok — Yeniden tara'ya basın
settings-radio-six-rates-are-six-different =
    Altı hız, bir yazmacın değil altı farklı FPGA görüntüsüdür — bu yüzden ELAD'ın protokolünde hiçbir şey aralarından seçim yapmaz ve bu ayarın ne anlama geldiği modele bağlıdır.

    FDM-S1 / FDM-S2: bir komut. FPGA'ları bilgisayar tarafından her açılışta yüklenir ve sdroxide, bu hızın görüntüsünü yerleştirmek için ELAD'ın kendi elad-firmware yükleyicisini çalıştırır. Bu yükleyici kurulu değilse bir örnekleyici hiç örnek göndermez ve sdroxide bunu ekranda söyler.

    FDM-DUO: bir komut değil. Radyo kendi FPGA'sını başlatır ve açıldığı ya da FDM-SW2'nin en son bıraktığı hızda gelir (yeni birinde 192 kHz); bu ayar yalnızca akışın NASIL OKUNACAĞINI söyler. Yanlış ayarlarsanız panadaptör yalnızca yanlış genişlikte olur ve içindeki her frekans buna göre ölçeklenir.

    Akışın gerçek hızı, başladıktan birkaç saniye sonra ölçülür ve bir uyumsuzluk ekranda bildirilir. Uygula ile etkinleşir.
settings-radio-fdm-s-fpga-image-loaded = FDM-S: yüklenen FPGA görüntüsü — FDM-DUO: akışın nasıl okunduğu
settings-radio-receiver-s-input-pad-one = Alıcının giriş pedi — bir adım, içeri ya da dışarı. Ana penceredeki Kazanç kaydırıcısıyla aynı denetim.
settings-radio-pre-selection-filters = Ön seçim filtreleri
settings-radio-low-pass-bank-front-adc = ADC'nin önündeki alçak geçiren filtre bankası. Onu atlamak en geniş görünümü verir ama güçlü bant dışı sinyallerin yakınında en kötü davranışı — birkaç mil uzaktaki bir yayın vericisi bandın her yerine görüntüler düşürür. Bilerek filtrelenmiş aralığın dışında dinlemiyorsanız açık bırakın.
settings-radio-filters-circuit = Devredeki filtreler
settings-radio-rig-control-fdm-duo-only = Telsiz denetimi — yalnızca FDM-DUO
settings-radio-transceiver-s-cat-usb-port = Telsizin CAT USB bağlantı noktası; alma kablosundan ayrı bir kablo. FDM-S1 ya da FDM-S2'de bağlantı noktasını ayarsız bırakın, çünkü bunların yoktur — FDM-DUO'da ise yalnızca alma kablosu üzerinden sürmeyi tercih edersiniz; orada ayarlanabilir ve anahtarlanabilir ama hiçbir metre okunamaz.
settings-radio-port-set-radio-s-display = Bağlantı noktası ayarlıyken radyonun ekranı ve bu skala iki yönde de aynı sayıdır: FDM-DUO'nun alma penceresi VFO'su etrafında kurulmuştur; bu yüzden buradan ayarlamak ön paneli hareket ettirir ve ön paneldeki düğmeyi çevirmek buradaki okumayı değiştirir. Panadaptör, skala hareket ettikçe yeniden ortalanır. Bağlantı noktası yoksa — ya da radyonun yanıt verdiği bir bağlantı noktası yoksa — komut verilecek bir VFO yoktur ve ayarlama, radyonun zaten gönderdiği pencerede kalır.
settings-radio-none-usb-control-only = — yok (yalnızca USB denetimi) —
settings-radio-must-match-menu-70-cat = Radyodaki menü 70 “CAT BAUD” ile eşleşmelidir; fabrikadan 38400 gelir. Radyonun bu dört hızı vardır ve başka hiçbiri yoktur; bunların dışında bir hızda açılan bir bağlantı noktası iki yönde de sessizdir — radyo skalayı yok sayar ve neden söylemeden anahtarlamayı reddeder.
settings-radio-how-transmit-keyed-cat-sends = Yayının nasıl anahtarlandığı. “CAT”, radyonun kendi TX komutunu gönderir ve telsizde hiçbir kurulum gerektirmez; RTS ise menü 54 “PTT”nin PTT+RTS olarak ayarlanmasını ister.
settings-radio-where-radio-takes-transmit-audio-2 =
    Radyonun yayın sesini nereden aldığı — telsizin TI komutu; ön panelde menü 32 “TX IN”. Bağlantı noktası açıldığında etkinleştirilir.

    “USB audio”, yayını burada çalıştıran ayardır: FDM-DUO, sdroxide'ın USB ses kartına koyduğu şeyi gönderir. “Microphone”da bırakılmış bir telsiz bunun yerine odayı gönderir ve ekranda bunu söyleyen hiçbir şey yoktur.
settings-radio-which-two-sockets-back-receiver-2 =
    Alıcının arkadaki iki soketten hangisini dinlediği — telsizin AN komutu; ön panelde menü 31 “ANTENNAS” ve ekranındaki “ANT 1 2” göstergesi.

    “RTX”, yayını da taşıyan M tipi soket üzerinde tek bir antenin iki işi birden yapmasıdır. “RX only”, almayı ikinci sokete taşır ve yayını RTX'te bırakır — bir alma anteni, bir döngü ya da bir beverage, huzmenin hâlâ verici üzerinde olduğu.

    Bu, bu bütün alıcıyı taşır: panadaptör, demodülatörler ve telsizin kendi sesinin hepsi burada seçilen soketten gelir.

    Hemen uygulanır ve kontrol bağlantı noktası açıldığında telsizden geri okunur — burada tutulan bir kopya değil, telsizin kendi ayarıdır.
settings-radio-how-often-radio-asked-what-2 = Radyoya ne yaptığının sorulma sıklığı — skalası, modu ve metreleri. Telsizin kendi VFO düğmesini çevirin; okuma bir sorgulama içinde izler. Bu uçtan üretilen kontrol trafiğinin tamamıdır, bu yüzden radyonun sesi kesilirse düşürün.
settings-radio-transmit-audio = Yayın sesi
settings-radio-system-default = — sistem varsayılanı —
settings-radio-fdm-duo-s-own-usb = FDM-DUO'nun kendi USB Audio bağlantı noktası. Sistem varsayılanında bırakılırsa neredeyse hiçbir zaman radyo olmaz — makinenin hoparlörleridir.
settings-radio-press-rescan-list-sound-cards = ses kartlarını listelemek için Yeniden tara'ya basın
settings-radio-copies-last-session-s-trace = Son oturumun izini bir hata raporu için panoya kopyalar: aygıtla değiş tokuş edilen her komutu ve örnek akışının ilk baytlarını.
settings-radio-only-receiving-fdm-duo-has = Bir FDM-DUO'da şimdiye dek gerçek donanımda yalnızca alma çalıştırıldı. Bir şey düzgün çalışmazsa lütfen tanı raporunu bir hata bildirimine ekleyin.
settings-radio-cw-keyed-radio-s-own = CW, radyonun kendi tuşu ya da paletiyle anahtarlanır: FDM-DUO metin kabul eden bir komuta sahip değildir, bu yüzden CW paneli onu anahtarlayamaz.
settings-radio-press-apply-reopen-radio = Radyoyu yeniden açmak için Apply'a basın.
settings-radio-no-receivers-press-rescan = alıcı yok — Yeniden tara'ya basın
settings-radio-which-rates-receiver-has-depends = Bir alıcının hangi hızlara sahip olduğu modele ve yazılıma bağlıdır. Uygula ile etkinleşir.
settings-radio-every-rate-any-hf-model = herhangi bir HF+ modelinin sunduğu her hız — kendininkini görmek için birini bağlayın
settings-radio-receiver-s-own-gain-control = Alıcının kendi kazanç denetimi. Genel dinleme için açık bırakın; ölçüm için zayıflatıcıyı elle ayarlamak üzere kapatın.
settings-radio-agc-threshold = AGC eşiği
settings-radio-high = Yüksek
settings-radio-high-trades-little-sensitivity-headroom = Yüksek, güçlü komşulara karşı pay için biraz hassasiyetten vazgeçer — gece kalabalık bir bantta doğru ayar budur.
settings-radio-front-end-attenuation-gain-0 = Ön uç zayıflatması, bir kazanç olarak — 0 dB hiçbiri demektir. Yalnızca AGC kapalıyken uygulanır.
settings-radio-preamp = Ön yükselteç
settings-radio-hf-low-noise-amplifier-buys = HF düşük gürültülü yükselteç. Hassasiyet kazandırır, ama intermodülasyon pahasına; bu yüzden varsayılan olarak kapalıdır — gerçek bir anten üzerinde genellikle doğru olan da budur.
settings-radio-lna = LNA açık
settings-radio-frequency-calibration = Frekans kalibrasyonu
settings-radio-parts-per-billion-receiver-s = Milyarda kısım — bu alıcının kendi birimi, bir RTL-SDR'nin kullandığı ppm'den bin kat ince. Buradaki hiçbir şey alıcının belleğine yazılmaz: bu, saklanan değeri yalnızca oturum için geçersiz kılar.
settings-radio-use-receiver-s-stored-value = Alıcının saklı değerini kullan
settings-radio-ppb = {" "}ppb
settings-radio-feed-dc-up-coax = Koaksiyel kabloya DC besle
settings-radio-not-every-hf-has-one = Her HF+'ın bu özelliği yoktur; bunun olmadığı bir alıcıda bu hiçbir şey yapmaz.
settings-radio-host-dsp = Ana bilgisayar DSP'si
settings-radio-image-balancer-zero-if-offset = Görüntü dengeleyici, sıfır-IF kayması ve ince ayar osilatörü. Yalnızca ham donanım çıktısını görmek için kapatın — kapalıyken ayna görüntüsü sıfır-IF hızlarında görünür ve skala yalnızca en yakın kilohertze kadar doğrudur.
settings-radio-correct-image-fine-tune = Görüntüyü düzelt ve ince ayar yap
settings-radio-copies-last-session-s-trace-2 = Son oturumun izini bir hata raporu için panoya kopyalar: alıcıyla değiş tokuş edilen her komutu ve örnek akışının ilk baytlarını.
settings-radio-receive-only-no-soapysdr-no = Yalnızca alma. SoapySDR ya da libairspyhf gerekmez. Sentezleyicinin tabanının altında ana bilgisayarın osilatörü ayarlamayı yapar; alıcının VLF'ye ulaşma yolu budur. Alıcı ve örnekleme hızı Uygula ile etkinleşir; diğer her şey değiştirdikçe uygulanır.
settings-radio-not-yet-verified-against-real-3 = Henüz gerçek donanımda doğrulanmadı. Düzgün çalışmazsa lütfen tanı raporunu gönderin — alıcıyla değiş tokuş edilen her komutu ve I/Q çiftleri olarak çözülen örnek akışının ilk baytlarını içerir.
settings-radio-no-airspy-r2-mini-found = Airspy R2 ya da Mini bulunamadı — Yeniden tara'ya basın
settings-radio-read-receiver = bu alıcıdan okunur
settings-radio-r2-mini-offer-different-rates = bir R2 ve bir Mini farklı hızlar sunar ve biri açılana dek birbirinden ayırt edilemez
settings-radio-rate-you-get-receiver-s = Bu, elde ettiğiniz hızdır. Alıcının ADC'si bunun iki katında çalışır — gerçek bir sinyali sayısallaştırır ve sdroxide ondan ana bilgisayarda karmaşık taban bandı üretir.
settings-radio-gain-curve = Kazanç eğrisi
settings-radio-step-along-curve-above-not = Yukarıdaki eğri boyunca bir adım, bir dB rakamı değil — tuner'ın LNA'sı, karıştırıcısı ve VGA'sı birlikte hareket eder ve her adımın değeri eğriye ve banda bağlıdır. 0 sessiz uçtur.
settings-radio-tuner-agc = Tuner AGC
settings-radio-lna-2 = LNA
settings-radio-hands-lna-tuner-s-own = LNA'yı tuner'ın kendi döngüsüne verir ve eğriyi geçersiz kılar.
settings-radio-mixer = Karıştırıcı
settings-radio-same-mixer-stage = Karıştırıcı katı için de aynısı.
settings-radio-loop-running-gain-slider-no = Bir döngü çalışırken kazanç kaydırıcısı artık ona ait olan katı ayarlamaz — döngü onun üstüne yazar.
settings-radio-dc-antenna-port = Anten portunda DC
settings-radio-t-12-bit-packing = 12 bit paketleme
settings-radio-third-less-usb-traffic-leave = USB trafiğinin üçte biri kadar daha az. Açık bırakın: bu bir USB 2.0 aygıtıdır ve en yüksek hız, paketlenmemiş 40'a karşı 30 MB/s'dir. Yeniden bağlanınca uygulanır.
settings-radio-dc-removal = DC kaldırma
settings-radio-remove-adc-s-offset = ADC'nin ofsetini kaldır
settings-radio-turn-off-see-raw-hardware = Ham donanım çıktısını görmek için kapatın. Spurun nereye gittiğini bilmek faydalıdır: ofset, aralığın merkezinde değil kenarına düşer, çünkü sinyal yol boyunca örnekleme hızının dörtte biri kadar çevrilir.
settings-radio-every-command-exchanged-receiver-sample = Alıcıyla değiş tokuş edilen her komut, örnekleme hızı aritmetiği ve I/Q olarak çözülen ilk örnekler.
settings-radio-receive-only-24-1800-mhz = Yalnızca alma, 24–1800 MHz. SoapySDR ya da libairspy gerekmez. Alıcı, örnekleme hızı ve paketleme Uygula ile etkinleşir; diğer her şey değiştirdikçe uygulanır.
settings-radio-not-yet-verified-against-real-4 = Henüz gerçek donanımda doğrulanmadı. Düzgün çalışmazsa lütfen tanı raporunu gönderin — alıcıyla değiş tokuş edilen her komutu, hız aritmetiğini ve I/Q çiftleri olarak çözülen ilk örnekleri içerir.
settings-radio-no-hydrasdr-rfone-found-press = HydraSDR RFOne bulunamadı — Yeniden tara'ya basın
settings-radio-these-are-rates-receiver-turned = bunlar, bu alıcının sahip çıktığı hızlar
settings-radio-receiver-only-reports-three-these = alıcı bunların yalnızca üçünü bildirir; kalanı yazılımın alternatif tablosundadır ve eski bir derleme bunları taşımayabilir
settings-radio-rf-input = RF girişi
settings-radio-antenna-sma-only-socket-bias = Anten SMA'sı — arkasında bias tee bulunan tek soket.
settings-radio-cable-socket-no-bias-tee = Bir kablo soketi. Burada bias tee yoktur; donanım bunu yalnızca ANT'a koyar.
settings-radio-powers-active-antenna-preamp-down-2 = Aktif bir anteni ya da ön yükselteci koaksiyel kablodan besler. Yalnızca ANT soketinin bunu yapabilen bir devresi vardır — iki kablo portu düz girişlerdir.
settings-radio-third-less-usb-traffic-leave-2 = USB trafiğinin üçte biri kadar daha az. Açık bırakın: bu bir USB 2.0 aygıtıdır ve en yüksek hız, paketlenmemiş 48'e karşı 36 MB/s'dir. Yeniden bağlanınca uygulanır.
settings-radio-one-these-boards-1d50-60a1 = Bu kartlardan biri 1d50:60a1 üzerinde; HydraSDR'nin prototiplerinin Airspy R2 ve Mini ile paylaştığı USB kimliği. sdroxide, açtıktan sonra yazılımı denetler ve yanlış arabirim seçilmişse her iki yönde de bunu söyler.
settings-radio-receive-only-24-1800-mhz-2 = Yalnızca alma, 24–1800 MHz. SoapySDR ya da libhydrasdr gerekmez. Alıcı, örnekleme hızı ve paketleme Uygula ile etkinleşir; diğer her şey değiştirdikçe uygulanır.
settings-radio-re-list-fobos-sdrs-no = Fobos SDR'leri yeniden listele. Hiçbir aygıt açılmaz, bu yüzden alım sırasında basmak güvenlidir.
settings-radio-no-fobos-sdr-found-press = Fobos SDR bulunamadı — Yeniden tara'ya basın
settings-radio-input-2 = Giriş
settings-radio-through-tuner-lna-vga-mixer = Tuner üzerinden: LNA, VGA ve karıştırıcının kendi frekans aralığı.
settings-radio-direct-sampling-hf1-antenna-input = HF1 anten girişinde doğrudan örnekleme — tuner yok, LNA/VGA kazancı yok, tamamen yazılımla ayarlanır.
settings-radio-direct-sampling-hf2-antenna-input = HF2 anten girişinde doğrudan örnekleme, HF1 ile aynı.
settings-radio-both-real-adc-channels-once = Her iki gerçek ADC kanalı birlikte, aşağıdaki çeşitlilik filtresiyle birleştirilmiş — bir gürültü kaynağı sıfırlanmış ya da iki sönümlenen yol birleştirilmiş.
settings-radio-direct-sampling-tuner-so-lna = Doğrudan örnekleme — tuner (ve dolayısıyla LNA/VGA kazancı) yol üzerinde değil. Aşağıdaki hız, bir yazılım aşağı dönüştürücüsü için bir hedeftir, donanım ayarı değildir; bu yüzden ulaşılan hız ondan farklı olabilir.
settings-radio-receiver-s-own-reported-rates = bu alıcının kendi bildirdiği hızlar
settings-radio-measured-one-real-unit-connect = tek bir gerçek birimde ölçüldü — bu alıcıyı bağlayın ve kendi listesini görmek için Yeniden tara'ya basın
settings-radio-clock-source = Saat kaynağı
settings-radio-external-reference = Harici referans
settings-radio-diversity-taps = Çeşitlilik tap'leri
settings-radio-taps = {" "}tap
settings-radio-one-tap-gain-phase-null = Bir tap bir kazanç ve bir fazdır — bir frekansta, iki yanında kötüleşen bir boşluk; bir analog fazlayıcının yapabildiği tek şey budur. Her ek tap, filtrenin eşitleyebildiği HF1 ile HF2 arasındaki yol farkının bir örnekleme periyodunu sağlar; çentiği tüm genişlik boyunca sessiz bir banda çeviren de budur.
settings-radio-receive-only-rf-port-tuner = Yalnızca alma. RF portu (tuner), HF1/HF2 (doğrudan örnekleme, yazılımda çözülür) ve HF1+HF2 (ikisi birden, aşağıdaki çeşitlilik filtresiyle birleştirilmiş) hepsi çalışır. Alıcı, giriş ve örnekleme hızı Uygula ile etkinleşir; kazanç, saat kaynağı ve tap'ler değiştirdikçe uygulanır — çeşitlilik filtresinin modu, uyum hızı ve tutma, bu radyo seçildiğinde ana penceredeki kendi DIV kutusundadır.
settings-radio-no-hackrf-found-press-rescan = HackRF bulunamadı — Yeniden tara'ya basın
settings-radio-front-end-amplifier-8-db = Ön uç yükselteci, 8 dB adımlarla. Hassasiyeti değiştiren katmandır — ve gerçek bir anten üzerinde ilk aşırı yüklenen katmandır.
settings-radio-baseband-amplifier-after-mixer-2 = Karıştırıcıdan sonra taban bant yükselteci, 2 dB adımlarla. LNA'ya uzanmadan önce zayıf bir sinyal için bunu yükseltin.
settings-radio-rf-amp = RF yükselteci
settings-radio-one-switch-shared-transmit-setting = Tek bir anahtar; aşağıdaki yayın ayarıyla paylaşılır — radyo, girdiği yönün ait olduğu olanı uygular. Gerçek bir anten üzerinde genellikle kapalı olması doğrudur.
settings-radio-hackrf-pro-chooses-itself-three = Bir HackRF Pro bunu kendisi seçer — örnekleme hızının dörtte üçü, FPGA'da filtrelenmiş — ve ana bilgisayarın istediği her şeyi yok sayar.
settings-radio-leave-automatic-choosing-one-too = Otomatik'te bırakın. Çok dar birini seçmek yalnızca bant kenarlarını yumuşatmaz — sinyalinizden DC tepesini uzak tutan LO kaymasını geri çeker.
settings-radio-about-3-v-50-ma = Koaksiyel kablodan yaklaşık 50 mA'de 3 V, aktif bir anten ya da bir ön yükselteç için. Yalnızca bir HackRF One ya da Pro; Jawbreaker ve rad1o'nun böyle bir devresi yoktur.
settings-radio-remove-dc-mirror-image = DC'yi ve ayna görüntüsünü kaldır
settings-radio-zero-if-radio-so-own = Bu bir sıfır-IF radyodur; bu yüzden kendi LO sızıntısı orta aralıkta yer alır ve karıştırıcının kuadratür hatası onun üzerine bir ayna görüntüsü koyar. Bunu kapatmak ham donanım çıktısını gösterir; sürücü sorununu DSP sorunundan ayırmanın hızlı yoludur.
settings-radio-corrects-reference-oscillator = Referans osilatörünü düzeltir.
settings-radio-transmit = Yayın
settings-radio-transmitter = Verici
settings-radio-off-default-while-off-backend = Varsayılan olarak kapalı. Bu kapalıyken arka uç hiçbir yayın kanalı yayımlamaz; bu yüzden hiçbir şey radyoyu anahtarlayamaz. Yeniden bağlanınca uygulanır.
settings-radio-tx-vga-gain = TX VGA kazancı
settings-radio-transmit-driver-amplifier-drive-applied = Yayın sürücü yükselteci. Sürüş bu katın önünde sayısal olarak uygulanır; bu yüzden sürüşü yüksek tutun ve çıkış seviyesini burada ayarlayın — sürüşü kısmak, DAC'ı tam ölçeğin bir kısmında çalıştırır ve intermodülasyonu artırır.
settings-radio-same-switch-receive-setting-above = Yukarıdaki alma ayarıyla aynı anahtar; radyo yön değiştirdiğinde uygulanır.
settings-radio-transmit-armed-into-dummy-load = Yayın hazır. Ölçülene dek yapay yüke: bir HackRF'in harmonikleri, bulunduğunuz band için harici bir alçak geçiren filtre gerektirecek kadar güçlüdür ve yarı çift yöndür, bu yüzden her yayın süresince alma durur.
settings-radio-every-command-exchanged-radio-session = Bu oturumda radyoyla değiş tokuş edilen her komut, sırasıyla; her tuşlamanın etrafındaki sıra dahil.
settings-radio-dc-7-25-ghz-half = DC – 7,25 GHz, yarı çift yön. SoapySDR, libusb ya da libhackrf gerekmez. Radyo ve örnekleme hızı Uygula ile etkinleşir; diğer her şey değiştirdikçe uygulanır. Bant kenarları yazılımın kendisinindir: bir HackRF, 1 MHz – 6 GHz için belirtilmiştir (bir Pro'da 100 kHz) ve bunun dışında ağır biçimde zayıflatılır, ama orada da ayarlanır.
settings-radio-ask-sdrplay-api-service-device = SDRplay API servisinden aygıt listesini isteyin. Hiçbir şey açılmaz, bu yüzden alım sırasında basmak güvenlidir.
settings-radio-no-rsps-press-rescan-needs = RSP yok — Yeniden tara'ya basın (SDRplay API servisi gerekir)
settings-radio-both-tuners-running-api-fixes = Her iki tuner çalışırken API, ADC'yi 6 MHz'e sabitler ve düşük bir IF'ten 2 Msps verir — bu yüzden en geniş aralık 2 Msps'dir ve daha dar olanlar onun seyreltilmiş halidir. Uygula ile etkinleşir.
settings-radio-rates-below-2-msps-run = 2 Msps'nin altındaki hızlar ADC'yi 2 Msps'de çalıştırır ve servisteki seyreltmeyle elde edilir. Uygula ile etkinleşir.
settings-radio-reduced-adc-resolution = {"  "}(azaltılmış ADC çözünürlüğü)
settings-radio-if-bandwidth = IF bant genişliği
settings-radio-tuner-s-analog-filter-auto = Tuner'ın analog filtresi. Otomatik, örnekleme hızına sığan en geniş olanı seçer. Uygula ile etkinleşir.
settings-radio-rsp-s-own-if-gain = RSP'nin kendi IF kazanç döngüsü; API servisi tarafından çalıştırılır. Kapalı, IF kazanç kaydırıcısını size geri verir — ölçüm ve zayıf sinyal dijital modları için ayar.
settings-radio-agc-set-point = AGC ayar noktası
settings-radio-signal-level-loop-holds-adc = Döngünün ADC'yi tuttuğu sinyal seviyesi. Daha düşük, kanal dışı sinyaller için daha fazla pay bırakır. Ne kadar düşebileceği örnekleme hızına bağlıdır — 8,064 Msps'nin üstünde dönüştürücünün verebileceği pay daha azdır, 9,216 Msps'nin üstünde ise daha da az.
settings-radio-rsp-s-native-gain-unit = RSP'nin yerel kazanç birimi: 20 dB maksimum kazançtır (aşağıdaki genişletilmiş aralıkla 0), 59 dB minimumdur. Hemen uygulanır. AGC çalışırken yok sayılır — o zaman değeri döngü sahiplenir ve S-metre üzerinde durduğu değeri gösterir.
settings-radio-extended-if-range = Genişletilmiş IF aralığı
settings-radio-lets-if-gain-reduction-go = IF kazanç azaltmasının 20 dB'nin altına, 0'a kadar inmesine izin verir — alıcının sahip olduğu son 20 dB kazanç. Kapalı, API'nin kendi varsayılanıdır ve sıradan dinleme için doğru olanıdır, çünkü aralığın alt ucu bir RSP'nin en kolay aşırı yüklendiği yerdir. LNA zaten 0'dayken zayıf sinyaller için açmaya değer. AGC ayar noktasını da 0 dBFS'e çıkarmaya izin verir. Hemen uygulanır.
settings-radio-allow-if-gain-reduction-below = IF kazanç azaltmasına 20 dB'nin altında izin ver
settings-radio-front-end-attenuation-steps-0 = Adımlarla ön uç zayıflatması: 0 maksimum kazançtır, yukarı her adım daha fazla zayıflatma devreye alır. Bazı bantların daha az adımı vardır — sürücü sınırlar ve seçiminizi, geri ayar yaptığınızda kullanılmak üzere saklar; buradaki kaydırıcı, radyonun olduğu bandın alacağı yerde biter. Başka bir bantta yapılan daha yüksek bir seçim korunur ve yanında hâlâ gösterilir. Hemen uygulanır.
settings-radio-t-0-max-gain = 0 = maksimum kazanç
settings-radio-reference-error-parts-per-million-2 = Milyonda kısım cinsinden referans hatası; aygıtın kendisi tarafından uygulanır. Hemen uygulanır.
settings-radio-radio-s-tuner = Bu radyonun tuner'ı
settings-radio-tuner = Tuner
settings-radio-which-rspduo-s-two-tuners = Bu radyonun dinlediği RSPduo'nun iki tuner'ından hangisi. Diğeri, onun için yapılandırılmış radyoya aittir — o birine aynı alıcıyı ve diğer tuner'ı verin. Uygula ile etkinleşir.
settings-radio-which-rspduo-s-two-tuners-2 = RSPduo'nun iki tuner'ından hangisinin dinlediğiniz anteni taşıdığı. Diğeri ikinci anteni taşır. Uygula ile etkinleşir.
settings-radio-which-rspduo-s-two-tuners-3 = RSPduo'nun iki tuner'ından hangisinin çalıştırılacağı (bir seferde biri). Uygula ile etkinleşir.
settings-radio-applies-immediately = Hemen uygulanır.
settings-radio-fm-broadcast-notch = FM yayın çentiği
settings-radio-hardware-notch-over-88-108 = 88–108 MHz yayın bandı üzerinde donanım çentiği; yerel bir verici diğer her şeyi aşırı yüklediğinde. Hemen uygulanır.
settings-radio-dab-notch = DAB çentiği
settings-radio-hardware-notch-over-165-230 = 165–230 MHz DAB bandı üzerinde donanım çentiği. Hemen uygulanır.
settings-radio-hdr-mode = HDR modu
settings-radio-enable-below-2-mhz = 2 MHz'in altında etkinleştir
settings-radio-rspdx-s-high-dynamic-range =
    RSPdx'in LF/MF için yüksek dinamik aralık yolu. Hemen uygulanır.

    Henüz çalışmaz: denenen tek RSPdx'te açıldığında alıcı susar — denenen her merkezde, filtrede, portta, kazançta ve hızda. Modun ihtiyaç duyduğu şey belgelenmiş API yüzeyinde yoktur ve onu bulmak birinin sonraki işidir. Bakan siz değilseniz bunu kapalı bırakın.

    Bilinen şudur: skalayı izleyen bir mod değildir. Yolun sabit bir analog filtresi vardır ve yalnızca aşağıdaki filtrenin altında listelenen merkezlerde kurulmuştur; bu yüzden başka bir yere ayar yapmak, gerisi çalışsa bile işe yaramazdı.
settings-radio-hdr-filter = HDR filtresi
settings-radio-feed-4-7-v-dc = Koaksiyel kabloya ~4,7 V DC besle
settings-radio-second-tuner = İkinci tuner
settings-radio-rspduo-two-whole-tuners-one = Bir RSPduo, tek bir kart üzerinde iki tam tuner'dır; tek bir referanstan saatlenir. İkisini de çalıştırın: spanlerini aynı anda aynı saatten duyarlar — bu, üzerlerindeki iki anteni tutarlı yapar, bağıl faz rastlantıyla değil besleme hatlarıyla belirlenir ve çifti birleştirmeye izin verir. Ya da ayrı bırakın: tuner'lar ayrı ayrı ayarlanır, yani öteki kendi bandında ikinci bir radyo olabilir.
settings-radio-run-both-tuners = İki tuner'ı da çalıştır
settings-radio-puts-rspduo-api-s-dual = RSPduo'yu API'nin çift tuner moduna alır; burada ADC 6 MHz'e sabittir ve en geniş aralık 2 Msps'dir. Mod, aygıt açılırken seçildiği için Uygula ile etkinleşir.
settings-radio-used = Kullanım amacı
settings-radio-what-other-tuner-takes-effect = Öteki tuner'ın ne için kullanıldığı. Uygula ile etkinleşir — iki tuner'ı birden çalıştırmak gibi, bu da kart açılırken seçilir.
settings-radio-add-second-radio-settings-radio = İkinci bir radyo ekleyin (Ayarlar → Radyo → +), ona bu aynı alıcıyı ve RSPduo'nun öteki tuner'ını verin ve onu da iki tuneri çalıştıracak şekilde ayarlayın — kartı hangi radyo açarsa açsın çift tuner moduna alır, bu yüzden ikisinin de bunu beklemesi gerekir. İkisi bağımsız ayarlanır: biri bir sekmede HF, diğeri VHF, tek bir karttan. Tek bir ADC saatini paylaşırlar; bu yüzden kartı hangi radyo açmış olursa olsun istenen örnekleme hızında çalışırlar ve hiçbiri yayın yapamaz.
settings-radio-what-do = Ne yapılacağı
settings-radio-lna-state-2 = LNA durumu
settings-radio-second-tuner-s-own-front = İkinci tuner'ın kendi ön uç zayıflatması. İki anten de yaklaşık aynı gürültü tabanını göstersin diye ayarlanır: her şey bunun üzerine kuruludur, çünkü birleştirme iki dalı gürültülerine göre ağırlıklandırır ve aşırı yüklenmiş bir ikinci ön uç, filtreye girişimin bozulmuş bir kopyasını verir — bozulmamış birinden çıkarılamaz. Hemen uygulanır.
settings-radio-if-gain-reduction-2 = IF kazanç azaltması
settings-radio-second-tuner-s-if-gain = İkinci tuner'ın IF kazancı, RSP'nin yerel biriminde: 20 dB maksimum kazançtır. AGC çalışırken yok sayılır — ve filtre sabit bir kazanç ister, bu yüzden tutmak istediğiniz bir boşluk için AGC'yi kapatmaya değer.
settings-radio-filter-length = Filtre uzunluğu
settings-radio-one-tap-gain-phase-null-2 = Bir tap bir kazanç ve bir fazdır — bir frekansta, iki yanında kötüleşen bir boşluk; bir analog fazlayıcının yapabildiği tek şey budur. Her ek tap, filtrenin eşitleyebildiği iki anten arasındaki yol farkının bir örnekleme periyodunu sağlar; çentiği tüm genişlik boyunca sessiz bir banda çeviren de budur.
settings-radio-adaptation-2 = Uyarlama
settings-radio-slow-steady-left-converging-inside = Solda yavaş ve sabit, saniyenin küçük bir kesrinde yakınsayan ve sağda gözle görülür biçimde avlanan. Boşluğu bulmak için hızlı başlayın, sonra tutun.
settings-radio-stop-filter-moving-reach-moment = Filtrenin hareket etmesini durdurun. Bir boşluk belirir belirmez buna uzanın: uyarlanmaya bırakılmış bir filtre, en yüksek olan şeye yeniden yönelir; sessiz bir bantta bu, dinlediğiniz istasyondur.
settings-radio-restart = Yeniden başlat
settings-radio-zero-filter-find-null-again = Filtreyi sıfırlayın ve boşluğu yeniden bulun.
settings-radio-nothing-here-can-tell-wanted = Burada istenen bir sinyali istenmeyenden hiçbir şey ayırt edemez — filtre yalnızca iki antenin ortak olanını bilir. İptal'de, ikinci anten gürültü kaynağını duymak ve bandın mümkün olduğunca azını duymak ister; yoksa istasyonu da sadakatle iptal eder. Birleştir'de ikisi de aynı istasyonu duymak ister. Boşluğun ne kadar derinleştiği, her birkaç saniyede bir günlüğe yazılır.
settings-radio-not-yet-verified-against-rspduo = Bir RSPduo'ya karşı henüz doğrulanmadı: buradaki çift tuner işletimi, donanımda ölçülmek yerine SDRplay'in API'sinden yazılmıştır.
settings-radio-no-receiver-listed-check-against = Buna karşı denetlenecek bir alıcı listelenmiyor — Yeniden tara'ya basın ya da gerçekte neyin açıldığını görmek için günlüğü okuyun.
settings-radio-receive-only-1-khz-2 = Yalnızca alma, 1 kHz–2 GHz. Satıcının SDRplay API servisini (sdrplay.com/api) gerektirir — orijinal RSP1'den sonraki RSP'lerin açık bir protokolü yoktur. Aygıt, örnekleme hızı, bant genişliği ve RSPduo tuner'ı Uygula ile etkinleşir; diğer her şey değiştirdikçe uygulanır.
settings-radio-board = Kart
settings-radio-first-one-found-2 = Bulunan ilk
settings-radio-receive-chain = Alma zinciri
settings-radio-both-chains-tune-together-they = İki zincir birlikte ayarlanır — tek bir sentezleyiciyi paylaşırlar — ama ayrı soketlerdeki ayrı ön uçlardır. Anteninizin bağlı olduğunu seçin; bu, bir zincir HF eşleştirme modifikasyonunu almışken diğeri standart olduğunda uzanacağınız ayardır.
settings-radio-receive-gain = Alma kazancı
settings-radio-one-combined-figure-which-limesuite = LimeSuite'in LNA, TIA ve PGA'ya kendisinin dağıttığı tek birleşik rakam. Tam desibel alır; bunun daha inceleri kesilir.
settings-radio-receive-port = Alma portu
settings-radio-automatic-follows-frequency-lnal-low = Otomatik frekansı izler: LNAL düşük, LNAH yüksek — aşağıda bir LimeRFE bağlı değilse, ki o zaman bir kablo bir sokete bağlanır ve her frekansta LNAW olur. Kablonuz bu değilse bağlı olduğu soketi adlandırın. Soketin hangi zincire ait olduğu yukarıdaki seçicidir.
settings-radio-t-0-follows-sample-rate-worth = 0 örnekleme hızını izler. Orada bırakmaya değer: aralığın dörtte birinden dar bir filtre, sıfır-IF LO kaymasını sessizce geri çeker ve LO sızıntısını dinlediğiniz sinyalin üstüne geri koyar.
settings-radio-following-rate = hızı izliyor
settings-radio-corrections = Düzeltmeler
settings-radio-host-iq-dc-correction = Ana bilgisayar IQ / DC düzeltmesi
settings-radio-adaptive-image-dc-removal-side = Bu tarafta uyarlanabilir görüntü ve DC kaldırma, çipin kendi kalibrasyonuna ek olarak. Kapatmak, sürücü sorununu DSP sorunundan ayırmanın tek tıkla yoludur.
settings-radio-calibrate-automatically = Otomatik kalibre et
settings-radio-runs-chip-s-own-dc = Radyo açıldığında ve skala yeni bir bantta ya da farklı bir soketteyken yeniden yerleşince çipin kendi DC ofseti ve görüntü kalibrasyonunu çalıştırır. Bu sayılar tek bir frekansta ölçülür ve başka yerde yanlıştır; aralığın ortasında oturan bir taşıyıcı genellikle böyledir. Her seferinde yaklaşık bir saniyeye mal olur.
settings-radio-calibrate-now = Şimdi kalibre et
settings-radio-stalls-receiver-better-part-second = Alıcıyı saniyenin büyük kısmı kadar durdurur.
settings-radio-second-aerial = İkinci anten
settings-radio-socket = Soketi
settings-radio-same-first = Birincisiyle aynı
settings-radio-gain-2 = Kazancı
settings-radio-set-low-coupled-sample-your = Bunu DÜŞÜK ayarlayın. Kendi vericinizin kuplajlı örneği güçlü bir sinyaldir ve sıkıştırmaya sürülen bir geri besleme zinciri amplifikatörün eğrisini yanlış ölçer — düzeltmeye kendi bozulmasını öğretir. Alttan başlayın ve kuplajın zayıflatıcısını kullanın.
settings-radio-set-so-both-aerials-show = İki anten de yaklaşık aynı gürültü tabanını göstersin diye ayarlayın. Her şey bunun üzerine kuruludur: birleştirme iki dalı gürültülerine göre ağırlıklandırır ve sıkıştırmaya sürülen ikinci bir zincir, filtreye girişimin bozulmuş bir kopyasını verir — bozulmamış birinden çıkarılamaz.
settings-radio-table-steps-2 = Tablo adımları
settings-radio-how-finely-correction-follows-amplifier = Düzeltmenin amplifikatörün eğrisini ne kadar ince izlediği. Daha fazla adım daha keskin bir dirseği izler, ama her biri içine düşen örneklerden öğrenilmelidir — ve bir konuşma genliği histogramının tepesi incedir. Otuz iki, bir HF amplifikatörünün gerçekte sahip olduğu düzgün eğri için yeterlidir. Değiştirmek düzeltmeyi yeniden başlatır.
settings-radio-how-hard-each-block-feedback = Her geri besleme bloğunun düzeltmeyi ne kadar sert hareket ettirdiği. Bir amplifikatörün eğrisi değişmez, bu yüzden acele etmeye gerek yoktur — orta, birkaç yayının gürültüsünü ondan ortalar.
settings-radio-keep-correction-curve-learned-clean = Düzeltmeyi olduğu gibi tutun. Temiz bir yayında öğrenilmiş bir eğri tutmaya değer ve amplifikatör bir sonrakine kadar değişmemiş olur.
settings-radio-forget-correction-learn-again = Düzeltmeyi unutun ve yeniden öğrenin.
settings-radio-directional-coupler-amplifier-s-output = Amplifikatörün çıkışındaki yönlü bir kuplaj bu zincire girer ve verici, geri gelenle göndermek istediğini karşılaştırır — sonra farkın tersini gönderir, böylece amplifikatörden çıkan düz olur. Başkalarının QSO'larında yirmi küsur desibel daha az intermodülasyon; amplifikatör gücünü korurken. Geri besleme iletimle hizalanana dek düzeltme birlikte kalır, bu yüzden bağlı olmayan bir kuplaj hiçbir şeye mal olmaz; ve dönüştürücüden tam ölçeğin üstünü asla isteyemez, bu yüzden saçma okuyan bir geri besleme yolu hiçbir şeyi aşırı süremez. Nasıl gittiği, siz yayın yaparken günlüğe yazılır.
settings-radio-transmit-not-armed-so-there = Yayın hazır değil, bu yüzden düzeltilecek bir şey yok.
settings-radio-off-interface-publishes-no-transmit = Bu kapalıyken arabirim hiçbir yayın kanalı yayımlamaz; bu yüzden hiçbir şey radyoyu anahtarlayamaz.
settings-radio-transmit-gain = Yayın kazancı
settings-radio-transmit-port = Yayın portu
settings-radio-board-s-two-transmit-sockets = Kartın iki yayın soketi, aynı ağa iki jak değil, iki farklı eşleştirme ağıdır: BAND1 (TX_1) 30 MHz'den 1,9 GHz'e kadar taşır ve BAND2 (TX_2) 13 cm portudur. 23 cm'nin altındaki bir amatör bandı BAND2'den anahtarlamak, yayını neredeyse hiçbirini geçirmeyen bir eşleştirme ağına sokar — her ayar doğru okunur, sürüş ne ayarlandıysa odur ve güç metresi sıfırda kalır. Bunu Otomatik'te bırakın; skalayı izler.
settings-radio-limesdr-transmits-about-100-khz = Bir LimeSDR, kendi filtrelemesi olmadan yaklaşık 100 kHz'den 3,8 GHz'e kadar yayın yapar. Bir alçak geçiren filtre, uygun bir LimeRFE kanalı ya da bir yapay yük kullanın.
settings-radio-limerfe-front-end = LimeRFE ön ucu
settings-radio-connected = Bağlı olduğu
settings-radio-limerfe-s-own-micro-usb = Radyonun değil, LimeRFE'nin kendi micro-USB portu. 9600 baud, yazılımı tarafından sabitlenmiş.
settings-radio-receive-connector = Alma konektörü
settings-radio-transmit-connector = Yayın konektörü
settings-radio-follow-dial = Skalayı izle
settings-radio-switch-filters-match-operating-frequency = Herhangi bir RF görünmeden önce filtreleri çalışma frekansına uyacak şekilde değiştirin. Tek bir bant içinde ayarlamak kontrol bağlantısına hiçbir şey koymaz.
settings-radio-relays = Röleler
settings-radio-receive-attenuator = Alma zayıflatıcısı
settings-radio-notch-filter = Çentik filtresi
settings-radio-fan = Fan
settings-radio-worth-having-any-sustained-transmitting = Sürekli yayın yapılan her durumda açmaya değer.
settings-radio-automatic-board-receives-switched-transmit = Otomatik'te kart alır ve tuşa basılınca yayına, bırakılınca geri alınmaya geçirilir — hangi kablolamada olursa olsun. Amatör kanallarında iki yanında alma/yayın anahtarı olan tek bir filtre vardır, bu yüzden her ikisini birden isteyen bir kart, o anahtarı vericiye koyar ve hiçbir şey duymaz. 30 MHz kanalına yalnızca J5 üzerinden ulaşılır; bu, iki yön için de tek bir konektördür; onun üstünde, J4'ten yayın yapmak, alma yolunu amplifikatörün sürdüğü konektörün dışında tutar.
settings-radio-limesdr-support-has-not-been = LimeSDR desteği donanıma karşı doğrulanmadı. Bu, son oturumun izidir; bir sorun bildirimi içindir.
settings-radio-gains-filters-corrections-antenna-sockets = Kazançlar, filtreler, düzeltmeler, anten soketleri ve her LimeRFE denetimi — konektörleri, bandı, röleleri, zayıflatıcısı, çentiği ve fanı — hemen uygulanır. Kart, alma zinciri, örnekleme hızı, yayını etkinleştirme ve LimeRFE'nin bağlantısı Uygula ile etkinleşir.

## Ayarlar penceresi, Radyo sekmesi: değerli iletiler

settings-radio-fill-from = { $name } İLE DOLDUR
settings-radio-decimation-auto = Otomatik (en yakın { $khz } kHz)
settings-radio-decimation-stage-server = Basamak { $stage } — sunucunun hızı ÷ { $divisor }
settings-radio-decimation-stage = Basamak { $stage } — hız ÷ { $divisor }
settings-radio-kiwi-receive-only = Yalnızca alma — bu başkasının anteni. Panadaptör, alıcının I/Q olarak gönderdiği ve skalayı izleyen yaklaşık 12 kHz'dir; üstündeki bant görünümü kendi şelalesidir ve üzerinde ayar yapmak alıcıyı yeniden ayarlar. Bağlıyken yaklaşık { $rate } kB/s. Yeniden başlatmadan geçmek için “Apply / reconnect”e basın.
settings-radio-connected-detail = Bağlı: { $detail }
settings-radio-failed = Başarısız: { $error }
settings-radio-pluto-buffer = ≈ { $ms } ms, aktarım başına { $kib } KiB
settings-radio-driver-settings = { $driver } ayarları
settings-radio-rx888-coverage = 0–{ $mhz } MHz kapsama, USB üzerinden { $mbytes } MB/s
settings-radio-rx888-digitised = { $mhz } MHz'in sayısallaştırılmış kısmı
settings-radio-elad-16-bit = { $khz } kHz (16 bit örnek)
settings-radio-elad-pad = { $db } dB pad girişte
settings-radio-hackrf-rx-preamp = Alımda { $db } dB ön yükselteç
settings-radio-hackrf-tx-preamp = Yayında { $db } dB ön yükselteç
settings-radio-device-not-found = { $device } (bulunamadı)
settings-radio-lime-chain = Zincir { $n } (RX{ $n }_*)
settings-radio-lime-chain-tx = Zincir { $n } (RX{ $n }_* / TX{ $n }_*)
settings-radio-lime-second-chain = Kartın diğer alma zinciri, { $sockets } soketlerinde. Sentezleyiciyi paylaşır, bu yüzden ilkiyle aynı bandı aynı anda duyar — bu da ikinci bir anteni ya da kendi vericinizin bir örneğini taşımasını sağlar.
settings-radio-tx-blocked = Yayın engellendi: { $reason }
settings-radio-rfe-rx-fallback = { $port } üzerinde alırken, bunlar filtresiz geniş bant yoluna düşer: { $bands }.
settings-radio-rfe-tx-fallback = { $port } üzerinde yayın yaparken, bunlar geniş bant yoluna düşer — bant amplifikatörü ve filtreleme yok: { $bands }.

## Üst çubuk ve denetim şeridi (src/app/top_bar.rs)

topbar-how-far-one-press-moves = Bir basışın skalayı ne kadar hareket ettirdiği. Bir sonraki adımı almak için dokunun: 10 Hz, 100 Hz, 500 Hz, 1, 2,5, 5, 9, 10 ve 25 kHz. Satırı Ayarlar › UI içinden kapatın.
topbar-volume-gain-agc-squelch-noise = Ses düzeyi, kazanç, AGC, susturma, gürültü denetimleri ve FM yayınında RDS okuması
topbar-receiver = ALICI
topbar-vfo-b-split-rit-xit = VFO A/B, split ve RIT/XIT kaymaları
topbar-tuning = AYAR
topbar-diversity-filter-which-way-combines = Çeşitlilik filtresi: iki anteni hangi yönde birleştirdiği, ne kadar hızlı uyum sağladığı ve olduğu yerde tutulması
topbar-diversity = ÇEŞİTLİLİK
topbar-second-receiver-s-frequency-mode = İkinci alıcının frekansı, modu, filtresi ve seviyesi
topbar-sub-receiver = ALT ALICI
topbar-transceiver-s-own-aerial-socket = Telsizin kendi anten soketi ve güç anahtarı
topbar-radio = RADYO
topbar-tune-voice-keyer-drive-mic = Ayar, sesli keyer ve sürüş ve mikrofon seviyeleri
topbar-transmit = YAYIN
topbar-tune = {" "}AYAR{" "}
topbar-panadapter-two-layers-their-speeds = Panadaptör — iki katmanı, hızları ve ayrıntısı, tepe tutma — artı şelale kontrastı, FFT boyutu ve skimmer'lar
topbar-display = EKRAN
topbar-logbook-spots-awards-memories-scanner = Günlük, spotlar, ödüller, hafızalar, tarayıcı, ayarlar ve kılavuz
topbar-system = SİSTEM
topbar-linked = Bağlı
topbar-not-linked = Bağlı değil
topbar-bar-trace = çubuk / iz
topbar-needle-bar-trace = iğne / çubuk / iz
topbar-swap-vfos = VFO'ları değiştir
topbar-copy-b = A'yı B'ye kopyala
topbar-second-receiver-right-ear-tunes = İkinci alıcı, sağ kulakta. A/B'den bağımsız ayarlanır — denetimleri SUB modülünde ve geçiş bandı şelalede görünür.
topbar-band-plan = , bant planından
topbar-repeater-shift-following-band-plan = Tekrarlayıcı kayması: bant planını izliyor, bu frekans için kayma yok — simpleks
topbar-repeater-shift-transmit-above-below = Tekrarlayıcı kayması: bir tekrarlayıcıyla çalışmak için skalanın üstünde ya da altında yayın yapın. Şimdi simpleks
topbar-repeater-shift = TEKRARLAYICI KAYMASI
topbar-auto = OTO
topbar-take-shift-band-plan-dial = Skala hareket ettikçe kaymayı bant planından alın. Yalnızca bir tekrarlayıcı çıkış alt bandı içinde konuşur — başka her yerde radyoyu simpleks bırakır, bu yüzden çağrı kanalları simpleks kalır.
topbar-offset = Kayma
topbar-repeater-tone-ctcss-dcs-goes = Tekrarlayıcı tonu: sesin altında giden CTCSS/DCS, 1750 Hz patlaması ve alma ton susturması
topbar-transmit-tone = YAYIN TONU
topbar-set-here-sent-nfm-sub = Burada ayarlanır, NFM'de gönderilir — alt-işitsel sinyalleme bir FM kanalının işidir
topbar-normal = NORMAL
topbar-invert = TERS
topbar-bit-order-dcs-encoded-here = DCS'nin burada kodlandığı bit sırası standarttan aktarılmıştır ve hiçbir tekrarlayıcıya karşı denetlenmemiştir — açılmazsa CTCSS'i deneyin
topbar-t-1750-hz-burst = 1750 HZ PATLAMASI
topbar-every-over = HER YAYINDA
topbar-open-burst-every-time-transmitter = Verici her tuşlandığında patlamayla açın
topbar-send = GÖNDER
topbar-send-burst-now-keying-transmitter = Patlamayı şimdi gönderin — zaten tuşlu değilse uzunluğu kadar vericiyi tuşlayarak
topbar-nfm-only-burst-fm-repeater = Yalnızca NFM: patlama bir FM tekrarlayıcının kapı açıcısıdır
topbar-receive-tone-squelch = ALMA TON SUSTURMASI
topbar-require-receive-what-station-transmits = Bu istasyonun yayınladığını alırken iste
topbar-hide-docked-band-selector-click = Yerleşik bant seçicisini gizle — geri getirmek için yeniden tıklayın
topbar-show-docked-band-selector = Yerleşik bant seçicisini göster
topbar-dock = YERLEŞTİR
topbar-keep-band-mode-selector-open = Bu açılır pencereyi her seferinde kapatmak yerine bant ve mod seçicisini şelalenin yanında açık tutun
topbar-band-mode = BANT VE MOD
topbar-hide-band-chip-brings-back = Gizle — bant çipi onu geri getirir
topbar-undock = YERLEŞTİRMEYİ KALDIR
topbar-return-selector-top-bar-popup = Seçiciyi üst çubuktaki açılır pencereye geri döndürün
topbar-dec-off = DEC kapalı
topbar-vol = Ses
topbar-counted-steps-not-decibels-what =


    Adımla sayılır, desibelle değil: bir adımın ne değerinde olduğu alıcının kendi işidir ve SDRplay'de bant da buna dahildir. Doğrusu yine daha fazla kazançtır.
topbar-gain = Kazanç
topbar-plutosdr-s-own-agc-setting =


    PlutoSDR'nin kendi AGC'si bu kazancı ayarlıyor, bu yüzden kaydırıcıyı hareket ettirmek bir şey yapmaz. Burada denetlemek için Ayarlar → Radyo → AGC'yi Manuel'e alın.
topbar-agc-hang-time-click-cycle = AGC tutma süresi — döngü için tıklayın: Kapalı / Yavaş / Orta / Hızlı
topbar-radio-s-own-agc-click = Radyonun kendi AGC'si — döngü için tıklayın: Yavaş / Orta / Hızlı. Buradan kapatılamaz.
topbar-man = Man
topbar-manual-audio-gain-used-while = Elle ses kazancı, AGC kapalıyken kullanılır. Kapatıldığı andaki AGC'nin tuttuğu seviyeden başlar.
topbar-radio-s-own-squelch-sent = Radyonun kendi susturması, kontrol bağlantısı üzerinden gönderilir. Ses gerçekten bu kapıdan geçer — yazılım susturması, telsizin zaten geçirdiğinin üstüne yalnızca daha fazla kapanabilirdi.
topbar-max-gain = en fazla kazanç
topbar-manual-gain = elle kazanç
topbar-noise-reduction = gürültü azaltma
topbar-impulse-noise-blanker = Darbe gürültü söndürücü
topbar-auto-notch-cancel-constant-tone = Otomatik çentik: sabit ton bileşenlerini iptal eder (heterodinler)
topbar-noise-reduction-off-click-switch = Gürültü azaltma, kapalı — açmak için tıklayın ya da aşağıdaki satırlardan bir motor seçin
topbar-binaural-audio-not-while-sub = Binaural ses — alt alıcı sağ kulaktayken değil
topbar-binaural-audio-passband-spread-across = Binaural ses: geçiş bandı iki kulağa yayılır; böylece farklı perdelerdeki sinyaller farklı yönlerden gelir ve ayar yaptığınız sinyal iki yana kayar. Mono'ya dönmek için tıklayın
topbar-binaural-cw-spread-passband-across = Binaural CW: geçiş bandını iki kulağa yayın; böylece farklı perdelerdeki sinyaller farklı yönlerden gelir — bir yığılma tek bir kalabalık nota yerine birkaç yer olur ve bir istasyonu ayarlamak onu yana kaydırır. Kulaklıkla en iyisi
topbar-binaural-audio-spread-passband-across = Binaural ses: geçiş bandını iki kulağa yayın. Konuşmada gürültü tüm görüntüye yayılır, istasyon ise ortada kalır; bu bir saat dinlemek için daha kolaydır — konuşmanın kendisinin yayılması pahasına. Kulaklıkla en iyisi
topbar-mute = SUSTUR
topbar-record-audio-raw-i-q = Sesi, ham I/Q'yu ya da ikisini birden kaydedin
topbar-wfm-stereo-forced-off-click = WFM stereo zorla kapalı — otomatik stereo için tıklayın
topbar-wfm-stereo-pilot-locked-click = WFM stereo: pilot kilitli. Mono'yu zorlamak için tıklayın
topbar-wfm-stereo-automatic-no-pilot = WFM stereo: otomatik, bu istasyonda pilot yok
topbar-rds-data-arriving-click-see = RDS verisi geliyor — görmek için tıklayın
topbar-rds-no-data-station-click = RDS — bu istasyonda veri yok. Yine de çözücünün tanılamaları için tıklayın
topbar-drm-decoding-click-details = DRM çözülüyor — ayrıntılar için tıklayın
topbar-drm-click-decoder-s-state = DRM — çözücünün durumu için tıklayın
topbar-hd-radio-decoding-click-details = HD Radio çözülüyor — ayrıntılar için tıklayın
topbar-hd-radio-click-decoder-s = HD Radio — çözücünün durumu için tıklayın
topbar-presets = ÖN AYARLAR
topbar-width-per-sideband = YAN BANT BAŞINA GENİŞLİK
topbar-width = GENİŞLİK
topbar-passband-width-hertz-type-drag = Hertz cinsinden geçiş bandı genişliği — yazın ya da sürükleyin. Kesin bir değer girmek için alana tıklayın.
topbar-edges = KENARLAR
topbar-low-edge-hz-carrier = Alt kenar, taşıyıcıdan Hz cinsinden
topbar-high-edge-hz-carrier = Üst kenar, taşıyıcıdan Hz cinsinden
topbar-noise-reduction-voice-off-click = Gürültü azaltma (ses), kapalı — bir motor seçmek için tıklayın
topbar-record-audio = SESİ KAYDET
topbar-mp3 = ● MP3
topbar-what-you-hear-mp3-receiver = MP3 olarak duyduklarınız: alıcı bir kanalda, kendi yayınınız diğerinde. Bu bir QSO'nun kaydıdır.
topbar-mono = MONO
topbar-mixed-one-channel-click-two = Tek kanala karıştırıldı — ikisi için tıklayın
topbar-two-channels-rx-left-tx = İki kanal: RX sol, TX sağ — tek karışık kanal için tıklayın
topbar-quick-clip = HIZLI KLİP
topbar-stop-after = ŞUNDAN SONRA DURDUR
topbar-no-stop = durdurma yok
topbar-auto-record = OTOMATİK KAYIT
topbar-set-squelch-auto-record-needs = Bir susturma ayarlayın — otomatik kayıt sessizliği bilmek için birine ihtiyaç duyar
topbar-auto-record-needs-sdr-front = Otomatik kayıt, yazılım susturması olan bir SDR ön ucu gerektirir
topbar-stop-following-squelch = Susturmayı izlemeyi bırak
topbar-waiting-signal = bir sinyal bekleniyor
topbar-record-spectrum = SPEKTRUMU KAYDET
topbar-i-q-wav = ● I/Q WAV
topbar-i-q-wav-2 = I/Q WAV
topbar-raw-spectrum-receiver-delivering-stereo = Alıcının verdiği ham spektrum, SDR#, SDRuno, HDSDR ve SDRangel'in açtığı stereo 32 bit kayan nokta WAV olarak (4 GB'ın üstünde RF64) — sdroxide'ın kendisi de --file ile oynatır. Büyüktür: örnek başına 8 bayt.
topbar-radio-hands-over-demodulated-audio = Bu radyo demodüle edilmiş ses verir, bu yüzden kaydedilecek I/Q yoktur.
topbar-what-capture-costs-sample-rate = Bu örnekleme hızında bir kaydın maliyeti
topbar-engine = MOTOR
topbar-no-noise-reduction-decoders-never = Gürültü azaltma yok — çözücüler onu zaten hiç görmedi
topbar-strength = GÜÇ
topbar-tone = TON
topbar-ctcss-dcs-no-sub-audible = CTCSS / DCS — bu sinyalde alt-işitsel ton yok
topbar-tone-squelch = TON SUSTURMASI
topbar-carrier-squelch-open-any-signal = Taşıyıcı susturması: herhangi bir sinyalde açılır
topbar-require-tone-currently-being-received = Şu anda alınan tonu iste
topbar-ctcss = CTCSS
topbar-any-dcs = HERHANGİ BİR DCS
topbar-open-any-dcs-coded-signal = Herhangi bir DCS kodlu sinyalde açılır. Taşıdığı 104 koddan hangisinin olduğu burada güvenilir biçimde okunamaz, bu yüzden daha ince seçilecek bir şey yoktur
topbar-which-socket-back-radio-receiving =
    Radyonun arkadaki hangi soketi aldığı — kendi ANT komutu; ön paneldeki ANT düğmesiyle aynı ayar. Bir sonrakine geçmek için tıklayın.

    Seçim bant başına hatırlanır ve skala o banda yeniden girdiğinde geri konur.
topbar-radio-s-separate-receiving-antenna =
    Radyonun ayrı alma anteni, alma yoluna ya da alma yolundan çıkarılmış — kendi RX ANT ayarı. Ana soketteki anten her iki durumda da yayında kalır.

    Radyo bunu bant başına kendisi hatırlar, bu yüzden sdroxide her bant değişiminden sonra geri okur; son gördüğünü geri koymaz: buraya tıklamak onu hareket ettiren tek şeydir.
topbar-radio-s-own-power-switch =
    Radyonun kendi güç anahtarı, kontrol bağlantısı üzerinden; programdaki tek gerçek açma/kapatma budur — sdroxide'ın LINK anahtarı değildir, o yalnızca sdroxide'ın tarafını kapatır ve radyo çalışmaya devam eder.

    AÇIK komutunun bir şeye ulaşması için radyonun kontrol ucu kapalıyken uyanık kalmalıdır: LAN üzerinden Network Control, ya da ön panelden kapalı bir cihazda şebekeden beslenmeye devam eden bir CI-V portu.
topbar-switch-radio = Radyoyu aç
topbar-switch-radio-off-audio-meters = Radyoyu kapat. Ses ve metreler onunla birlikte durur.
topbar-what-second-aerial-click-swap =
    İkinci anten ne için — değiştirmek için tıklayın.

    İPTAL, onu birinciden, yerel bir gürültü kaynağının her ikisinde de hizalandığı kazanç, faz ve gecikmeyle çıkarır: gürültü iptal eden bir fazlayıcının DSP biçimi. BİRLEŞTİR, ikisini her birinin ne kadar iyi duyduğuna göre ağırlıklandırıp güçlendiren fazda toplar — çeşitlilik alımı, sönümlemeleri doldurur.
topbar-hold = TUT
topbar-stop-filter-moving-reach-moment = Filtrenin hareket etmesini durdurun. Bir boşluk belirir belirmez buna uzanın: uyarlanmaya bırakılmış bir filtre, en yüksek olan şeye yeniden yönelir; sessiz bir bantta bu, dinlediğiniz istasyondur.
topbar-restart = YENİDEN BAŞLAT
topbar-zero-filter-find-null-again = Filtreyi sıfırlayın ve boşluğu yeniden bulun.
topbar-adapt = Uyarla
topbar-how-fast-filter-chases-slow = Filtrenin ne kadar hızlı kovaladığı: solda yavaş ve sabit, saniyenin küçük bir kesrinde yakınsayan ve sağda gözle görülür biçimde avlanan. Boşluğu bulmak için hızlı başlayın, sonra TUTUN.
topbar-where-sub-receiver-listens-shift = Alt alıcının nereyi dinlediği. Taşımak için şelaleye Shift-tıklayın ya da alt'ın geçiş bandı içinde sürükleyin.
topbar-dial = ←SKALA
topbar-move-sub-receiver-main-dial = Alt alıcıyı ana skalaya taşı
topbar-dial-2 = SKALA←
topbar-move-main-dial-sub-receiver = Ana skalayı alt alıcıya taşı
topbar-filter = Filtre
topbar-sub-receiver-passband-edges-hz = Alt alıcının geçiş bandı kenarları, Hz cinsinden
topbar-sub-receiver-level-plays-right = Alt alıcının seviyesi (sağ kulakta çalar)
topbar-ptt = {" "}PTT{" "}
topbar-voice-keyer-record-transmit-stored = Sesli keyer: kayıtlı iletileri kaydedin ve yayınlayın
topbar-drive = Sürüş
topbar-tune-2 = Ayar
topbar-tx-audio-processing-microphone-click = TX Audio mikrofonu işliyor — açmak için tıklayın
topbar-click-open-tx-audio-gate = TX Audio'yu açmak için tıklayın: mikrofon için kapı, EQ, kompresör ve sınırlayıcı
topbar-mic = Mik
topbar-cessb = CESSB
topbar-microphone-gain = Mikrofon kazancı
topbar-deviation-how-far-mode-s =
    Sapma: bu modun patlamasının, onu kendisi modüle eden bir radyoyu ne kadar salladığı. Bir FM verici ses seviyesini frekans salınımına çevirir ve onu yakalayacak bir ALC'si yoktur; bu yüzden konuşma için ayarlanmış bir veri girişine tam ölçek, aşırı sapar — bu, dinleyiciye tamamen normal gelir ve kimse için çözülmez.

    Mod başına tutulur, böylece 1200 baud paket için ayarlanan bir sapma hiçbir zaman FT8'e geçmez.
topbar-transmit-audio-how-hard-mode =
    Yayın sesi: bu mod, gönderdiğimiz şeyi modüle eden bir radyonun modülatörünü ne kadar sert sürer — ses kartı üzerinden bir CAT telsizi, bir FLEX, ağ portundaki bir Icom. Telsizin ALC'si zar zor hareket edene dek kısın ve gücü radyoda ayarlayın; sabit zarflı bir dijital modda ALC'nin yükselmesi saçılmaya yol açar.

    Sürüş bu denetim değildir: bu radyolarda Sürüş, telsizin güç yazmacına ulaşır ve sesine hiç dokunmaz.

    Mod başına tutulur — FT8, RTTY, PSK ve MCW kendi değerlerini tutar.
topbar-tx-audio = TX sesi
topbar-cw-psk-rtty-skimmers-decode = CW / PSK / RTTY skimmer'ları — bant boyunca sinyalleri çözer ve şelalede işaretler
topbar-skimmers = SKIMMER'LAR
topbar-run-skimmer = Bu skimmer'ı çalıştır
topbar-minimum-snr-decoded-signal-needs = Çözülen bir sinyalin spot olarak işaretlenmesi için gereken en düşük SNR
topbar-needs-wideband-iq-source = geniş bant IQ kaynağı gerektirir
topbar-cw-decoder = CW ÇÖZÜCÜ
topbar-how-many-signals-model-reads = Model bir seferde kaç sinyal okur. Kalanlar işaretini korur ama metin taşımaz.
topbar-spots = SPOTLAR
topbar-save-skimmer-s-spot-list = Skimmer'ın spot listesini bir dosyaya kaydedin
topbar-show-hide-full-band-waterfall = Panadaptörün üstündeki tüm bant şelale şeridini göster/gizle — bu alıcının bir anda görebildiği her şey
topbar-spectrum-waterfall-either-layer-both = Spektrum ve şelale — biri, ikisi ya da hiçbiri. İkisi de gösterilirken yanar.
topbar-spectrum = SPEKTRUM
topbar-show-spectrum = SPEKTRUMU GÖSTER
topbar-draw-spectrum-line-across-top = Spektrum çizgisini panadaptörün üstünden çizin. Kapalıyken şelale tüm yüksekliği alır.
topbar-peak-hold = TEPE TUTMA
topbar-trace-highest-level-each-column = Her sütunun canlı çizginin üstünde ulaştığı en yüksek seviyeyi izler ve geri aşağı söner — siz başka yere bakarken bandın yaptıkları
topbar-draw-spectrum-receding-surface-instead = Spektrumu düz bir çizgi yerine geri çekilen bir yüzey olarak çizin: en yeni spektrum ön tarafta, ondan öncekiler sizden uzağa akar. Bandın son birkaç saniyesi bir manzara olarak — gelip giden bir taşıyıcı, seğiren bir çizgi değil bir sırttır. Izgara ve tepe tutma düz çizgiye aittir ve onun üzerine çizilmez.
topbar-how-quickly-spectrum-line-follows = Spektrum çizgisinin bandı ne kadar hızlı izlediği. Daha yavaş, her kareyi birbirine daha fazla karıştırır: daha sakin bir çizgi ve okunabilecek kadar uzun duran zayıf bir taşıyıcı. Şelaleye dokunmaz — o satırlar her iki durumda da her kareyi alır.
topbar-waterfall = ŞELALE
topbar-show-waterfall = ŞELALEYİ GÖSTER
topbar-draw-scrolling-waterfall-below-spectrum = Kayan şelaleyi spektrumun altında çizin. Kapalıyken spektrum çizgisi tüm yüksekliği alır.
topbar-how-fast-waterfall-scrolls-lines = Şelalenin saniyede kaç satır kaydığı: Yavaş 5, Orta 28, Hızlı 56, Daha hızlı 112, En hızlı 224. Motor bunları kendisi zamanlar; bu yüzden en hızlı iki seçenek, aynı çizginin iki kez çizilmesi değil gerçek ayrıntıdır — alıcı onları besleyebildiği ölçüde: bir satır asla birden fazla dönüşüm gösteremez ve dar bir ön uç saniyede yalnızca birkaç düzine üretir. Geçmişe mal olurlar, çünkü şelale sabit sayıda satır tutar — Orta'da 73 saniye, En hızlı'da 9.
topbar-decode-labels = ÇÖZÜM ETİKETLERİ
topbar-mark-every-decoded-station-waterfall = Çözülen her istasyonu şelalede, duyulduğu frekansta çağrı işaretiyle işaretleyin — FT8, FT4 ve diğer zamanlanmış modlar. İyi bir açılış, dakikada iki kez bandın boyunca otuz tanesini okuduğunuz izlerin üstüne koyar; kapatırsanız şelale yalnızca bant olur. Yanındaki çözüm listesinde yine her çağrı işareti vardır.
topbar-switch-3d-set = Bunu ayarlamak için 3B'yi açın
topbar-how-3d-spectrum-drawn-only = 3B spektrumun nasıl çizildiği. Yalnızca biçim farklıdır — iki gösterim de aynı spektrumları gösterir ve ikisi de arkalarındakini gizler; bu yüzden önde duran güçlü bir sinyal, kapladığı bandın üstünde durur.
topbar-lines = ÇİZGİLER
topbar-one-trace-per-remembered-spectrum = Hatırlanan spektrum başına bir iz, düz çizginin kendi renginde; her biri arkasındakileri gizler. Seviyeler olmadan biçim — ve biçim istediğinizde seçilecek okuma; katı olandan biraz daha fazla maliyeti vardır, çünkü aynı yüzeyi çizer ve sonra her tepeyi üstüne çizer.
topbar-solid = KATI
topbar-filled-surface-coloured-waterfall-s = Şelalenin paletiyle renklendirilmiş dolu bir yüzey; seviye hem yükseklikte hem renkte yer alır ve panadaptörün iki yarısı güçlü bir sinyalin neye benzediği konusunda hemfikirdir. Paleti Ayarlar › Ekran bölümünden değiştirin.
topbar-how-fast-3d-spectrum-flows = 3B spektrumun sizden uzağa saniyede kaç satır aktığı: Yavaş 6, Orta 12, Hızlı 24, Daha hızlı 48. Yüzey sabit sayıda satır derinliğindedir, bu yüzden bu aynı zamanda ne kadar zaman tuttuğudur. Daha yavaş daha uzun bir hafıza ve sürünen bir yüzey demektir; daha hızlı ise daha kısa ve hareketli bir yüzey. Şelale, aşağıda kendi kayma hızını tutar.
topbar-how-many-columns-panadapter-waterfall = Panadaptör ve şelalesinin kaç sütunla çizildiği. Otomatik, bu makinenin çizicisini ve panadaptörün boyutunu okur ve taşıyabileceği en fazlasını seçer — 4K ekran 4096 ister. Her sütun her karedeki bir bayttır; bu yüzden uzak bir istasyona bağlı bir istemci, ayrıntının maliyetini kendi bağlantısında öder: 60 kare/sn'de 4096 sütun, saniyede yaklaşık çeyrek megabayt eder — standart genişliğin iki katı.
topbar-read-machine-screen-take-most = Bu makineyi ve bu ekranı okuyup taşıyabildikleri en fazlasını alın ve değişirlerse onları izleyin
topbar-width-force-right-now-whatever = Yukarıdaki satırın istediğinden bağımsız olarak şu anda geçerli olan genişlik
topbar-keep-floor-ceiling-set-best = En iyi şelale kontrastı için ayarlanmış tabanı/tavanı koruyun — bant değişiminde, kaydırma ya da yakınlaştırmadan sonra ve seviyeler kaydığında yerine yumuşakça oturtulur. Hemen sığdırmak için açın; seviyeleri ayarladığınız yerde tutmak için kapatın.
topbar-keep-tuned-frequency-middle-panadapter = Ayarlı frekansı panadaptörün ortasında tutun: skala kenardan ayar yaptığınızda resim bütün bir aralık sıçramak yerine pencere skalanın altında kayar. Hemen ortalamak için açın; kaydırıp yakınlaştırmak istediğiniz yerde kapatın.
topbar-spectrum-floor-ceiling-fft-size = Spektrum tabanı / tavanı ve FFT boyutu
topbar-level-drawn-bottom-spectrum-darkest = Spektrumun altında ve en koyu şelale rengi olarak çizilen seviye. Gürültü tabanı tam olarak kararana dek yukarı çekin. Ayarlamak SIĞDIR'ı kapatır.
topbar-ceil = tavan{" "}
topbar-level-drawn-top-bring-down = Üstte çizilen seviye. İlgilendiğiniz en güçlü sinyal tam renge ulaşana dek aşağı indirin. Ayarlamak SIĞDIR'ı kapatır.
topbar-fft-size = FFT BOYUTU
topbar-flip = ÇEVİR
topbar-scroll-waterfall-upwards-newest-row = Şelaleyi yukarı kaydırın — en yeni satır altta (V)
topbar-logbook-all-qsos-digital-manual = Günlük — tüm QSO'lar (dijital + elle)
topbar-live-spots-dx-cluster-pota = Canlı spotlar — DX kümesi, POTA, SOTA, PSK Reporter
topbar-award-tracking-dxcc-was-waz = Ödül takibi — DXCC / WAS / WAZ / lokatörler
topbar-band-conditions-published-forecast-beside = Bant koşulları — yayımlanmış tahmin, her bantta gerçekte duyulanın yanında
topbar-satellite-qo-100-beacon-calibration = Uydu — QO-100 işaret kalibrasyonu çalışıyor
topbar-satellite-doppler-tracking-qo-100 = Uydu — Doppler izleme ve QO-100 işaret kalibrasyonu
topbar-ism-band-devices-decoding-now = ISM bandı aygıtları — şimdi çözülüyor, bu pencere açık olsun ya da olmasın. Pencerenin içindeki ÇÖZME ile kapatın.
topbar-ism-band-devices-weather-sensors = ISM bandı aygıtları — çevrenizde duyulan hava sensörleri, sayaçlar ve ev otomasyonu
topbar-public-sdrs-internet-browse-kiwisdr = İnternetteki genel SDR'ler — KiwiSDR ve SpyServer dizinlerine göz atın ve birini radyo olarak açın
topbar-winlink-radio-email = Winlink telsiz e-postası
topbar-memory-channels = Hafıza kanalları
topbar-scanner-stopped-signal = Tarayıcı — bir sinyalde durdu
topbar-scanner-running = Tarayıcı — çalışıyor
topbar-scan-memory-channels-frequency-range = Hafıza kanallarını ya da bir frekans aralığını tarayın
topbar-hfdl-ground-network-decode-log = HFDL yer ağı — çözüm günlüğü ve uçak haritası, şelalenin altında. Çözücüyü panelin içindeki DİNLE ile açın.
topbar-hfdl-ground-network-decoding-now = HFDL yer ağı — şimdi çözülüyor. Paneli açın.
topbar-hfdl-ground-network-aircraft-shortwave = HFDL yer ağı — uçakların kısa dalga veri bağlantısı, her seferinde bir dinleme kanalı
topbar-grid-tracker-worked-maidenhead-squares = Lokatör izleyici — haritada çalışılmış Maidenhead kareleri ve duyulanlar
topbar-amplifier-set-up-hardrock-under = Amplifikatör — Ayarlar → Amplifikatör altında bir Hardrock kurun
topbar-settings-device-gains-antennas-audio = Ayarlar — aygıt kazançları, antenler, ses aygıtları
topbar-user-manual-f1 = Kullanım kılavuzu (F1)
topbar-range-was-typed-settings-radio = Bu aralık, radyo tarafından bildirilmemiş; Ayarlar ▸ Radyo ▸ RX aralığı içinde yazıldı. Genişletin ya da aygıtın kendisi hakkında söylediği şeyi kullanmak için kutuyu boşaltın.
topbar-what-device-reports-about-itself = Aygıtın kendisi hakkında bildirdiği budur. Radyo bu bandı gerçekten kapsıyorsa, gerçek aralığını Ayarlar ▸ Radyo ▸ RX aralığı içinde belirtin.
topbar-band = BANT
topbar-mode = MOD
topbar-digital = DİJİTAL

## Üst çubuk: Sistem kutusunun pencere çipleri.

topbar-chip-log = GÜNLÜK
topbar-chip-spots = SPOTLAR
topbar-chip-awards = ÖDÜLLER
topbar-chip-bands = BANTLAR
topbar-chip-sat = UYDU
topbar-chip-ism = ISM
topbar-chip-public-sdr = GENEL SDR
topbar-chip-mail = POSTA
topbar-chip-mem = HAFIZA
topbar-chip-scan = TARA
topbar-chip-hfdl = HFDL
topbar-chip-grid = LOKATÖR
topbar-chip-amp = AMP
topbar-chip-settings = ⚙ AYARLAR
topbar-chip-help = ? YARDIM

## Üst çubuk: mod varsayılanları çipi.

topbar-changed-agc = AGC
topbar-changed-squelch = susturma
topbar-changed-auto-notch = otomatik çentik
topbar-changed-stereo = stereo
topbar-changed-binaural = binaural
# $mode bir mod adıdır (USB, CW…); $changed, varsayılanlarından farklı olanları listeler.
topbar-mode-defaults =
    { $mode } varsayılanlarına dön. Bunlardan değiştirilenler: { $changed }.

    Geri koymak için tıklayın. { $mode } modunun kendi değerleri geri döner ve burada ayarladığınız unutulur.

## Ayarlar penceresi, Radyo sekmesi: değerli uzun iletiler

settings-radio-no-such-baud = Radyonun { $baud } baud ayarı yok — bunun yerine { $default } kullanılacak
settings-radio-icom-scope-hint =
    Radyonun kendi ekran tarama taramasını CI-V bağlantısı üzerinden akıtır ve panadaptör olarak çizer — demodüle ses veren bir telsizin bant için verebileceği tek resimdir, çünkü gönderdiği ses zaten filtresinden geçmiş olandır.

    Bunun için radyonun ayarlı olması gerekir: CI-V USB Baud Rate 115200'e ve CI-V USB Port “Unlink from {"["}REMOTE]”e ayarlı (ikisi de SET > Connectors > CI-V altında), ve yukarıdaki baud hızı buna uygun ayarlı — { $min_baud } altında taramalar bağlantıya sığmaz ve ekran kapalı kalır.

    Taramalar, radyonun dahili USB veriyolunu kendi ses kartıyla paylaşır. Ekran açıkken alınan ses kesilmeye başlarsa, kapatılacak ilk şey bu kutudur.
settings-radio-rx-tx-words = { $rx } / TX { $tx }
settings-radio-soapy-native = Aşağıdaki donanım sdroxide tarafından doğrudan desteklenir: { $names }. Yukarıda bu arabirimi seçmek size kendi ayarlarını verir — SoapySDR'nin ifade edemediği kazanç katları, filtreler ve çentikler — ve bir SoapySDR modülü gerektirmez.
settings-radio-rx888-width-hint = Panadaptörün bir anda gösterdiği sayısallaştırılmış spektrumun ne kadarı. Tüm DSP zinciri bu genişlikte çalışır, bu yüzden daha geniş olması orantılı olarak daha fazla CPU demektir — 1/2, şelalede tüm bandtır ve ciddi miktarda aritmetik. VHF geçişinin ({ $crossover } MHz) üstünde panadaptör antene değil tuner'ın IF'ine bakar; bu, 8 MHz genişliğindedir ve { $if_centre } MHz'e park edilmiştir — bu yüzden { $widest } değerinden geniş hiçbir şey üzerine ortalanamaz ve fazla genişlik, daha fazla spektrum değil sinyalin yanında ölü spektrumdur.
settings-radio-sdrplay-serial-missing = { $serial } seri numarası, SDRplay servisinin bildirdiği alıcılar arasında yok. Listelenen alıcılardan birini seçin ya da bunu yeniden takıp Yeniden tara'ya basın.
settings-radio-hdr-filter-hint =
    HDR yolunun önündeki analog filtre — alıcının kendi bant genişliğinden farklı bir denetim. Her ayar sabit bir merkez kümesinde kurulur ve başka yerde hiçbir şey yapamaz; bu { $centres } merkezlerinde kurulmuştur. Hemen uygulanır.

    Yol kendisi henüz çalışmıyor — yukarıdaki anahtara bakın — bu yüzden şu anda sessiz olan bir mod için bir filtre seçer.
settings-radio-rspduo-only = Yalnızca bir RSPduo ikinci bir tuner'a sahiptir ve burada seçilen alıcı bir { $model }, bu yüzden bu hiçbir şey yapmayacak.
settings-radio-lime-tx-gain-low = Yayın kazancı { $db } dB, aralığının en alt ucunda — kartta birkaç mikrowatt, bu da arkasında ne olursa olsun bir güç metresinde hiçbir şey olarak okunur. Hazır bir verici sizi şaşırtmasın diye varsayılan budur; tuşlamadan önce yükseltin.

## Ayarlar penceresi, Genel: SWR

settings-general-swr-tune-limit = (ayar: { $limit }:1)

## Üst çubuk ve denetim şeridi: değerli iletiler

# $reading, metrenin kendi üzerine gelince çıkan metindir; $faces, bir tıklamanın döngüye aldığı yüzleri listeler.
topbar-smeter-hover =
    { $reading }

    Metre yüzünü döngüye almak için tıklayın: { $faces }
topbar-step-down = Aşağı { $step }
topbar-step-up = Yukarı { $step }
# $auto topbar-band-plan ya da boştur.
topbar-repeater-shift-hover = Tekrarlayıcı kayması { $shift } — { $rx } üzerinde alınıyor, { $tx } MHz üzerinde yayınlanıyor{ $auto }
topbar-tone-and-burst = Sesin altında { $tone } yayınlanıyor ve her yayının başında { $ms } ms'lik 1750 Hz patlaması
topbar-tone-under-voice = Sesin altında { $tone } yayınlanıyor
topbar-burst-every-over = Her yayının başında { $ms } ms'lik 1750 Hz patlaması
topbar-match-tx = YAYINLA EŞLE ({ $tone })
topbar-decimation-on = Ön uç decimation: alıcıya, bu radyonun akıttığı { $msps } Msps'nin ortadaki 1/{ $n } kısmı veriliyor — { $span } — ve geri kalanı alıcıya ulaşmadan atılıyor.
topbar-decimation-off = Ön uç decimation, kapalı: alıcı, bu radyonun akıttığı tüm { $span } aralığını görüyor.
topbar-decimation-hint =
    { $what }

    Daha dar bir aralık, daha ince şelale çözünürlüğü, daha sessiz bir gürültü tabanı (yarılandıkça 3 dB) ve daha az CPU demektir — iki yanındaki bandın bedeliyle. Skala yine her yere ayarlanır; radyo, LO'sunu izlemek için taşır.

    Döngü: kapalı / 2 / 4 … { $max }. Döngü için tıklayın.
topbar-rx-gain-hint = Ön uç RX kazancı ({ $stage}). Çok fazlası alıcının ADC'sini kırpar ve sahte sinyalleri bant boyunca yayar; az olursa alıcı sağır olur.
topbar-rx-gain-stages =
    {""}

    Bu telsizin { $n } RX kazanç katı var — geri kalanı Ayarlar → Aygıt içindedir.
topbar-passband-now =
    {""}

    Geçiş bandı şu anda { $dbfs } dBFS.
# $level topbar-passband-now ya da boştur.
topbar-squelch-hint =
    Alma geçiş bandındaki sesi, bunun altındaki gücte dBFS cinsinden kapatır. Soldaki açıktır.{ $level } Gürültünün üstünde ve sinyalin altında ayarlayın.

    Radyonun kendi AGC'si zaten içinde olan bir akış — örneğin bir Icom'un 12 kHz IF'i — bir SDR'nin ham taban bandından bu ölçekte çok daha yüksek oturur; bu yüzden kaydırıcı tam ölçeğe ulaşır.
topbar-nr-cycle-hint =
    Gürültü azaltma: { $strength } — { $engine }

    Gücü döngüye almak için tıklayın: Düşük / Orta / Yüksek / Kapalı. Motor aşağıdaki satırlardadır.
topbar-recording-both = { $audio } ve { $iq } kaydediliyor
topbar-recording-audio = Ses { $audio } dosyasına kaydediliyor
topbar-recording-iq = I/Q { $iq} dosyasına kaydediliyor
topbar-auto-record-armed = Otomatik kayıt hazır: her yayın için bir dosya, { $secs } sn sessizlikten sonra kapanır
topbar-rds-hover = RDS: { $ps }. İstasyonun verisi için tıklayın
topbar-drm-hover = DRM: { $service }. Yayının ayrıntıları için tıklayın
topbar-drm-no-codec = DRM: { $summary } — kilitli, ama { $codec } sesi burada çözülemez. Eksik olanı görmek için tıklayın
topbar-drm-state = DRM: { $summary } — çözücünün durumu için tıklayın
topbar-hd-hover = HD Radio: { $station }. Yayının ayrıntıları için tıklayın
topbar-hd-state = HD Radio: { $summary } — çözücünün durumu için tıklayın
topbar-binaural-preset = Her kulakta { $preset } — { $lo } … { $hi } Hz, iki yan bant
topbar-nr-hover =
    Gürültü azaltma: { $strength } — { $engine }

    Motoru ya da gücü değiştirmek için tıklayın
topbar-clipping = { $span } klip kaydediliyor — baştan başlatmak için yeniden basın
topbar-record-clip = { $span } kaydet ve durdur — bir alım raporuna eklenecek bir klip
topbar-starting-clip = { $span } klip başlatılıyor…
topbar-stop-recording-after = MP3 kaydını { $minutes } dakika sonra durdur
topbar-stops-in = { $time } içinde durur
topbar-auto-record-hint = Her yayını kendi dosyasına kaydeder; { $secs } saniye sessizlikten sonra kapanır
topbar-recording-silence = kaydediliyor · { $secs } sn sessizlik ({ $hold } anında kapanır)
topbar-tone-open = { $tone } alınıyor, bu ton susturması — ses açık
topbar-tone-closed = { $heard } alınıyor, ama ton susturması { $wanted } istiyor — ses kapalı kalır
topbar-tone-heard = CTCSS/DCS { $tone } alınıyor
topbar-tone-nothing = Ton susturması { $tone }: buna uyan hiçbir şey alınmıyor
topbar-tone-use = { $tone } KULLAN
topbar-sockets = Soketler: { $sockets }
topbar-voice-keyer-tx = { $slot } yayınlanıyor — sesli keyer'ı açmak için tıklayın
topbar-drive-uncal-gen = Hiçbir amatör bandında olmayan bu frekans kalibre edilmemiş; bu yüzden ayar, vericiye tam olarak ulaşır.
topbar-drive-uncal-band = { $band } kalibre edilmemiş; bu yüzden ayar, vericiye tam olarak ulaşır.
# $trim işaretlidir (+1.5); $diff büyüklüğüdür; $more "less" ya da "more"dur.
topbar-drive-cal-gen = Hiçbir amatör bandında olmayan bu frekans { $trim } dB kalibre edilmiş; bu yüzden aynı ayar, kalibre edilmemiş bir banda göre burada havaya { $diff } dB { $more ->
        [less] daha az
       *[more] daha fazla
    } verir.
topbar-drive-cal-band = { $band } { $trim } dB kalibre edilmiş; bu yüzden aynı ayar, kalibre edilmemiş bir banda göre burada havaya { $diff } dB { $more ->
        [less] daha az
       *[more] daha fazla
    } verir.
topbar-drive-hover =
    Verici ne kadar sürülüyor. sdroxide'ın kendisinin modüle ettiği bir radyoda, modüle edilmiş örnekleri ölçekler; kendi güç denetimi olan bir telsizde ise telsize istenen, nominal gücün kesridir.

    { $trim }

    Ayarlar → Radyo → Banda göre yayın sürüşü.
topbar-cessb-hover =
    Kontrollü zarflı SSB: aynı tepe için daha fazla ortalama güç, saçılma olmadan — uzaktaki tarafta üç ya da dört desibel görünür yükseklik. Sesin işlemciye ne kadar desibel sürüldüğü; 0 kapalıdır, 6 makul bir ilk denemedir. Sürüşe dokunmaz.

    Şimdi: { $now }
topbar-waterfall-rate = Saniyede { $rows } satır — { $seconds } saniyelik bant
topbar-detail-auto-columns = { $n } sütun, Otomatik'in burada seçeceği her ne ise
topbar-n-columns = { $n } sütun
topbar-fft-bins-whole = Tüm akış boyunca { $n } kutu — sütun başına bir ya da daha ince
topbar-fft-bins-pooled = Panadaptörün { $cols } sütununa birleştirilmiş { $n } kutu: daha keskin sinyaller, daha fazla değil
topbar-sat-locked = Uydu — { $name } üzerine kilitli
topbar-amp-in-line = { $model } — hatta (PTT)
topbar-amp-bypassed = { $model } — atlanmış (KAPALI)
topbar-amp-not-answering = { $model } — yanıt vermiyor
topbar-filter-edges = Taşıyıcıdan { $lo } … { $hi } Hz
topbar-filter-isb = Alma filtresi: her kulakta { $width } Hz ({ $edges }). ISB'nin iki yan bandı ayrı yayınlardır, bu yüzden genişlik birinin genişliğidir.
topbar-filter-width = Alma filtresi: { $width } Hz geniş ({ $edges }).
topbar-filter-hint =
    { $what }

    Genişliği, iki kenarı ve bu modun ön ayarlarını sayı olarak görmek için tıklayın — panadaptörün tutamaçları geçiş bandını göz kararı yerleştirir; kesin bir değer burada yazılır.
topbar-band-outside-range =
    { $band } bu telsizin aldığı aralığın dışında ({ $ranges } MHz).

    { $why }
# $time "day" ya da "night"tır.
topbar-band-forecast = { $band }: { $verdict } ({ $time ->
        [day] gündüz
       *[night] gece
    }) — HAMQSL.com tarafından güneş indekslerinden tahmin, sizin kendi yolunuzun ölçümü değil.

## Üst çubuk: kısa sözcükler ve başlıklar

topbar-squelch-open = açık
topbar-off = kapalı
topbar-recording = kaydediliyor
topbar-sql = sql
topbar-stations = istasyonlar
topbar-caption-reaction = tepki
topbar-caption-scroll = kaydırma
topbar-caption-surface = yüzey
topbar-caption-flow = akış
topbar-caption-detail = ayrıntı
topbar-floor = taban

## Dijital mod panel sekmeleri (src/app/panels/mod.rs, pane_label).

pane-aircraft = UÇAK
pane-chart = GRAFİK
pane-chat = SOHBET
pane-decodes = ÇÖZÜMLER
pane-files = DOSYALAR
pane-heard = DUYULAN
pane-image = GÖRÜNTÜ
pane-map = HARİTA
pane-messages = İLETİLER
pane-monitor = İZLEYİCİ
pane-panel = PANEL
pane-qso = QSO
pane-reading = OKUMA
pane-receive = ALMA
pane-saved = KAYITLI
pane-send = GÖNDER
pane-spots = SPOTLAR
pane-stations = İSTASYONLAR
pane-status = DURUM
pane-terminal = TERMİNAL
pane-text = METİN
pane-traffic = TRAFİK
pane-vessels = GEMİLER
pane-wfall = ŞELALE

## Dijital mod panelleri, ortak (src/app/panels/mod.rs)

panels-save = {" "}KAYDET{" "}
panels-nothing-decoded-save = Kaydedilecek çözülmüş bir şey yok
panels-clear-rx = {" "}ALMAYI TEMİZLE{" "}
panels-empty-receive-window-nothing-air = Alma penceresini boşaltın. Havada olan hiçbir şey durmaz.
panels-nothing-received-clear = Temizlenecek alınmış bir şey yok
panels-save-what-panel-has-decoded = Bu panelin çözdüğü şeyi bir dosyaya kaydedin
panels-prop = YAYILIM
panels-shade-map-where-signals-are = Haritayı, her bantta sinyallerin gerçekten nereden geçtiğine göre gölgeleyin. Her alım, yolunun orta noktasına — onu büken iyonosfer parçasına — yerleştirilir, uzaktaki istasyona değil; bu yüzden bu, radyo amatörlerinin nerede yaşadığının değil gökyüzünün bir haritasıdır. Bu istasyonun katıldığı yollardan ve — Ayarlar → Spotlar altında Reverse Beacon Network açıksa — dünyadaki skimmer'ların duyduklarından kurulur; bu da bu radyonun olmadığı bantları kapsar.
panels-night = GECE
panels-shade-where-sun-down-twilight = Güneşin battığı ve aradaki alacakaranlığı gölgeleyin; böylece gri çizgi haritada görünür. Alçak bantlar onun gece tarafında uzar, yüksek bantlar kapanır ve sınır çizgisinin kendisi DX'in olduğu yerdir.
panels-all-bands = TÜM BANTLAR
panels-every-band-once-one-hue = Her bant bir kez, her biri kendi tonunda — genel koşullar.
panels-one-band = TEK BANT
panels-one-band-cold-hot-blue = Tek bir bant, soğuktan sıcağa: mavi, yeşil, sarı, kırmızı.
panels-frequency-you-are-actually-working = Gerçekten çalıştığınız frekans: skala artı yanındaki ton kayması. Günlüğe giren budur — yalnızca skala o kadar düşük kalır.
panels-send-return = DÖNÜŞTE GÖNDER
panels-freq = ⇵ FREK
panels-nothing-agreed-mode-save-your = Bu mod için üzerinde anlaşılmış bir şey yok — aşağıya kendinizinkini kaydedin.
panels-forget-saved-frequency = Bu kayıtlı frekansı unut
panels-dial-already-one-your-saved = Bu skala zaten kayıtlı frekanslarınızdan biri.
panels-dial-already-agreed-list-mode = Bu skala zaten bu mod için anlaşılmış listede.
panels-decode-squelch-raise-stop-decoding = Çözme susturması — gürültüyü çözmeyi durdurmak için yükseltin
panels-message-repeats-long-transmit-held = Yayın basılı tutuldukça ileti tekrarlanır — bir meteor, geçen parçasını yakalar.
panels-w1abc-w9xyz-fn42 = W1ABC W9XYZ FN42
panels-tx = {"  "}YAYIN AÇIK{"  "}
panels-tx-2 = {"   "}YAYIN{"   "}
panels-call-cq = {" "}CQ ÇAĞRISI{" "}
panels-nocall = NOCALL

## TX Audio penceresi (src/app/tx_audio.rs)

txaudio-heard-too-little-speech-talk = Çok az konuşma duyuldu — tüm sayım boyunca normal konuşun ve yeniden deneyin.
txaudio-tx-audio = TX Audio
txaudio-mode-not-voice-mode-so = Bu mod bir ses modu değil, bu yüzden gönderdiği hiçbir şey zincirden geçmez. Burada yapılan ayarlar SSB, AM ve FM'de etkinleşir.
txaudio-bypass = ATLA
txaudio-switch-every-stage-out-once = Gürültü temizleyici dahil her katı bir anda devreden çıkarın; mikrofon işlenmeden gider. Tam olarak açık olan katları geri getirmek için yeniden tıklayın — zincirin ne yaptığının en hızlı A/B karşılaştırması.
txaudio-denoise = GÜRÜLTÜ TEMİZLE
txaudio-rnnoise-microphone-ahead-every-other = Mikrofonda, diğer her katın önünde RNNoise; böylece oda gürültüsü, kapı ya da kompresör onu yükseltmeden önce temizlenir. Yaklaşık 10 ms gecikme ekler.
txaudio-digital-voice = DİJİTAL SES
txaudio-also-run-freedv-rade-digital = FreeDV / RADE dijital sesi de zincirden geçirin. Varsayılan olarak kapalı: bir vocoder doğal konuşma üzerinde eğitilmiştir ve ağır işlenmiş bir sesten daha kötü çözülebilir, bu yüzden güvenmeden önce havada deneyin. TCI sesi ve dijital modlar asla zincirden geçmez.
txaudio-test = Test
txaudio-rec = {" "}⏺ KAYIT{" "}
txaudio-play = {" "}▶ OYNAT{" "}
txaudio-play-test-phrase-through-chain = Test cümlesini, şu anda ayarlandığı haliyle zincirden hoparlörlerinize oynatın. Çalarken ayarlayın ve değişikliği duyun. Çalarken alma sesi yerine geçer.
txaudio-recording = kaydediliyor…
txaudio-no-phrase-yet = henüz cümle yok
txaudio-live = Canlı
txaudio-live-2 = {" "}🎧 CANLI{" "}
txaudio-hear-your-microphone-through-chain = Konuşurken mikrofonunuzu zincirden duyun; vericiyi tuşlamadan. Kulaklık kullanın: hoparlörle geri besleme yapar. Kendinizi birkaç on milisaniye geç duyarsınız, bu da konuşmayı zorlaştırır — seviyeleri ölçerlerden ayarlayın ve sesi KAYIT / OYNAT ile değerlendirin. Çalışırken alma sesi yerine geçer; PTT'ye basmak durdurur.
txaudio-meters = {" "}ÖLÇERLER{" "}
txaudio-run-your-microphone-through-chain = Mikrofonunuzu yalnızca ölçerler için zincirden geçirin: hoparlörde hiçbir şey yok, bu yüzden konuşmaya karşı gecikme ve geri beslenecek bir şey yok. Mikrofon ve giriş kazancını, kapı eşiğini ve kompresörün ne kadar çalıştığını ayarlamanın yolu budur. Alma sesi devam eder; PTT'ye basmak durdurur.
txaudio-monitoring-live-use-headphones-speakers = Canlı izleniyor — kulaklık kullanın, yoksa hoparlörler mikrofona geri beslenir.
txaudio-voice-modes-only-tci-clients = Yalnızca ses modları. TCI istemcileri ve dijital modlar zincirden atlanır; kullanımdayken radyonun kendi TX EQ'sunu ve işlemcisini düz tutun.
txaudio-auto-set-stopped-microphone-was = Otomatik ayar durdu: mikrofon alındı.
txaudio-microphone-may-clipping-before-gets = {" "}Mikrofon buraya gelmeden kırpılıyor olabilir — sert geliyorsa kendi kazancını kısın.
txaudio-level = Seviye
txaudio-mic-control-same-one-transmit = Mik denetimi — yayın çubuğundakiyle aynı, her profilde paylaşılır. Ortasında 0 dB, en üstte +6 dB.
txaudio-mic = Mik
txaudio-gain-top-mic-microphone-stored = Bu mikrofon için Mik'in üstüne kazanç, profille birlikte saklanır. Ön ayarların eşikleri, IN ölçerinde yaklaşık -10 dBFS'de tepe yapan bir ses varsayar; Otomatik ayar onu oraya koyar. BYPASS altında da geçerlidir ve dijital sese de uygulanır: bu bir işleme değil seviyedir.
txaudio-input = Giriş
txaudio-auto-set = Otomatik ayar
txaudio-auto-set-cancelled = Otomatik ayar iptal edildi.
txaudio-talk-now-you-would-air = Şimdi, havada yapacağınız gibi konuşun…
txaudio-db = -∞ dB
txaudio-chain = Zincir
txaudio-order-audio-meets-them = Sıraya göre ses onlarla karşılaşır
txaudio-switch-stage = Bu katı açın
txaudio-earlier-chain = Zincirde daha önce
txaudio-later-chain = Zincirde daha sonra
txaudio-output-limiter = Çıkış sınırlayıcısı
txaudio-output = Çıkış
txaudio-trim-limiter-always-last = Ayar ve sınırlayıcı — her zaman en sonda
txaudio-profiles = Profiller…
txaudio-gate-expander = Kapı / genişletici
txaudio-pushes-room-noise-fan-rumble = Duraklamalarda oda gürültüsünü, fan uğultusunu ve tıslamayı aşağı iter. Eşiği gürültü tabanınızın hemen üstüne ve en sessiz hecelerinizin altına koyun.
txaudio-gate = KAPI
txaudio-hard-gate-10-1-down = Sert kapı: 10:1, -40 dB'ye kadar
txaudio-expander = GENİŞLETİCİ
txaudio-gentle-expander-2-1-never = Yumuşak genişletici: 2:1, asla -15 dB'den fazla değil — oda hiçbir zaman ölü olmaz
txaudio-reset = SIFIRLA
txaudio-back-defaults = Varsayılanlara dön
txaudio-threshold = Eşik
txaudio-below-gate-starts-close = Bunun altında kapı kapanmaya başlar.
txaudio-return = Dönüş
txaudio-hysteresis-once-open-closes-only = Histerezis: açıldıktan sonra yalnızca eşiğin bu kadar altında kapanır; böylece eşikte gezinen bir ses kapının titremesine yol açmaz.
txaudio-hold = Tutma
txaudio-how-long-stays-open-after = Ses düştükten sonra ne kadar açık kaldığı; böylece kelime sonları kesilmez.
txaudio-release = Bırakma
txaudio-how-fast-closes-once-hold = Tutma bittikten sonra ne kadar hızlı kapandığı. Kapı pompalanıyorsa uzatın.
txaudio-floor = Taban
txaudio-most-ever-turns-pauses-down = Duraklamaları en fazla ne kadar kıstığı.
txaudio-ratio = Oran
txaudio-how-steeply-closes-below-threshold = Eşiğin altında ne kadar dik kapandığı.
txaudio-look-ahead = İleriye bakış
txaudio-opens-much-before-word-starts = Bir kelime başlamadan bu kadar önce açılır; böylece ilk ünsüzü kesilmez. Gönderilen her şeye aynı gecikmeyi ekler.
txaudio-high-pass-low-shelf-two = Yüksek geçiren, alçak raf, iki tepe, yüksek raf ve alçak geçiren. Eğri üzerindeki bir noktayı sürükleyin ya da aşağıdan ayarlayın. Alçak geçireni yayın filtrenizin genişliğine ayarlayın.
txaudio-flat = DÜZ
txaudio-every-band-back-0-db = Her bant 0 dB'ye dönsün, iki kenar da kapalı
txaudio-freq = Frek
txaudio-gain = Kazanç
txaudio-q-slope = Q / Eğim
txaudio-high-pass = Yüksek geçiren
txaudio-removes-handling-noise-breath-pops = Köşenin altında tutma gürültüsünü, nefes patlamalarını ve uğultuyu giderir.
txaudio-low-shelf = Alçak raf
txaudio-body-mud-cut-little-here = Gövde ya da çamur: kutu gibi çalan bir ses için burada biraz kesin.
txaudio-peak-1 = Tepe 1
txaudio-bump-dip-voice-band-often = Ses bandında bir tümsek ya da çukur — çoğu zaman 300–500 Hz civarında bir kesme.
txaudio-peak-2 = Tepe 2
txaudio-presence-lift-around-2-3 = Varlık: 2–3 kHz civarında bir yükseltme, SSB'de öne çıkan şeydir.
txaudio-high-shelf = Yüksek raf
txaudio-brightness = Parlaklık.
txaudio-low-pass = Alçak geçiren
txaudio-stops-energy-above-transmit-filter = Yayın filtresinin üstündeki enerjinin modülatörde harcanmasını önler.
txaudio-t-24-db-oct = 24 dB/okt
txaudio-t-12-db-oct = 12 dB/okt
txaudio-click-steeper-gentler-slope = Daha dik ya da daha yumuşak bir eğim için tıklayın
txaudio-de-esser = De-esser
txaudio-turns-down-only-sibilant-band = “s” ve “sh” eşiği geçtiğinde yalnızca tıslama bandını kısar; böylece sesin geri kalanı seviyesini korur. Frekansı “sister silver cellophane” derken tarayarak ve tıslamanın en yüksek olduğu yerde durarak bulun.
txaudio-frequency = Frekans
txaudio-centre-sibilant-band = Tıslama bandının merkezi.
txaudio-width-band-higher-narrower = Bandın genişliği — yüksek olması daha dar demektir.
txaudio-band-level-where-reduction-starts = Azaltmanın başladığı bant seviyesi. Çok düşükse sürekli çalışır ve sesi körleştirir.
txaudio-amount = Miktar
txaudio-most-band-ever-turned-down = Bandın en fazla ne kadar kısıldığı. -6 dB civarında başlayın.
txaudio-attack = Atak
txaudio-how-fast-reacts = Ne kadar hızlı tepki verdiği.
txaudio-how-fast-lets-go = Ne kadar hızlı bıraktığı.
txaudio-compressor = Kompresör
txaudio-brings-quiet-syllables-up-holds = Sessiz heceleri yükseltir, yüksek olanları geri tutar. Tepelerde 3–6 dB azaltma hedefleyin; 10 dB'nin üstü belirgin sıkıştırmadır, yarışmalar için daha uygundur.
txaudio-level-above-which-starts-compress = Bunun üstündeki seviyede sıkıştırmaya başlar.
txaudio-how-hard-compresses-above-threshold = Eşiğin üstünde ne kadar sert sıkıştırdığı.
txaudio-how-fast-reacts-peak-10 = Bir tepeye ne kadar hızlı tepki verdiği. 10–25 ms ünsüzleri keskin tutar.
txaudio-how-fast-recovers = Ne kadar hızlı toparlandığı.
txaudio-knee = Diz
txaudio-soft-knee-width-0-hard = Yumuşak diz genişliği; 0 sert dizdir.
txaudio-makeup = Telafi
txaudio-gain-after-compression-bring-level = Seviyeyi yeniden yükseltmek için sıkıştırmadan sonraki kazanç.
txaudio-drive = Sürüş
txaudio-gain-ahead-compressor-so-more = Kompresörün önündeki kazanç; böylece sesin daha fazlası eşiği geçer — ortalama gücü yükseltir. Çıkış sınırlayıcısıyla birlikte kullanın.
txaudio-phase = Faz
txaudio-all-pass-phase-rotator-ahead = Dedektörün önündeki tüm geçiren faz döndürücü. Bir sesin dengesiz tepelerini düzleştirir; böylece aynı tepe seviyesi daha fazla ortalama güç taşır. 4 yaygın bir yayın ayarıdır; 0 kapalıdır.
txaudio-last-stage-trim-look-ahead = Son kat: bir ayar ve tavanın altında her tepeyi tutan ileriye bakışlı bir sınırlayıcı; sert kırpmanın yaptığı saçılma olmadan. Varsayılan olarak kapalıdır, bu yüzden zinciri açmak zaten ayarladığınız bir sürüş seviyesini değiştirmez.
txaudio-limiter = Sınırlayıcı
txaudio-on = Açık
txaudio-ceiling = Tavan
txaudio-no-peak-leaves-chain-above = Zincirden bunun üstünde hiçbir tepe çıkmaz.
txaudio-trim = Ayar
txaudio-output-gain-ahead-limiter = Sınırlayıcının önündeki çıkış kazancı.
txaudio-off = {"  "}kapalı
txaudio-meters-move-while-you-transmit = Bir sesli yayın yaparken, test cümlesini çalarken ya da CANLI ya da ÖLÇERLER'i çalıştırırken ölçerler hareket eder.
txaudio-profiles-2 = Profiller
txaudio-profile-whole-chain-every-stage = Bir profil bütün zincirdir: her katın ayarları, hangilerinin açık olduğu, sıraları ve mikrofonunuz için giriş kazancı. Birini yüklemek zinciri değiştirir; Mik denetimi ve sürüş olduğu gibi kalır.
txaudio-name = Ad
txaudio-save = Kaydet
txaudio-store-chain-now-under-name = Zinciri şu an olduğu gibi bu ad altında saklayın; aynı adlı bir profilin yerine
txaudio-import = İçe aktar…
txaudio-read-profile-json-file = Bir profili .json dosyasından okuyun
txaudio-tx-audio-profile = TX ses profili
txaudio-no-profiles-yet-save-stores = Henüz profil yok. Kaydet, şu an kurduğunuz zinciri saklar.
txaudio-load = Yükle
txaudio-export = Dışa aktar…
txaudio-delete = Sil
txaudio-starting-points = Başlangıç noktaları
txaudio-set-your-level-auto-set = Önce seviyenizi Otomatik ayar ile ayarlayın — ön ayarların eşikleri bunu varsayar — sonra yüklenmiş haliyle yayın yapmak yerine buradan ince ayar yapın: bir cümle kaydedin ve her değişiklikten sonra dinleyin. Bir ön ayar giriş kazancınızı korur.
txaudio-profile-loaded = Profil yüklendi.

## Güneş sistemi 3B görünüm kaplaması (src/solar3d/overlay.rs)

solar-prop = YAYILIM
solar-where-signals-are-actually-getting =
    Sinyallerin, bant bant, gerçekten nereden geçtiği. Bu istasyonun katıldığı her alım — WSPR, FT8/FT4, JS8 ve günlük — uzaktaki istasyonda değil, yolunun orta noktasına, yani onu büken iyonosfer parçasına yerleştirilir. Yani bu, radyo amatörlerinin nerede olduğunun değil gökyüzünün bir haritasıdır.

    Yalnızca bu istasyonun bir ucu olduğu yolları gösterebilir. Okyanuslar, uzun yolların orta noktaları oraya düştüğü için aydınlanır; Antarktika ise oradan kimse yayın yapmadığı için karanlık kalır. Varsayılan olarak kapalıdır — tüm gezegeni boyar.
solar-orbits = YÖRÜNGELER
solar-orbital-paths = Yörünge yolları
solar-clouds = BULUTLAR
solar-cloud-cover-noaa-s-global = NOAA'nın küresel jeostasyoner mozaiğinden bulut örtüsü; yüzeye yapıştırılmış bir resim olarak değil, bir hava derinliği olarak çizilir: kızılötesi her bulut tepesinin ne kadar soğuk olduğunu söyler ve bu da ne kadar yüksekte durduğudur. Fırtınalar, kendi kurdukları kulelerin içinde çakar.
solar-planets = GEZEGENLER
solar-other-seven-planets-eighteen-their = Diğer yedi gezegen, onların on sekiz uydusu ve Satürn ile Uranüs'ün halkaları. Gezegen konumları JPL'nin Kepler eleman setinden gelir; uydular JPL Horizons'a uydurulmuş dairesel yörüngelerdir.
solar-coronal-mass-ejection-trajectory-cones = Koronal kütle atımı yörünge konileri
solar-sun-obs = GÜNEŞ GÖZLEMİ
solar-solar-observations-sun-s-disk = Güneşin diskindeki güneş gözlemleri: güneş lekesi aktif bölgeleri ve parlamaların nereden geldiği
solar-labels = ETİKETLER
solar-body-region-labels = Gök cismi ve bölge etiketleri
solar-small-bodies = KÜÇÜK CİSİMLER
solar-names-asteroids-comets-they-are = Asteroitlerin ve kuyrukluyıldızların adları. Her iki durumda da çizilirler — bu yalnızca otuz beş tanımlamanın onlarla birlikte çizilip çizilmeyeceğidir. Ne ayarlanmış olursa olsun, kameranın üstünde olduğu gök cismi ve arama kutusuyla eşleşen her şey adlı kalır.
solar-decoded-ft8-ft4-stations-path = Çözülmüş FT8/FT4 istasyonları ve çalışılan istasyona giden yol
solar-sats = UYDULAR
solar-amateur-radio-satellites-their-orbits = Amatör radyo uyduları, yörüngeleri ve QTH'nizden yükseklikleri
solar-aurora = KUTUP IŞIĞI
solar-auroral-oval-noaa-s-ovation = NOAA'nın OVATION modelinden kutup ışığı ovali; gerçek yüksekliklerinde yayılım kabukları olarak çizilir ve yüzeyde %10 konturunun ekvatora bakan kenarı işaretlenmiştir
solar-awards = ÖDÜLLER
solar-award-coverage-every-dxcc-entity = Ödül kapsamı: her DXCC varlığı, günlüğünüzün hâlâ neyi eksik bıraktığıyla boyanmış. Varlıkla hiç çalışmadığınız yer turuncu yanar, çalıştığınız ama onaylanmamış yer kehribar, bir QSL geri geldiyse soluk yeşil. ÖDÜLLER penceresindeki bant filtresini izler.
solar-view = GÖRÜNÜM
solar-which-body-camera-orbits-animated = Kameranın hangi gök cismi etrafında döndüğü ve animasyonlu tur
solar-layers = KATMANLAR
solar-what-drawn-orbits-clouds-cmes = Ne çizildiği: yörüngeler, bulutlar, CME'ler, etiketler, QSO'lar, kutup ışığı
solar-which-sdo-channel-wraps-sun = Hangi SDO kanalının Güneşi sardığı ve ne kadar eski olduğu
solar-weather = HAVA
solar-aurora-propagation-numbers-which-bands = Kutup ışığı, yayılım sayıları ve hangi bantların açık olduğu
solar-space-weather = UZAY HAVASI
solar-nothing-measured-yet = henüz hiçbir şey ölçülmedi
solar-scale = ÖLÇEK
solar-body-moon-orbit-exaggeration = Gök cismi ve Ay yörüngesi abartısı
solar-time = ZAMAN
solar-scrub-whole-scene-forward-back = Tüm sahneyi ileri ve geri kaydırın
solar-activity = ETKİNLİK
solar-replay-last-hour-decodes-globe = Küre üzerinde son bir saatin çözümlemelerini tekrar oynatın
solar-what-propagation-heat-map-showing = Yayılım ısı haritasının gösterdiği şey
solar-propagation = YAYILIM
solar-auto = ▶ OTO
solar-fly-spline-through-set-framed = Çerçevelenmiş bir bakış noktası kümesinden bir spline geçirin. Herhangi bir fare girdisi bunu iptal eder.
solar-moons-ride-circular-orbits-fitted = Uydular JPL Horizons'a uydurulmuş dairesel yörüngelerde ilerler: gerçekte oldukları yerden bir ya da iki derece içinde ve yörünge düzlemi bir daire tarafından izlenemeyecek kadar hızlı salınan Miranda için altıya kadar.
solar-fetch-everything-again-now = Her şeyi şimdi yeniden al
solar-all-sats = TÜM UYDULAR
solar-show-every-satellite-element-set = Eleman setindeki her uyduyu gösterin, yalnızca popüler olanları değil. Yörünge halkaları, küratörlü birkaçında kalır — aynı anda doksan tanesi okunmaz.
solar-starting = başlatılıyor…
solar-body = gök cismi{" "}
solar-moon-orbit = Ay yörüngesi{" "}
solar-stretch-earth-moon-distance-so = Dünya→Ay mesafesini, çifti ayrı görebilmek için uzatın
solar-sun = güneş{" "}
solar-sun-radius-exaggeration-leave-1 = Güneş yarıçapı abartısı. CME geometrisini okunur tutmak için 1× bırakın — şişkin bir Güneş her koninin tabanını yutar.
solar-show = GÖSTER
solar-paint-globe-what-getting-through = Küreyi, neyin geçtiğine göre boyayın
solar-paints-globe-what-getting-through = Küreyi, her bantta neyin geçtiğine göre boyar. Her alım, yolunun orta noktasına — iyonosferin işi yaptığı yere — uzaktaki istasyona değil yerleştirilir. Bu istasyonun duyduklarından ve Reverse Beacon Network açıkken dünyadaki skimmer'lardan kurulur.
solar-display = EKRAN
solar-all-bands = TÜM BANTLAR
solar-every-band-once-one-hue = Her bant bir kez, her biri kendi tonunda; ikisinin üst üste geldiği yerde karışarak. “Koşullar nasıl” için görünüm, “yirmi açık mı” için değil.
solar-one-band = TEK BANT
solar-one-band-cold-hot-blue = Tek bir bant, soğuktan sıcağa: mavi, yeşil, sarı, kırmızı.
solar-band = BANT
solar-nothing-heard-yet = henüz hiçbir şey duyulmadı
solar-bands = BANTLAR
solar-sources = KAYNAKLAR
solar-contacts-logbook-path-was-open = Günlükteki temaslar. Açık olan bir yol, ama değeri olan bir sinyal raporu yok — bir RST bir SNR değildir, bu yüzden bunlar bir hücrenin ne kadar meşgul olduğuna sayılır ve asla payına sayılmaz.
solar-stations-reported-hearing-one-downloaded = Bu istasyonu duyduklarını bildiren istasyonlar, WSPRnet'ten indirildi. Yayın yapan bir işaretin aldığı tek geri bildirim budur.
solar-reverse-beacon-network-skimmers-what = Reverse Beacon Network skimmer'ları: herkesin, her bantta duyduğu şey — bu radyonun olmadığı bantlar dahil. Diğerlerinden daha kabadır — bir RBN satırı lokatör taşımaz, bu yüzden iki uç da ülkelerinin merkezine yerleştirilir: küçük bir ülke için yakın, büyük bir ülke için çok yanlış. Ayarlar → Spotlar altında açın.
solar-decodes-station-made-each-measured = Bu istasyonun yaptığı çözümlemeler. Her biri kendi modunun tabanına göre ölçülür; böylece bir WSPR raporu ile bir FT4 raporu karşılaştırılabilir olur.
solar-memory = HAFIZA
solar-half-life = yarılanma ömrü{" "}
solar-how-long-reception-takes-count = Bir alımın yarısı kadar saymak için ne kadar sürdüğü. İyonosferin kendi hafızası kısadır — iki saat önceki bir açılım, iki dakika öncekiyle tartışmamalıdır.
solar-live = CANLI
solar-follow-band-happens = Bant olurken izleyin
solar-replay = ⏸ TEKRAR
solar-replay-2 = ▶ TEKRAR
solar-replay-last-hour-decodes-over = Son bir saatin çözümlemelerini, tekrar tekrar oynatın
solar-min-ago = {" "}dk önce
solar-where-last-hour-globe-showing = Son bir saatte kürenin nerede gösterdiği
solar-trail = iz{" "}
solar-how-long-decode-s-arc = Bir çözümlemenin yayının, başın arkasında küre üzerinde ne kadar kaldığı
solar-speed = hız{" "}
solar-how-much-faster-than-real = Tekrarın gerçek zamandan ne kadar hızlı oynadığı
solar-qso-layer-off = QSO katmanı kapalı
solar-no-decodes-yet = henüz çözümleme yok
solar-qth-not-set-enter-your = QTH ayarlı değil — Ayarlar'da lokatörünüzü girin
solar-mode = MOD
solar-path = YOL
solar-sent = GÖNDERİLEN
solar-rcvd = ALINAN
solar-grid = LOKATÖR
solar-dxcc = DXCC
solar-geostationary-always-above-your-horizon = Jeostasyoner — her zaman ufkunuzun üstünde.
solar-no-passes-next-48-hours = QTH'nizden önümüzdeki 48 saatte geçiş yok.
solar-start = BAŞLANGIÇ
solar-max-el = EN YÜKSEK YÜKSEKLİK
solar-aos-los-are-azimuths-horizon = AOS/LOS, ufuktaki azimutlardır. Zamanlar UTC'dir.
solar-locked-unlock = {" "}● KİLİTLİ — KİLİDİ AÇ{" "}
solar-release-satellite-lock = Uydu kilidini bırak
solar-lock = {" "}KİLİTLE{" "}
solar-track-satellite-main-window-doppler = Bu uyduyu ana pencerede izleyin: Doppler düzeltmeli alma ve yayın, transponder eşleşmeli yukarı bağlantı, döndürücü yönlendirmesi
solar-no-frequencies-file-one-add = Bu uydu için dosyada frekans yok — bunları Ayarlar ▸ TLE içinde ekleyin.
solar-frequencies-yours = FREKANSLAR · SİZİNKİLER
solar-frequencies = FREKANSLAR
solar-link = BAĞLANTI
solar-downlink-mhz = AŞAĞI BAĞLANTI (MHz)
solar-uplink-mhz = YUKARI BAĞLANTI (MHz)
solar-doppler-shifts-these-few-khz = Doppler, bir LEO geçişi boyunca bunları birkaç kHz kaydırır.
solar-sim = -- SİM
solar-satellite-planet-comet = uydu, gezegen ya da kuyrukluyıldız
solar-planet-asteroid-comet = gezegen, asteroit ya da kuyrukluyıldız
solar-clear-search = Aramayı temizle
solar-no-match = eşleşme yok
solar-no-sounder = sondaj cihazı yok
solar-heard = DUYULAN ≥
solar-kp = Kp / A
solar-x-ray = X-ışını
solar-observed-above-sounder-better-than = sondaj cihazının üstünde gözlemlendi — modellenenden daha iyi
solar-power-n-s = güç K/G
solar-edge-n-s = kenar K/G
solar-kp-peak-24-h = Kp tepe 24 sa
solar-observed-forecast = gözlemlenen | tahmin
solar-bands-open = BANTLAR AÇIK
solar-dxcc-coverage = DXCC KAPSAMI
solar-ir-vis = KIZILÖTESİ+GÖRÜNÜR
solar-ir-only = yalnızca KIZILÖTESİ
solar-no-deep-convection = derin konveksiyon yok
solar-t-1-storm = 1 fırtına
solar-glancing = {" "}· sıyırarak
solar-direction-estimated = {" "}· yön tahmin edildi

## Dijital mod kurulum iletişim kutuları (src/app/panels/setup.rs)

setup-aprs-setup = APRS kurulumu
setup-packet-setup = Paket kurulumu
setup-ft8-ft4-ft2-setup = FT8 / FT4 / FT2 kurulumu
setup-my-callsign = Çağrı işaretim
setup-my-grid = Lokatörüm
setup-station-call = İstasyon çağrısı
setup-ssid-oe3jjs-10-packet-station = Bir SSID ile — OE3JJS-10. Bir paket istasyonu geleneksel olarak operatörden farklı bir SSID kullanır ve bu ayarlanana dek hiçbir şey yayınlanmaz.
setup-speed = Hız
setup-t-9600-needs-radio-s-data = 9600, radyonun veri portunu gerektirir. Bir mikrofon ve hoparlör yolu her iki uçta da onu yok eder — bu sdroxide'ın değil radyonun sorunudur.
setup-tx-delay = Yayın gecikmesi
setup-flags-sent-before-frame-so = Bir çerçeveden önce gönderilen bayraklar; böylece uzak uç bizi duyabilir ve saatini kilitleyebilir. Bir CAT telsizinde yalnızca sdroxide, havaya çıkmak için 165–240 ms harcar ve telsizin kendi yayına hazır olma süresi buna eklenir.
setup-packet-length = Paket uzunluğu
setup-bytes = {" "}bayt
setup-most-single-frame-carries-shorter = Tek bir çerçevenin taşıdığı en fazla miktar. Daha kısa çerçeveler sınırda bir yolda hayatta kalır — yalnızca vurulan çerçeve yeniden gönderilir — ama daha fazla ek yük getirir. HF'de 128, yolun sağlam olduğu yerde 256.
setup-frames-sent-before-waiting-acknowledgement = Bir onay beklemeden önce gönderilen çerçeveler. Daha fazlası iyi bir yolu doldurur; kötüde ise yeniden gönderilecek daha fazla şey vardır.
setup-extended = Genişletilmiş
setup-ask-mod-128 = mod-128 iste
setup-extended-sequence-numbers-window-bigger = Yediden büyük bir pencere için genişletilmiş sıra numaraları. Birçok düğüm isteği bir DM ile reddeder; bu, tam olarak sizinle konuşmak istemeyen bir istasyon gibi görünür — bu yüzden uzak ucun bunu istediğini bilmiyorsanız kapalı bırakın.
setup-answer-calls = Çağrılara yanıt ver
setup-accept-connections = Bağlantıları kabul et
setup-off-winlink-client-which-dials = Dışarı çeviren ve yanıt vermek için hiçbir nedeni olmayan bir Winlink istemcisi için kapalı. Bir eş olarak erişilebilir olmak için açık: bir çağrı terminal bölmesine gelir ve onu yapanla konuşursunuz. Bağlantısı zaten bir Winlink oturumuyla meşgul olan bir istasyon, o bitene dek çağrıları reddeder.
setup-connect-text = Bağlantı metni
setup-what-caller-greeted = Bir arayan neyle karşılanır
setup-sent-station-connects-us-once = Bize bağlanan bir istasyona, bağlantı kurulunca gönderilir. Boş bırakılırsa hiçbir şey gönderilmez — bu, arayan için bozuk gibi görünür.
setup-default-via = Varsayılan üzerinden
setup-oe3xlr-1-oe3xms-1 = OE3XLR-1,OE3XMS-1
setup-digipeater-path-terminal-starts-path = Terminalin başladığı digipeater yolu. Yerel düğümünüze giden yol her seferinde aynıdır ve yeniden yazmak, bir atlamanın unutulmasının yoludur.
setup-beacon = İşaret
setup-t-0-disables-timer = 0 zamanlayıcıyı kapatır
setup-beacon-text = işaret metni
setup-kiss-server = KISS sunucusu
setup-serve = Sun
setup-offer-modem-kiss-tnc-socket = Bu modemi bir soket üzerinde KISS TNC olarak sunun; böylece Pat, bir APRS istemcisi ya da Linux AX.25 yığını telsizi kullanabilir.
setup-aprs-call = APRS çağrısı
setup-callsign-station-beacons-under-ssid = Bu istasyonun altında işaret verdiği çağrı işareti, SSID'siyle — bir araba için OE3JJS-9, bir I-gate için -10, bir telefon için -5. Yukarıdaki çağrı işaretini kullanmak için boş bırakın.
setup-no-callsign-station-will-not = Çağrı işareti yok: bu istasyon hiç yayın yapmaz — ne işaret, ne ileti, ne onay.
setup-symbol = Sembol
setup-two-characters-protocol-carries-them = Protokolün taşıdığı biçimdeki iki karakter: bir tablo (`/` ya da `\`) ve bir sembol kodu. İlk konumdaki bir rakam ya da harf, alternatif tablonun simgesinin üstüne çizilen bir kaplamadır.
setup-path = Yol
setup-wide1-1-wide2-1 = WIDE1-1,WIDE2-1
setup-how-far-you-ask-repeated = Tekrarlanmanızın ne kadar uzağa istendiği. Bu, kanaldaki en önemli ayardır: her atlama, tüm ağın tek bir paylaşılan frekansta yaptığı yayınları çarpar.
setup-position = Konum
setup-my-grid-2 = Lokatörümden
setup-beacon-centre-locator-above-reported = Yukarıdaki lokatörün merkezini, bir lokatörün gerçekten taşıdığı belirsizlikle birlikte işaretleyin — altı karakterlisi birkaç kilometre genişliğindedir ve bunu söylemek dürüsttür. Bunun yerine kesin koordinatlar vermek için işareti kaldırın.
setup-latitude = Enlem
setup-longitude = Boylam
setup-locator = lokatörden
setup-not-coordinate = koordinat değil
setup-comment = Yorum
setup-sent-every-beacon = Her işaretle birlikte gönderilir
setup-t-0-default-never-beacons-timer = 0 — varsayılan — zamanlayıcıyla asla işaret vermez. Sabit bir istasyon için otuz dakika geleneksel değerdir; hareket halindeki bir istasyon daha sık işaret verir, ama her işaret başkasının kanal zamanıdır.
setup-compressed = Sıkıştırılmış
setup-compressed-position-format-third-air = Sıkıştırılmış konum biçimi: havada süresinin üçte biri ve daha kesin. 1990'lardan beri her alıcı bunu okur.
setup-first-goes-out-one-interval = İlki, hemen değil şimdiden bir aralık sonra gider.
setup-messages = İletiler
setup-acknowledge = Onayla
setup-answer-messages-addressed-you-acknowledgement = Size gönderilen iletilere yanıt verin. Bir onay, bu istasyonun istenmeden yaptığı bir yayındır; bu yüzden yalnızca alan bir kurulum bunu kapatmalıdır — ve işaret de ulaşılabilir olduğunu iddia etmeyi bırakır.
setup-flags-sent-ahead-every-frame = Her çerçevenin önüne gönderilen bayraklar; böylece uzak ucun modemi taşıyıcıyı duyar ve veri başlamadan saatini kilitler. Yayın tuşuna basmakla ışımanın arasındaki her şeyden daha uzun sürmelidir: sdroxide tek başına bir CAT telsizinde bunun 165–240 ms'ini harcar ve ses ağ üzerinden alan bir telsiz buna daha fazla tampon ekler. Çok az olursa uzak uç hiç kilitlenmez — yayın oradadır ve hiçbir şey onu çözmez.
setup-shared-packet-mode = paket modu ile paylaşılır
setup-keep-stations = İstasyonları tut
setup-how-long-station-stays-map = Bir istasyonun en son duyulduktan sonra haritada ne kadar kaldığı. Ayrıca haritanın solmasının ölçüldüğü değer; bu yüzden kısa bir pencere yalnızca canlı olanı gösterir.
setup-auto-reply = Otomatik yanıt
setup-answer-snr-grid-status = SNR? / GRID? / STATUS? yanıtla
setup-answer-direct-question-addressed-you = Size ya da @ALLCALL'a yöneltilen doğrudan bir soruyu, bir onay yerine yanıtla yanıtlayın. Başka bir istasyonun trafiğini asla yanıtlamaz ve kendi kendini asla yanıtlamaz.
setup-heartbeat = Kalp atışı
setup-off = Kapalı
setup-turbo-does-not-beacon-local = Turbo işaret vermez — yerel ve VHF hızıdır.
setup-sends-your-callsign-grid-so = Çağrı işaretinizi ve lokatörünüzü gönderir; böylece başkaları alınabilir olduğunuzu bilir. İlki, hemen değil şimdiden bir aralık sonra gider.
setup-beacon-frequency = İşaret frekansı
setup-t-500-1000-hz = 500–1000 Hz
setup-move-each-beacon-free-slot = Her işareti, JS8Call'ın yaptığı gibi kalp atışı alt bandındaki boş bir yuvaya taşıyın: işaret arayan istasyonların baktığı yer burasıdır ve denetimsiz bir vericiyi başkasının QSO'sundan uzak tutar. Yuva, işaret gerçekten gönderildiğinde, çözülen her şeyden uzakta seçilir.
setup-working-freq = Çalışma frekansı
setup-beacon-where-you-are-working = Bunun yerine çalıştığınız yerde işaret verin. Bant geleneğine aykırı, ama yayınladığınız her şeyi tek bir yerde tutar.
setup-heartbeat-reply = Kalp atışı yanıtı
setup-answer-heartbeats-signal-report = Kalp atışlarını bir sinyal raporuyla yanıtla
setup-tell-station-beaconed-how-well = İşaret veren bir istasyona, onu ne kadar iyi aldığınızı bildirin. Varsayılan olarak kapalı: meşgul bir bant her yuvada bir kalp atışı taşır ve hepsini yanıtlamak, tam da kalp atışlarının sessiz tutmaya yaradığı bandı doldurur. Her istasyona 15 dakikada bir yanıtla sınırlıdır ve bir ileti hâlâ gelirken ya da gönderilmek üzere sıraya bir şey koymuşken asla yanıtlanmaz.
setup-status-message = Durum iletisi
setup-tx-period = Yayın periyodu
setup-even = Çift
setup-odd = Tek
setup-auto-sequence = Otomatik sıralama
setup-decode-depth = Çözme derinliği
setup-fast = Hızlı
setup-normal = Normal
setup-deep = Derin
setup-how-hard-ft8-decoder-works =
    FT8 çözücüsünün zayıf sinyaller için ne kadar çalıştığı.

    Hızlı — tek geçiş, sinyal çıkarma yok: en hızlısı, en az çözümleme.
    Normal — düz çok geçişli çıkarma: Derin'den biraz hızlı, biraz daha az titiz.
    Derin — kontrol noktalı çok geçişli: en çok çözümleme ve yoğun bir yuvada ~1,2 sn.
setup-auto-tx-frequency = Otomatik yayın frekansı
setup-choose-transmit-frequency-automatically-quietest = Yayın frekansını otomatik seçin: yayın yaptığınız periyotta en sessiz nokta, yanıt verdiğiniz istasyonun frekansı yerine. Kapalıyken BU TUTMAZ — çağrılan istasyonun frekansında yanıt verir. Tutmak için Yayın frekansını tut kullanın.
setup-hold-tx-frequency = Yayın frekansını tut
setup-pin-transmit-tone-where-nothing = Yayın tonunu olduğu yerde sabitleyin. Hiçbir şey onu hareket ettirmez: bir istasyona yanıt vermek, çağrı kuyruğu, CQ çağırmak ya da bir çözümlemeye ya da şelaleye tıklamak. Otomatik yayın frekansını geçersiz kılar. Bant değiştirmek tek istisnadır: yeni bantta en son ayarladığınız kayma onunla geri gelir. Lisansınız bant planından dar olduğunda — 5357 kHz'deki bir İngiltere 60 m skalasında tahsis 5358.0'da biter, bu yüzden ton 1000 Hz'in altında kalmalıdır.
setup-tx-watchdog = Yayın bekçisi
setup-stop-transmitting-after-long-no = Yanıt ve sizden hiçbir eylem olmadan bu kadar uzun süre sonra yayını durdurun. 0 kapatır.
setup-give-up-after = Şundan sonra vazgeç
setup-calls = {" "}çağrı
setup-unanswered-calls-one-station-before = Geçmeden önce bir istasyona yanıtsız çağrılar. CQ çağırmak bundan muaftır. 0 kapatır.
setup-contest = Yarışma
setup-everyday-exchange-grid-report-rr73 = Günlük alışveriş: lokatör, rapor, RR73.
setup-european-vhf-contests-send-signal = Avrupa VHF yarışmaları: WSJT-X'in EU VHF Contest dediği ileti düzeninde bir sinyal raporu, bir seri numarası ve altı karakterli lokatörünüzü gönderin. CQ, CQ TEST olarak gider.
setup-serial = Seri
setup-serial-number-next-exchange-will = Bir sonraki alışverişin taşıyacağı seri numarası. Her temas günlüğe yazıldıkça kendiliğinden ilerler ve oturumlar arasında hatırlanır — yalnızca bir yarışmaya başlamak ya da kağıt günlüğün kaldığı yerden devam etmek için elle ayarlayın.
setup-needs-6-character-grid = ⚠ altı karakterli lokatör gerektirir
setup-eu-vhf-contest-messages-carry = EU VHF Contest iletileri altı karakterli bir lokatör taşır (JN88 değil JN88DD). Lokatörüm bunu içerene dek bunun yerine olağan alışveriş gönderilir.
setup-dxpedition = DXpedition
setup-ordinary-ft8-operation = Olağan FT8 işletimi.
setup-calling-dxpedition-running-fox-mode = Fox modunda çalışan bir DXpedition'ı çağırmak: 1000 Hz'in üstünden çağırın, yanıt verdiğinde Fox'a doğru aşağı inin ve 73 göndermeden onun RR73'ünden kaydedin.
setup-run-pile-up-several-signals = Yığını yönetin: aynı anda birkaç sinyal, çağıranların bir kuyruğu, en güçlü ve en nadir olandan başlayarak çalışılır. CQ ÇAĞRISI onu başlatır, QSO'YU DURDUR onu bırakır.
setup-fox-signals = Fox sinyalleri
setup-once = {" "}bir kerede
setup-simultaneous-transmissions-spaced-60-hz = Birbirinden 60 Hz aralıklı eş zamanlı yayınlar. Verici gücünü paylaşırlar, bu yüzden daha fazla sinyal her birini daha zayıf yapar.
setup-tx-audio = TX sesi
setup-how-loud-mode-s-burst =
    Bu modun patlamasının, onu kendisi modüle eden bir radyoya ne kadar yüksek verildiği — FM'de bu sapmadır, çünkü bir FM verici ses seviyesini frekans salınımına çevirir ve onu yakalayacak bir ALC'si yoktur. 1200 baud paket yaklaşık 3 kHz ister, ses ise 5 kHz; bu yüzden konuşma için ayarlanmış bir veri girişine tam ölçek aşırı sapar — bu, dinleyiciye tamamen normal gelir ve kimse için çözülmez. Diğer istasyonlar sizi bildirene dek kısın ya da seviyeyi radyoda ayarlayın.

    Bu seviye yalnızca bu moda aittir. Hiç ayarlanmamış bir mod FM değerinden başlar, yan bant modları kendi değerlerinden — bu yüzden 1200 baud için ayarlanan bir sapma hiçbir zaman FT8'e geçmez.
setup-how-loud-mode-s-over =
    Bu modun yayınının, onu kendisi modüle eden bir radyoya ne kadar yüksek verildiği — ses kartındaki bir CAT telsizi, bir FLEX, ağ portundaki bir Icom. Yan bantta bu modülatöre sürüştür: telsizin ALC'si zar zor hareket edene dek kısın ve gücü radyoda ayarlayın, çünkü sabit zarflı bir modda ALC'nin yükselmesi saçılmaya yol açar. Sürüş burada telsizin güç yazmacına ulaşır, sesine değil, bu yüzden bu seviyedir.

    Bu seviye yalnızca bu moda aittir: FT8, RTTY, PSK ve MCW kendi değerlerini tutar, çünkü bir modülatörü aynı şekilde yüklemezler. Aynı ray, yayın şeridinde de bulunur; yayın yaparken oraya ulaşabilirsiniz.
setup-deviation-fm = FM'de sapma
setup-drive-ssb = SSB'de sürüş
setup-grid = Lokatör
setup-report = Rapor
setup-r-report = R+Rapor

## AMP penceresi (src/app/amp.rs)

amp-t-10-w-scale-rescaled = , 10 W ölçeği, yeniden ölçeklendi
amp-t-10-w-scale = , 10 W ölçeği
amp-t-2-w-scale-rescaled = , 2 W ölçeği, yeniden ölçeklendi
amp-t-2-w-scale = , 2 W ölçeği
amp-t-100-w-scale-rescaled = , 100 W ölçeği, yeniden ölçeklendi
amp-t-100-w-scale = , 100 W ölçeği
amp-t-20-w-scale-rescaled = , 20 W ölçeği, yeniden ölçeklendi
amp-t-20-w-scale = , 20 W ölçeği
amp-rescaled = , yeniden ölçeklendi
amp-and = {" "}VE{" "}
amp-not-while-transmitting-switching-relay = Yayın yaparken değil: üzerinde RF varken bir röleyi anahtarlamak onu zarar görebilir. Önce tuşu bırakın.
amp-no-amplifier-set-up-choose = Hiçbir amplifikatör kurulmadı. Portunu Ayarlar → Amplifikatör altından seçin.
amp-settings = AYARLAR
amp-dock = YERLEŞTİR
amp-put-amp-window-back-inside = AMP penceresini yeniden ana pencerenin içine koyun
amp-pop-out = AYIR
amp-give-amp-window-window-own = AMP penceresine kendi penceresini verin; başka bir ekrana taşınabilsin
amp-rig = , telsizden
amp-rig-flagged-reading = , telsiz okumayı işaretledi
amp-drv = , DRV'den
amp-rig-s-swr = , telsizin SWR'sinden
amp-ptt-keys-amplifier-radio-off = PTT, amplifikatörü radyoyla birlikte anahtarlar; KAPALI onu atlamada bırakır. Değiştirmek için tıklayın.
amp-switch-antenna-jack = Anten jakını değiştir
amp-band-down = Bant aşağı
amp-band-up = Bant yukarı
amp-tuner = TUNER
amp-tuned = Ayarlandı
amp-tune = AYARLA
amp-start-tune-then-key-steady = Bir ayarlamayı başlatın — sonra sabit bir taşıyıcıyı tuşlayın (CW, FM ya da RTTY). Durdurmak için yeniden basın.
amp-put-tuner-line-bypass = Tuner'ı hatta alın ya da atlayın
amp-tuning = Ayarlanıyor…
amp-bottom = ALT
amp-pick-any-reading-either-needle = İki ibreden herhangi biri için bir okuma seçin. Üst ibre beyaz, alt ibre mavidir.
amp-forward-power-measured-radio-amplifier = İleri güç, radyo tarafından ölçülür: amplifikatör KAPALI'da bir değer bildirmez
amp-forward-power-taken-drv-amplifier = İleri güç, DRV'den alınır: amplifikatör KAPALI'da bir değer bildirmez ve atlamada radyonun gücü doğrudan geçer
amp-forward-power = İleri güç
amp-reflected-power-measured-radio-amplifier = Yansıyan güç, radyo tarafından ölçülür: amplifikatör KAPALI'da bir değer bildirmez
amp-reflected-power-worked-out-fwd = Yansıyan güç, FWD ve SWR'den hesaplanır: amplifikatör KAPALI'da bir değer bildirmez
amp-reflected-power = Yansıyan güç
amp-standing-wave-ratio-red-past = Duran dalga oranı: ayarlama sınırında ya da üstünde kırmızı (Ayarlar → Genel). Anteni denetleyin
amp-standing-wave-ratio-measured-radio = Duran dalga oranı, radyo tarafından ölçülür: amplifikatör KAPALI'da bir değer bildirmez
amp-standing-wave-ratio = Duran dalga oranı
amp-drive-radio = Radyodan sürüş
amp-drain-supply = Drenaj beslemesi
amp-drain-current = Drenaj akımı
amp-temp = SICAKLIK
amp-heat-spreader-temperature-yellow-49 = Isı yayıcı sıcaklığı: 49 °C'den (120 °F) sarı, 65 °C'nin (149 °F) üstünde kırmızı, 70 °C'nin (158 °F) üstünde hattan çıkar. °C/°F için tıklayın.
amp-c = °C
amp-f = °F
amp-performance = PERFORMANS
amp-numbers-hold-peaks-bars-are = sayılar tepeleri tutar · çubuklar canlıdır
amp-input-power = GİRİŞ GÜCÜ
amp-supply-volts-drain-amps = Besleme voltu × drenaj amperi
amp-efficiency = VERİM
amp-output-power-input-power = Çıkış gücü ÷ giriş gücü
amp-gain = KAZANÇ
amp-output-power-over-drive-decibels = Sürüşe göre çıkış gücü, desibel cinsinden
amp-off-line-over-temperature = HATTA DEĞİL · aşırı sıcaklık
amp-connected-transmitting = BAĞLI · YAYIN YAPIYOR
amp-connected = BAĞLI
amp-connecting = Bağlanıyor…
amp-atu-bypassed = ATU ATLANDI
amp-no-inductance-needed = endüktans gerekmez
amp-no-capacitance-needed = kapasitans gerekmez
amp-c-out-antenna-side = C OUT · anten tarafı
amp-c-transmitter-side = C IN · verici tarafı

## SSTV paneli (src/app/panels/sstv.rs)

sstv-chunks-received-lit-still-missing = Alınan parçalar (yanan) ve hâlâ eksik olanlar (sönük)
sstv-sstv = SSTV
sstv-auto = Otomatik
sstv-signal = Sinyal
sstv-station-sending-mode-sdroxide-does = Bir istasyon, sdroxide'ın sahip olmadığı bir modda yayın yapıyor. Sinyal ve ayar iyi — bunun için yalnızca bir çözücü yok. Sizin tarafınızda düzeltilecek bir şey yok.
sstv-listening = dinleniyor…
sstv-restart-rx = Alımı yeniden başlat
sstv-abandon-picture-being-received-listen = Alınmakta olan resmi bırakın ve bir sonraki başlığı dinleyin. Yanlış modda çözülmeye başlayan bir yayın için — aksi halde alıcı, o mod bitene dek bağlı kalır.
sstv-callsign-last-station-sent-fsk = Son istasyonun resminden sonra bir FSK kimliği olarak gönderdiği çağrı işareti.
sstv-tx-slant = YAYIN eğimi
sstv-transmit-clock-trim-ppm-remove = Uzak uçtaki çözücüdeki eğimi gidermek için yayın saat ayarı (ppm)
sstv-reset-0-ppm = 0 ppm'e sıfırla
sstv-fsk-id = FSK kimliği
sstv-send-your-callsign-tones-after = Her resimden sonra çağrı işaretinizi tonlarla gönderin — SSTV tekrarlayıcılarının ve diğer programların okuduğu kimlik. Yaklaşık 2,5 saniye ekler ve siz bir çağrı işareti ayarlayana dek hiçbir şey göndermez.
sstv-tx-lead = YAYIN öncüsü
sstv-silence-sent-after-keying-before = Tuşlamadan sonra, resmin öncüsünden ve VIS kodundan önce gönderilen sessizlik. Bu başlığın herhangi birini kaçıran bir çözücü hiç resim göstermez; bu yüzden bu, bir telsizden PTT istemek ile gerçekten havaya çıkması arasındaki boşluğu kapsar. Anında tuşlayan bir SDR için 0.
sstv-live = CANLI
sstv-waiting-picture = bir resim bekleniyor…
sstv-waiting-signal = bir sinyal bekleniyor…
sstv-no-low-audio = ses yok / düşük
sstv-received = ALINDI
sstv-reading-radio-s-pictures = Radyonun resimleri okunuyor…
sstv-decoded-pictures-collect-here = Çözülen resimler burada toplanır.
sstv-click-enlarge-right-click-delete = Büyütmek için tıklayın · silmek için sağ tıklayın
sstv-delete-picture = Bu resmi sil
sstv-loading = yükleniyor…
sstv-transmit = YAYINLA
sstv-click-edit-slot-s-message = Bu yuvanın iletisini düzenlemek için tıklayın · bir görüntü yüklemek için çift tıklayın
sstv-change-image = Görüntüyü değiştir…
sstv-load-image = Görüntü yükle…
sstv-clear = Temizle
sstv-empty-slot-s-picture-message = Bu yuvanın resmini boşaltın (ileti kalır)
sstv-banner = Afiş…
sstv-what-printed-across-top-every = Bu istasyonun gönderdiği her resmin üstüne basılan şey
sstv-fetching-slot-s-picture = Bu yuvanın resmi alınıyor…
sstv-load-image-into-slot = Bu yuvaya bir görüntü yükleyin →
sstv-drawn-slot-s-image = Bu yuvanın görüntüsünün üstüne çizilir
sstv-tx = {"   "}YAYIN{"   "}
sstv-abort-tx = {" "}YAYINI İPTAL ET{" "}
sstv-delete-picture-store = Bu resmi depodan sil
sstv-received-image = Alınan görüntü
sstv-no-longer-store = artık depoda değil
sstv-loading-full-size = tam boyut yükleniyor…
sstv-save-picture = Resmi kaydet…
sstv-save-copy-computer = Bu bilgisayara bir kopya kaydedin
sstv-delete-sure = Sil — emin misiniz?
sstv-click-again-delete-good = Kalıcı olarak silmek için yeniden tıklayın
sstv-delete = Sil…
sstv-image-banner = Görüntü afişi
sstv-drawn-into-every-picture-station = Bu istasyonun yayınladığı her resme, yuvanın kendi iletisinin üstüne çizilir.
sstv-draw-banner = Afişi çiz
sstv-substituted-when-picture-composed = Resim oluşturulurken yerine konur:
sstv-top-left = Sol üst
sstv-top-right = Sağ üst
sstv-colours = Renkler
sstv-strip-top-edge-fades-black = Şerit, üst kenarında — siyaha ya da altındaki geçiş rengine solar
sstv-both-texts = Her iki metin
sstv-gradient = Geçiş
sstv-fade = şuna solsun
sstv-fade-strip-top-colour-second = Şeridi siyaha değil, üst renginden ikinci bir renge soldurun.
sstv-text-outline = Metin dış çizgisi
sstv-draw-outline-around-banner-text = Afiş metninin etrafına bir dış çizgi çizin; böylece yoğun ya da açık bir şeritte okunur kalır.
sstv-text-gradient = Metin geçişi
sstv-fade-banner-text-colour-second = Afiş metnini, resmin genişliği boyunca renginden ikinci bir renge soldurun.
sstv-rainbow-text = Gökkuşağı metni
sstv-all-text-overriding-colours-above = bütün metin, yukarıdaki renkleri geçersiz kılarak
sstv-draw-every-text-picture-banner = Resimdeki her metni — afiş ve ileti — yukarıdaki renkleri yok sayarak yatay bir gökkuşağı olarak çizin.
sstv-message = İleti
sstv-slot-message-s-text-colour = Yuva iletisinin metin rengi
sstv-outline = dış çizgi
sstv-height = Yükseklik
sstv-how-tall-strip-pixels-transmitted = Şeridin, yayınlanan resmin piksel cinsinden ne kadar yüksek olduğu. Metin ondan boyutlandırılır — bir SSTV karesi yalnızca 320 piksel genişliğindedir, bu yüzden daha yüksek bir afiş onu uzak uçta okunur kılan şeydir.
sstv-reset = Sıfırla
sstv-back-callsign-left-version-right = Sol tarafta çağrı işaretine ve sağ tarafta sürüme geri dön
sstv-waiting-station-s-digital-mode = İstasyonun dijital mod ayarları yüklenene dek bekleniyor…
sstv-rifp = RIFP
sstv-size = Boyut
sstv-calling-frequency-draft-names = Taslağın adlandırdığı çağrı frekansı
sstv-encode = Kodla
sstv-how-picture-encoded-into-object = Resmin RIFP'nin taşıdığı nesneye nasıl kodlandığı. Otomatik her birini dener ve en küçüğünü gönderir.
sstv-try-every-encoding-send-smallest = Her kodlamayı dene, en küçüğünü gönder (asla kayıplı değil)
sstv-gray = Gri
sstv-grayscale-depth-rifp-s-raster = Gri tonlama derinliği. RIFP'nin rasteri tanım gereği gri tonlamalıdır — renk manifestoda yer almaz.
sstv-dither = Dither
sstv-diffuse-quantisation-error-worth-below = Nicemleme hatasını yay — 8 bitin altında değer
sstv-caption = Başlık
sstv-sent-manifest-content-hint-shown = Manifestoda içerik ipucu olarak gönderilir ve alıcılar tarafından resmin altında gösterilir. Metin olarak taşınır — görüntüye çizilmez.
sstv-what-picture = Bu resim nedir
sstv-repeat-data = Veriyi tekrarla
sstv-send-every-data-frame-many = Her veri karesini bu kadar kez gönderin. RIFP tek yönlüdür ve onarım istekleri yoktur, bu yüzden alıcının alabildiği tek kurtarma budur.
sstv-chunk = Parça
sstv-payload-octets-per-data-frame = Veri karesi başına yük oktet sayısı (profil 192 önerir)
sstv-valid-frames-frames-failed-crc = Geçerli kareler, CRC'si tutmayan kareler ve tam doğrulanmış resimler
sstv-no-transfer-progress = devam eden bir aktarım yok
sstv-forget-incomplete-transfer = Bu tamamlanmamış aktarımı unut

## Hafızalar ve ses tuşlayıcı pencereleri (src/app/windows.rs)

windows-recall = Geri çağır
windows-delete = Sil
windows-edit-name-frequency-mode-antenna = Adı, frekansı, modu, anteni ve tekrarlayıcı ayarını düzenleyin
windows-r = {" "}R
windows-repeater-shift-tone-1750-hz = Bu kanalla saklanan tekrarlayıcı kayması, ton ve 1750 Hz patlaması
windows-abandon-edit-escape = Düzenlemeden vazgeç (Escape)
windows-save = KAYDET
windows-keep-changes-enter = Değişiklikleri koru (Enter)
windows-recall-channel-without-moving-antenna = Bu kanalı anteni taşımadan geri çağırın
windows-antenna-switch-when-channel-recalled = Bu kanal geri çağrıldığında geçilecek anten. — radyonun zaten olduğu yerde bırakır.
windows-shift = Kayma
windows-tone = Ton
windows-invert = TERS
windows-normal = NORMAL
windows-dcs-polarity = DCS kutupluluğu
windows-open-every-over-tone-burst = Bu kanal geri çağrıldığı sürece her yayını ton patlamasıyla açın
windows-memories = Hafızalar
windows-memory-name = hafıza adı
windows-store = Kaydet
windows-folder-name = klasör adı
windows-new-folder = Yeni klasör
windows-import = İÇE AKTAR
windows-read-channel-list-chirp-csv =
    Bir kanal listesini bir CHIRP CSV dosyasından (.csv) okuyun — bir tekrarlayıcı dizini dışa aktarımı, bir deniz ya da PMR kanal tablosu ya da CHIRP'in yazabildiği herhangi bir şey.

    Her kanal, frekansını, modunu, tekrarlayıcı kaymasını ve CTCSS ya da DCS tonunu beraberinde getirir. Bu listede zaten bulunan kanallar — aynı frekans, aynı mod — atlanır; bu yüzden güncellenmiş bir dizini yeniden içe aktarmak, iki katına çıkarmak yerine yalnızca yeni olanı ekler.
windows-chirp-csv = CHIRP CSV
windows-export = DIŞA AKTAR
windows-write-list-out-chirp-csv = Bu listeyi bir el cihazına yüklemek ya da yedek olarak tutmak için bir CHIRP CSV dosyası olarak yazın.
windows-no-memories-yet = henüz hafıza yok
windows-drag-memory-onto-folder-file = Bir hafızayı kaydetmek için bir klasörün üzerine sürükleyin; ayırmak için onların altına
windows-drop-here-unfile = Ayırmak için buraya bırakın
windows-sort = Sırala
windows-order-they-were-stored = Kaydedildikleri sırayla
windows-name-ignoring-case = Ada göre, büyük-küçük harf gözetmeksizin
windows-frequency = Frekansa göre
windows-band-then-frequency-inside-each = Bant, sonra her bant içinde frekansa göre
windows-descending-click-ascending = Azalan — artan için tıklayın
windows-ascending-click-descending = Artan — azalan için tıklayın
windows-delete-folder-memories-move-top = Klasörü sil — hafızaları en üst düzeye taşınır
windows-rename-folder = Klasörü yeniden adlandır
windows-empty-drop-memories-here = boş — hafızaları buraya bırakın
windows-voice-keyer = Ses tuşlayıcı
windows-rec-records-your-microphone-play = KAYIT mikrofonunuzdan kaydeder, OYNAT kaydettiğinizi dinlemenizi sağlar, YAYIN havaya verir — sayısal tuş takımındaki bir tuş, bir MIDI pedi ya da rigctld'nin send_voice_mem komutu da öyle.
windows-stop-store = Durdur ve kaydet
windows-stop = DURDUR
windows-play = OYNAT
windows-stop-listening = Dinlemeyi durdur
windows-nothing-recorded-slot = Bu yuvada hiçbir şey kaydedilmedi
windows-not-while-transmitting = Yayın yaparken değil
windows-listen-message-nothing-transmitted = Bu iletiyi dinleyin — hiçbir şey yayınlanmaz
windows-stop-transmitting = Yayını durdur
windows-tune-active-switch-off-first = AYAR etkin — önce kapatın
windows-transmit-message = Bu iletiyi yayınla
windows-erase-recording = Bu kaydı sil
windows-rade-message-encoded-digital-voice = RADE: ileti, canlı bir yayın olacağı gibi dijital ses kodlayıcısı tarafından kodlanır.

## TX Audio penceresi: değerli iletiler

txaudio-rec-hint = Mikrofondan bir test cümlesi kaydedin — en fazla { $max } sn, vericiyi tuşlamadan. Durdurmak için yeniden tıklayın. İşlenmemiş tutulur, böylece OYNAT yaptığınız her değişiklikten aynı cümleyi geçirebilir.
txaudio-phrase-length = { $secs } sn cümle
# $clip, txaudio-microphone-may-clipping-before-gets ya da boştur.
txaudio-auto-set-done = Giriş kazancı { $gain } dB olarak ayarlandı (ses tepeleri { $loud } dBFS idi).{ $clip }
txaudio-listening = Dinleniyor… { $secs } sn
txaudio-auto-set-hint = { $secs } sn normal konuşun — havada yapacağınız gibi — ve giriş kazancı, sesiniz { $target } dBFS civarında tepe yapacak şekilde ayarlanır. İptal etmek için yeniden tıklayın.
txaudio-eq-point = { $hz } Hz, { $db } dB — taşımak için sürükleyin
txaudio-not-a-profile = Bir TX ses profili değil: { $error }
txaudio-imported = “{ $name }” içe aktarıldı.
txaudio-saved = “{ $name }” kaydedildi.
txaudio-deleted = “{ $name }” silindi.
txaudio-loaded-preset = “{ $name }” başlangıç noktası yüklendi.

## Güneş sistemi 3B görünüm: değerli iletiler

solar-age-offline = { $age } · çevrimdışı
solar-radius-exaggeration = Dünya/Ay yarıçapı abartısı (bu Ay yörüngesi ölçeğinde en fazla { $max }×)
solar-brightest-cell = en parlak hücre ≈ { $peak } yol · { $bands } { $bands ->
        [one] bant
       *[other] bant
    }
solar-hits-in-hour = saatte { $hits }
solar-passes-from = { $name } · { $grid } KONUMUNDAN GEÇİŞLER
solar-elements-age = elemanlar { $age } eski · SGP4
solar-point-at = { $az }° ({ $compass }) yönüne bakın, yükseklik { $el }°.
solar-published-as = { $name } olarak yayımlandı
solar-fly-there = { $name } — oraya uçmak için ↵
solar-sats-tracked = { $total } içinden { $n } izleniyor
solar-bodies-found = { $total } içinden { $n } gök cismi
solar-sats-and-bodies = { $total } içinden { $n } izleniyor · { $bodies_total } içinden { $bodies } gök cismi
solar-confidence-on-band = { $band } üzerinde { $confidence }
solar-kp-now = şu an Kp { $kp }
solar-kp-in = { $hours } sa içinde Kp { $kp }
solar-valid-age = geçerli { $valid } · { $age } eski
solar-full-bar = dolu çubuk = dünyanın { $share } kısmı
solar-own-paths-memory = bu istasyonun kendi yolları nereye gitti · { $minutes } dk hafıza
solar-n-storms = { $n } fırtına
solar-clouds-legend = BULUTLAR { $time } · { $age } eski · { $channels } · { $storms } · şimşek simüle edildi
solar-cme-eta = ETA { $eta } (+{ $hours } sa)
solar-cme-arrived = varış { $eta } idi ({ $hours } sa önce)
# $glancing ve $estimated, solar-glancing / solar-direction-estimated ya da boştur.
solar-cme-earth-directed = DÜNYA'YA YÖNELİK CME { $time } · { $speed } km/sn · { $when }{ $glancing }{ $estimated }

## Dijital mod kurulum iletişim kutuları: değerli iletiler

setup-mode-setup = { $mode } kurulumu
setup-transmits-as = { $call } olarak yayın yapar
setup-decimal-degrees = Ondalık derece, { $hemisphere } için negatif. Doğrudan bir haritadan yapıştırın.

## AMP penceresi: değerli iletiler

amp-unreliable-at-swr = { $meters } BU SWR'DE GÜVENİLİR DEĞİL
amp-lower-drive-first = Önce sürüşü düşürün: { $drive } W, hattaki amplifikatörü aşırı sürerdi (sınırı { $limit } W).
amp-live-value = canlı { $value } { $unit }
amp-tripped = KESİLDİ{ $why }

## Dijital mod panelleri, ortak: değerli iletiler

panels-top-paths = en üst ≈ { $n } yol
panels-no-agreed-frequency = { $mode } için anlaşılmış bir frekans yok — üzerinde olduğunuzu kaydetmek için bunu açın
panels-mode-frequencies-hint = { $mode } için { $n } frekans — birini seçmek skalayı ayarlar ve kendinizinkini kaydedebilirsiniz
panels-mode-frequencies = { $mode } · frekanslar
panels-global-convention = IARU { $region } Bölgesi bant planının dar veri koymadığı genel bir gelenek — buradan yayın yapmadan önce kendi bant planınızı denetleyin.
panels-outside-data-segment = Kehribar: { $region } Bölgesi veri segmentinin dışında.
panels-save-dial = ＋ { $mhz } MHz kaydet
panels-save-dial-hint = Bu skalayı { $mode } altında hatırlayın ve bundan sonra burada sunun. İstasyonda kaydedilir; bu yüzden bu radyoya bağlı her ekranda bulunur.
panels-tr-period = { $period } saniyelik T/R periyodu
# $mark, yayın sırasında panels-burst-mark, aksi halde boştur.
panels-this-turn = Bu sıra: { $slot } içinde { $into } sn. Çözümler sonunda gelir ve bir sonraki yayın ancak o zaman başlayabilir.{ $mark }
panels-burst-mark = {" "}İşaret, patlamanın { $secs } sn'de durduğu yerdir.
panels-tr-period-burst = { $slot } saniyelik T/R periyodu, { $burst } sn patlama

## Hafızalar ve ses tuşlayıcı pencereleri: değerli iletiler

windows-n-channels = { $n } { $n ->
        [one] kanal
       *[other] kanal
    }
windows-slot-n = Yuva { $n }
windows-record-from-mic = Mikrofondan kaydet (en fazla { $max } sn)

## SSTV paneli: değerli iletiler

sstv-picture-too-big = Bu resim { $size } MB — sınır { $limit } MB.
sstv-auto-mode = Otomatik ({ $mode })
sstv-not-decoded = { $what } — çözülmedi
sstv-last = son: { $mode }
# $place, sstv-store-on-radio ya da sstv-store-in'dir.
sstv-saved-where = Çözülen resimler { $place } kaydedilir ve burada toplanır.
sstv-older-in-store = Depoda { $n } daha eski
sstv-older-load-more = { $n } daha eski — daha fazlasını yükle
sstv-delete-picture-from = Bu resmi { $place } sil
sstv-rifp-profile = { $profile } · yaklaşık { $khz } kHz kaplar · skala kanal merkezidir
sstv-rifp-too-wide = ⚠ { $profile } yaklaşık { $khz } kHz kaplar — dar bantlı bir segment için çok geniş
sstv-rifp-no-frequency = RIFP bir frekans atamaz ve sdroxide, hangi yere ayarlarsanız orada yayınlar. { $khz } kHz'lik bir kanal yalnızca geniş bant ya da FM işletimine izin verilen yerlerde sığar — { $segments } — ve dar bantlı bir segmentte değil, en az HF'de. Bunların içinde bile kendi lisans koşullarınız daha dar olabilir. Operatör sizsiniz; kendi kurallarınızı denetleyin.
sstv-rifp-tx-frame = ● YAYIN karesi { $frame }/{ $frames } · { $secs } sn kaldı
sstv-rifp-sent-as = { $encoding } olarak gönderildi · { $octets } oktet
sstv-rifp-rx-stats = kareler { $frames } · bozuk { $bad } · resimler { $pictures }
sstv-rifp-session-idle = oturum { $session } · boşta { $secs } sn
sstv-rifp-session-waiting = oturum { $session } · parçalar tutuluyor, hâlâ manifesto bekleniyor · boşta { $secs } sn
sstv-store-on-radio = radyoda, { $dir } içinde
sstv-store-in = { $dir } içinde

## Çözümler (src/app/panels/decodes.rs)

decodes-decodes = ÇÖZÜMLER
decodes-adif = ADIF
decodes-sort = Sırala
decodes-single-list = Tek liste
decodes-every-decode-one-list-newest = Her çözümleme tek bir listede, en yeni sıra önce; tek/çift sıra blokları yerine. Sıralama çipleri bunun yerine her sırayı değil tüm listeyi sıralar ve her satır yuva zamanını taşır — çift yuva için camgöbeği, tek yuva için altın.
decodes-cq-only = Yalnızca CQ
decodes-only-stations-calling-cq-only = Yalnızca CQ çağıran istasyonlar — ve yalnızca yanıt verebileceğiniz çağrılar: yönlendirilmiş bir CQ (DX, EU, JA, POTA, TEST …) size adını verdiğinde listelenir, başkasının adını verdiğinde gizlenir.
decodes-new-only = Yalnızca yeni
decodes-only-stations-would-new-entity = Yalnızca yeni olacak istasyonlar: varlık, bant-yuva, lokatör ya da çağrı
decodes-auto-tx-frq = Otomatik YAYIN FRK
decodes-overridden-hold-tx-lift-hold = Yayın frekansını tut tarafından geçersiz kılındı. Yayın frekansının hangi yöne gideceğini seçmek için tutmayı kaldırın.
decodes-pick-our-transmit-frequency-automatically = Yayın frekansımızı otomatik seçin: yayın yaptığımız periyotta en sessiz nokta, yanıt verdiğimiz istasyonun frekansı yerine — onlar diğer periyotta yayın yapar, bu yüzden onların frekansı, tuşladığımızda orada kimin olduğu hakkında bir şey söylemez. Kapalıyken frekansı TUTMAZ: çağrılan istasyonun frekansında yanıt verir. Tutmak için Yayın frekansını tut kullanın.
decodes-hold-tx = Yayın frekansını tut
decodes-pin-transmit-tone-where-nothing =
    Yayın tonunu olduğu yerde sabitleyin. Hiçbir şey onu hareket ettirmez: bir istasyona yanıt vermek, çağrı kuyruğu, CQ çağırmak, bir çözümlemeye ya da şelaleye tıklamak. Hareket ettirmek için kapatın, sonra yeniden açın.

    Bant değiştirmek tek istisnadır ve bu sizin kendi eyleminizdir: yeni bantta en son ayarladığınız kayma onunla geri gelir; böylece 60 m değeri 20 m'ye, 20 m'nin değeri 60 m'ye taşınmaz.

    Lisansınız bant planından dar olduğunda — 5357 kHz'deki bir İngiltere 60 m skalasında tahsis 5358.0'da biter, bu yüzden ton 1000 Hz'in altında kalmalıdır ve iki otomatik taşıyıcıdan biri bir yayından diğerine bandın dışına çıkar.
decodes-type-transmit-offset-hz-then = Yayın kaymasını Hz cinsinden yazın, sonra Enter'a basın. Lisansınız bant planından dar olduğunda, aşağıdaki kenarın altındaki tüm aralık sizin seçiminizdir, bu yüzden burada bir öneri yapılmaz.
decodes-clear-rx = {" "}ALMAYI TEMİZLE{" "}
decodes-empty-decode-list-nothing-air = Çözümleme listesini boşaltın. Havada olan hiçbir şey durmaz.
decodes-even = ÇİFT
decodes-text = METİN
decodes-dxcc = DXCC
decodes-band = BANT
decodes-grid = LOKATÖR
decodes-dupe = KOPYA
decodes-reply = YANIT
decodes-queued-click-remove = Sıraya alındı — kaldırmak için tıklayın
decodes-work-station-after-current-one = Bu istasyonla, şu anki istasyondan sonra çalışın
decodes-qsos-worked-since-sdroxide-was = sdroxide başlatıldığından beri çalışılan QSO'lar
decodes-setup = ⚙ KURULUM
decodes-qso-complete = ✓ QSO TAMAMLANDI
decodes-contact-complete-log-held-here = Temas tamamlandı ve günlükte. Diğer istasyon kendi iletisini tekrarlarsa son iletinin yeniden gönderilebilmesi için birkaç dakika burada tutulur — başka bir şey borçlu değilsiniz.
decodes-tx = ● TX
decodes-dxpedition-mode-transmit-frequency-held = DXpedition modu. Yayın frekansı, Fox yanıt verene dek geçiş bandının Fox'un yarısının dışında (1000 Hz'in altında) tutulur.
decodes-watchdog = BEKÇİ
decodes-transmitting-stopped-no-reply-no = Yayın durdu: bekçi süresince ne yanıt ne de eylem var. Devam etmek için CQ çağırın ya da bir ileti seçin.
decodes-well-inside-tolerance = Tolerans içinde rahat.
decodes-no-active-qso-pick-decode = etkin QSO yok — yanıt vermek için bir çözümleme seçin ya da CQ çağırın
decodes-ft8-engine-idle = FT8 motoru boşta
decodes-being-worked = çalışılıyor
decodes-clear = TEMİZLE
decodes-no-messages = — ileti yok —
decodes-call-cq = {"  "}CQ ÇAĞIR{"  "}
decodes-stop-qso = {" "}QSO'YU DURDUR{" "}
decodes-stop-tx = {" "}YAYINI DURDUR{" "}
decodes-free-text-13-chars = serbest metin (13 karakter)
decodes-send = GÖNDER

## sat (src/app/sat.rs)

sat-pass-profile = GEÇİŞ PROFİLİ
sat-satellite = UYDU
sat-satellites = UYDULAR
sat-passes = GEÇİŞLER
sat-set-your-grid-passes-settings = Geçişler için lokatörünüzü ayarlayın (Ayarlar ▸ Genel).
sat-next = SONRAKİ
sat-peak = TEPE ≥
sat-all-satellites = tüm uydular
sat-off-only-satellites-frequencies-file = Kapalı: yalnızca dosyasında frekansı olan uydular — çalışabileceğiniz şeyler — ve kendinizin yapıştırdıkları. Açık: listedeki her uydu; bir abonelikle binlerce olabilir ve aramak bir süre alır.
sat-no-passes-window-lower-peak = Bu pencerede geçiş yok. Tepeyi düşürün ya da pencereyi genişletin.
sat-length = SÜRE
sat-peak-2 = TEPE
sat-from-to = BAŞLANGIÇ → BİTİŞ
sat-up-now = ● şu an yukarıda
sat-unlock = KİLİDİ AÇ
sat-release-lock-dial-stays-where = Kilidi bırakın; skala olduğu yerde kalır
sat-azimuth = AZİMUT
sat-elevation = YÜKSEKLİK
sat-range = MESAFE
sat-rate = HIZ
sat-downlink = AŞAĞI BAĞLANTI
sat-doppler-rx = DOPPLER RX
sat-uplink = YUKARI BAĞLANTI
sat-doppler-tx = DOPPLER TX
sat-no-pass-inside-next-48 = QTH'nizden önümüzdeki 48 saat içinde geçiş yok.
sat-elements-stale-corrections-suspended-until = ⚠ Elemanlar eski — bir TLE yenilenene dek düzeltmeler askıda
sat-doppler-correction-needs-iq-front = Doppler düzeltmesi bir IQ ön ucu gerektirir — bir CAT telsizi yalnızca izler
sat-rotator-not-connected = Döndürücü: bağlı değil
sat-doppler = DOPPLER
sat-apply-computed-correction-rx-tx = Hesaplanan düzeltmeyi RX ve TX'e uygulayın
sat-rotator = DÖNDÜRÜCÜ
sat-steer-antenna-through-rotctld-client = Anteni rotctld istemcisi üzerinden yönlendirin (Ayarlar ▸ Sunucular)
sat-name-norad = ad ya da NORAD
sat-set-your-grid-passes-settings-2 = geçişler için lokatörünüzü ayarlayın (Ayarlar ▸ Genel)
sat-norad = NORAD
sat-pass = GEÇİŞ
sat-always-visible = her zaman görünür
sat-not-here = buradan değil
sat-no-elements = eleman yok
sat-pick-satellite-see-links-lock = Bağlantılarını görmek ve kilitlenmek için bir uydu seçin.
sat-no-frequencies-file-one-add = Bunun için dosyada frekans yok — bunları Ayarlar ▸ TLE içinde ekleyin.
sat-link = BAĞLANTI
sat-downlink-mhz = AŞAĞI BAĞLANTI (MHz)
sat-uplink-mhz = YUKARI BAĞLANTI (MHz)
sat-mode = MOD
sat-tune = AYARLA
sat-set-dial-mode-link-no = Skalayı ve modu bu bağlantıya ayarlayın — kilit yok, düzeltme yok
sat-locked = {" "}KİLİTLİ{" "}
sat-lock = {" "}KİLİTLE{" "}
sat-track-satellite-doppler-correction-rx = Bu uyduyu izleyin: RX ve TX'te Doppler düzeltmesi, transponder eşleşmeli yukarı bağlantı ve 3B görünüm onu izler

## wefax (src/app/panels/wefax.rs)

wefax-wefax = WEFAX
wefax-stop = {" "}■ DURDUR{" "}
wefax-end-chart-now-save-what = Grafiği şimdi bitirin ve gelen her şeyi kaydedin
wefax-start = {" "}● BAŞLAT{" "}
wefax-start-chart-now-without-waiting = Bir başlangıç tonu beklemeden şimdi bir grafik başlatın
wefax-phasing = fazlanıyor…
wefax-subcarrier-offset-1900-hz-tune = 1900 Hz'den alt taşıyıcı kayması. Yaklaşık sıfır için ayarlayın: faks taşıyıcısı 1500 Hz siyah ile 2300 Hz beyaz arasındadır ve birkaç yüz hertz kaymış bir alıcı resmi düz siyaha ya da düz beyaza kırpar.
wefax-auto-start = OTOMATİK BAŞLAT
wefax-begin-chart-when-300-hz = 300 Hz (IOC 576) ya da 675 Hz başlangıç tonu duyulduğunda bir grafik başlatın — ve devam eden grafiği bitirin ve bir sonraki yayın geldiğinde yenisini fazlayın; bir durdurma tonu önce gelmiş olsun ya da olmasın.
wefax-auto-stop = OTOMATİK DURDUR
wefax-end-450-hz-stop-tone = 450 Hz durdurma tonunda bitirin. Arka arkaya birkaç grafik gönderen bir istasyonda kaydetmeye devam etmek için kapatın.
wefax-phase = FAZ
wefax-slant = EĞİM
wefax-sample-clock-trim-if-chart = Örnekleme saati ayarı. Grafik sola yatıyorsa bunu artırın; sağa yatıyorsa azaltın. Yüz ppm kayık bir ses kartı, çeyrek saatlik bir grafiği satırın çoğu kadar yana kaydırır.
wefax-view = GÖRÜNÜM
wefax-scale-chart-panel-width = Grafiği panel genişliğine ölçekle
wefax-whole = TAM
wefax-shrink-chart-until-all-view = Grafiğin tamamı aynı anda görünene dek küçültün
wefax-half-size = Yarı boyut
wefax-one-screen-pixel-per-fax = Faks pikseli başına bir ekran pikseli — ayrıntı için kaydırın
wefax-twice-size-scroll-move-around = İki kat boyut; grafikte gezinmek için kaydırın
wefax-height = YÜKSEKLİK
wefax-stretch-picture-vertically-chart-came = Resmi dikey olarak gerin. Ezilmiş ya da gerilmiş çıkan bir grafik genellikle yanlış satır hızında çözülmüştür — bu onu okunur kılar, LPM çipleri ise bunu düzgünce düzeltir. Bir değer yazmak için çift tıklayın.
wefax-reset = SIFIRLA
wefax-back-picture-s-own-proportions = Resmin kendi oranlarına dönün
wefax-follow = İZLE
wefax-keep-newest-lines-view-scrolling = En yeni satırları görünürde tutun. Yukarı kaydırmak bunu kapatır, böylece gelmiş olanı okuyabilirsiniz; en alta geri kaydırmak yeniden açar.
wefax-tune-fax-schedule-usb-wait = USB'de bir faks programına ayarlanın ve bir başlangıç tonu bekleyin ya da grafiğin ortasından başlamak için BAŞLAT'a basın.
wefax-saved = KAYITLI
wefax-path = YOL
wefax-reading-radio-s-charts = Radyonun grafikleri okunuyor…
wefax-finished-charts-collect-here = Bitmiş grafikler burada toplanır.
wefax-delete-chart = Bu grafiği sil
wefax-loading = yükleniyor…
wefax-stations = İSTASYONLAR
wefax-broadcast-radiofax-schedules-picking-one = Yayın radyofaks programları — birini seçmek skalayı ayarlar
wefax-frequencies-are-published-carrier-dial = Frekanslar yayımlanmış taşıyıcılardır; skala bunun 1,9 kHz altına gider ve bu sizin için yapılır. Programlar değişir ve istasyonlar kapanır — bunu bir zaman çizelgesi değil, aramaya nereden başlayacağınız olarak görün.
wefax-delete-chart-store = Bu grafiği depodan sil
wefax-newer = ◀ DAHA YENİ
wefax-chart-received-after-one = Bundan sonra alınan grafik
wefax-older = DAHA ESKİ ▶
wefax-chart-received-before-one = Bundan önce alınan grafik
wefax-no-longer-store = artık depoda değil
wefax-loading-full-size = tam boyut yükleniyor…
wefax-save = KAYDET
wefax-save-copy-chart-computer = Bu grafiğin bir kopyasını bu bilgisayara kaydedin
wefax-sure = EMİN MİSİNİZ?
wefax-click-again-delete-chart-good = Bu grafiği kalıcı olarak silmek için yeniden tıklayın
wefax-delete = SİL
wefax-file-chart-was-saved = Bu grafiğin kaydedildiği dosya

## aprs (src/app/panels/aprs.rs)

aprs-starting-aprs-modem = APRS modemi başlatılıyor…
aprs-aprs = APRS
aprs-no-callsign = çağrı işareti yok
aprs-nothing-will-transmitted-no-beacon = Bu istasyonun bir çağrı işareti olana dek hiçbir şey yayınlanmaz — ne işaret, ne ileti, hatta bir onay bile. Ayarlar → Genel altında bir tane ayarlayın ya da KURULUM altında SSID'siyle birlikte APRS'ye özgü bir tane.
aprs-no-position = konum yok
aprs-beacon-needs-somewhere-report-fill = Bir işaretin bildireceği bir yer gerekir. Lokatörünüzü Ayarlar → Genel altında doldurun ya da KURULUM altında koordinat verin.
aprs-setup = KURULUM
aprs-callsign-symbol-digipeater-path-position = Çağrı işareti, sembol, digipeater yolu, konum ve işaret
aprs-how-often-beacon-your-position = Konumunuzu gözetimsiz olarak ne sıklıkla işaretleyin. `off` — varsayılan — hiçbir zaman işaret vermez: bir kipi seçmek bir istasyonu havaya koymamalıdır. Sabit bir istasyon için otuz dakika geleneksel değerdir, hareket halindekiler için daha sık; ve her işaret başkasının kanal zamanıdır.
aprs-beacon = {" "}İŞARET{" "}
aprs-station-has-no-callsign-see = Bu istasyonun çağrı işareti yok — soldaki uyarıya bakın.
aprs-station-has-no-position-report = Bu istasyonun bildireceği bir konum yok — soldaki uyarıya bakın.
aprs-send-one-position-report-now = Aralığı beklemeden şimdi bir konum raporu gönderin. Diğer her şey gibi kanalın boşalmasını bekler.
aprs-stations = İSTASYONLAR
aprs-nothing-heard-yet = henüz hiçbir şey duyulmadı
aprs-heard-direct = doğrudan duyuldu
aprs-two-characters-station-sent-say = İstasyonun ne olduğunu söylemek için gönderdiği iki karakter
aprs-killed = İPTAL EDİLDİ
aprs-station-put-object-here-has = Bu nesneyi buraya koyan istasyon onu iptal etti
aprs-sender-deliberately-reported-square-rather = Gönderen bir noktayı değil bir kareyi bilerek bildirdi. Harita kareyi çizer.
aprs-digipeaters-last-frame-came-through = Son çerçevenin geçtiği digipeater'lar. Gerçekten tekrarlayan birini `*` işaretler.
aprs-message = {" "}İLETİ{" "}
aprs-address-message-box-station = İleti kutusunu bu istasyona adresleyin
aprs-center = {" "}ORTALA{" "}
aprs-put-station-middle-map-double = Bu istasyonu haritanın ortasına koyun. Görünümü otomatik sığdırmaya geri vermek için haritaya çift tıklayın.
aprs-channel = KANAL
aprs-messages = İLETİLER
aprs-frames-arrived-failed-their-check = Gelen ve denetim dizisini geçemeyen çerçeveler — bir çarpışma, bir sönüm ya da okunamayacak kadar zayıf bir sinyal. Hiçbir şey çözülmezken artan bir sayı, modemin kanalı *duyduğu* anlamına gelir: seviye ölçerini denetleyin ve radyonun susturmasını açın.
aprs-frames-read-cleanly-off-channel = Kanaldan temiz okunan ve APRS olmayan çerçeveler — başkasının paket oturumu ya da bu sürümün çözmediği bir biçim.
aprs-every-frame-channel-arrived-other = Kanaldaki her çerçeve, geldiği gibi — başkalarının trafiği ve çözücünün okuyamadığı her şey dahil.
aprs-nothing-channel-yet = kanalda henüz hiçbir şey yok
aprs-no-messages = İleti yok.
aprs-waiting-channel = kanal bekleniyor
aprs-sent-retried-still-waiting-acknowledged = gönderildi ve yeniden denendi, hâlâ onay bekleniyor
aprs-air-waiting-acknowledged = havada, onay bekleniyor
aprs-acknowledged-far-end = uzak uç tarafından onaylandı
aprs-refused-far-end-not-retried = uzak uç tarafından reddedildi — yeniden denenmedi
aprs-no-ack = onay yok
aprs-no-answer-after-every-retry = her yeniden denemeden sonra yanıt yok
aprs-send = {" "}GÖNDER{" "}
aprs-station-has-no-callsign-so = Bu istasyonun çağrı işareti yok, bu yüzden hiçbir şey yayınlanamaz. Ayarlar → Genel altında bir tane ayarlayın ya da KURULUM altında SSID'siyle birlikte APRS'ye özgü bir tane.
aprs-needs-station-address-something-say = Adreslenecek bir istasyon ve söylenecek bir şey gerekir.
aprs-send-keep-retrying-until-far = Gönderin ve uzak uç onaylayana dek yeniden denemeye devam edin. İletiler, bu kanalda yanıtlanan tek şeydir.
aprs-busy = MEŞGUL
aprs-stations-heard-still-inside-window = APRS Kurulumu'nda ayarlanan pencere içinde hâlâ duyulan istasyonlar ve şu anda kanalda başka bir istasyon olup olmadığı. Meşgulken hiçbir şey anahtarlanmaz.
aprs-tx = ● TX
aprs-over-progress-frames-waiting-channel = Devam eden yayın, kanalın boşalmasını bekleyen çerçeveler ya da bir sonraki planlanmış işarete kadar geçen süre.
aprs-audio-reaching-modem-flat-means = Modeme ulaşan ses. Düz, hiçbir şeyin gelmediği anlamına gelir — radyonun veri çıkışının sdroxide'ın dinlediği çıkış olduğunu denetleyin. Modem bir taşıyıcı duyduğunda yeşil yanar.

## vdl2 (src/app/panels/vdl2.rs)

vdl2-starting-vdl2-decoder = VDL2 çözücüsü başlatılıyor…
vdl2-aircraft-datalink = uçak veri bağlantısı
vdl2-tune-middle-vdl2-group-decoder = VDL2 grubunun ortasına ayarlanın. Çözücünün kendi penceresi oradan kayarak alıcının ulaşabildiği on dört kanalın mümkün olduğunca çoğunu içine alır.
vdl2-setup = KURULUM
vdl2-which-channels-listen-how-hard = Hangi kanallarda dinleneceği, bir patlamanın ne kadar güçlü olması gerektiği ve ne kadar günlük tutulacağı
vdl2-save-vdl2-message-log-file = VDL2 ileti günlüğünü bir dosyaya kaydedin
vdl2-vdl2-channel = VDL2 kanalı
vdl2-messages = İLETİLER
vdl2-address-flight-registration-label = adres, uçuş, tescil ya da etiket
vdl2-held = TUTULDU
vdl2-hold = TUT
vdl2-hold-message-log-where-so = İleti günlüğünü olduğu yerde tutun; böylece bir sonraki yayın onu ekranda yukarı itmeden biri okunabilsin. Çözücü çalışmaya ve sayaçlar hareket etmeye devam eder; bırakmak, bu arada gelmiş her şeyi gösterir.
vdl2-nothing-decoded-yet = henüz hiçbir şey çözülmedi
vdl2-nothing-matches-filter = filtreyle eşleşen hiçbir şey yok
vdl2-stations = İSTASYONLAR
vdl2-nobody-heard-yet = henüz kimse duyulmadı
vdl2-vdl2-setup = VDL2 Kurulumu
vdl2-channels = Kanallar
vdl2-every-25-khz-slot-136 = 136,650 ile 136,975 MHz arasındaki her 25 kHz yuvası. Her biri için bir aşağı dönüştürücü, hepsi aynı alıcı penceresinin içinde. Birini kapatmak biraz işlemci zamanı kazandırır; ama diğerlerini daha hassas yapmaz.
vdl2-burst-threshold = Patlama eşiği
vdl2-above-each-channel-s-own = her kanalın kendi gürültü tabanının üstünde
vdl2-lower-catches-weaker-transmissions-costs = Daha düşük olması daha zayıf yayınları yakalar ama gürültüde işlemci zamanına mal olur; daha yüksek olması bunları kaçırır. Taban kanal başına öğrenilir ve buradaki bir değişiklik onu atmaz.
vdl2-keep-log = Günlükte tut
vdl2-messages-2 = {" "}ileti
vdl2-track-most = En fazla izle
vdl2-stations-2 = {" "}istasyon
vdl2-longest-silent-go-first = en uzun süre sessiz kalanlar önce gider
vdl2-forget-station-after = Bir istasyonu şundan sonra unut
vdl2-nothing-heard = ondan hiçbir şey duyulmadan
vdl2-show-unread-payloads = Okunmamış yükleri göster
vdl2-hex = onaltılık olarak
vdl2-frames-carrying-x-25-clnp = X.25, CLNP ya da bunların üstündeki veri bağlantısı uygulamalarını taşıyan çerçeveler. SDRoxide bunları adlandırır ve baytları okumak yerine gösterir; bunları gizlemek, trafiğin ne kadarının bu olduğunu gizlemek olur.
vdl2-registration = Tescil
vdl2-flight = Uçuş
vdl2-label = Etiket
vdl2-block = Blok
vdl2-sequence = Sıra
vdl2-mode = Mod
vdl2-more-blocks-follow = daha fazla blok geliyor
vdl2-check = Denetim
vdl2-message-crc-good = ileti CRC'si doğru
vdl2-message-crc-not-checked = ileti CRC'si denetlenmedi
vdl2-parity = Eşlik
vdl2-link-control-no-payload = bağlantı denetimi, yük yok
vdl2-name = AD
vdl2-addr = ADR
vdl2-type = TÜR
vdl2-msgs = İLT

## cw (src/app/panels/cw.rs)

cw-tone-being-copied-tone-transmitted = Kopyalanan ton ve yayınlanan ton — CW'de aynı frekanstır. Üzerine taşımak için şelaleye tıklayın.
cw-down-10-hz = 10 Hz aşağı
cw-up-10-hz = 10 Hz yukarı
cw-qrg-main-readout-tuning-line = QRG: ana gösterge ve ayar çizgisi çalışılan frekanstadır. Bunun yerine skalayı görmek için tıklayın, altında bir yan ton perdesiyle — çoğu radyonun gösterdiği.
cw-put-main-readout-tuning-line =
    Ana gösterge ve ayar çizgisini skalada değil sinyalde tutun; böylece gösterilen frekans, iki operatörün de vereceği frekans olur ve ayar çizgisi geçiş bandının ortasında durur. Ayar değişmez; yalnızca sayılar hareket eder.

    Bir sinyale tıklamak onu, tıklama adımının izin verdiği kadar yakın bir yere koyar — Denetimler → tıkla-ayarla yuvarlaması, varsayılan 10 Hz. Kaba bir adım, sinyali perdeden yarısına kadar uzak bırakır ve gösterge bunu söyler.
cw-sending-speed-read-off-signal = Gönderme hızı sinyalden okunur
cw-signal-noise-500-hz-same = 500 Hz'de sinyal-gürültü — bir raporun verdiği aynı rakam
cw-how-far-off-cursor-signal = Sinyalin imleçten gerçekte ne kadar uzakta olduğu. Çözücü onu izler; geçiş bandı izlemez, bu yüzden büyürse imleci hafifçe itin.
cw-listening = — dinleniyor —
cw-tx = ● TX
cw-trainer = EĞİTMEN
cw-learn-morse-read-characters-words = Morse öğrenin: karakterleri ve kelimeleri okuyun, bunları kendi hızınızda oynatın ve her seferinde bir karakter ekleyen bir Koch alıştırması. Burada, bu bilgisayarın hoparlörlerinden çalar — hiçbir şey radyoyu anahtarlamaz.
cw-nothing-copied-yet = — henüz hiçbir şey kopyalanmadı —
cw-key-send-characters-you-send = Göndermek için tuşlayın — gönderdiğiniz karakterler burada görünür.
cw-type-line-return-sends = Bir satır yazın, Return gönderir…
cw-type-here-send = Göndermek için buraya yazın…
cw-key = {" "}TUŞ ●{" "}
cw-key-2 = {" "}TUŞ{" "}
cw-hold-key-bound-cw-straight = CW düz tuşa atanmış tuşu — varsayılan olarak Space, Ayarlar → Denetimler'de istediğiniz herhangi bir tuş — düz bir tuş olarak basılı tutun: basılı tutulduğunda aşağı, bırakıldığında yukarı, metin yazmak yerine. Açıkken yayın kutusu kilitlenir ve tüm keyer tuşa verilir: sıraya alınmış her metin düşer.
cw-radio-sends-own-keyer-text =
    Bu radyo kendi keyer'ından gönderir: metin kontrol portundan gider ve telsiz elemanların zamanlamasını yapar; bu yüzden düz tuşla havaya çıkış arasında bir elin sürebileceği hiçbir şey yoktur.

    Elle tuşlamak için Ayarlar → Radyo'da CW anahtarlamasını “Sound card (MCW)” olarak ayarlayın. Telsiz o zaman bir yan banda alınır ve keyer'ın kendi yan tonu ses olarak yayınlanır; düz tuşun sürdüğü yol budur.
cw-watchdog = BEKÇİ
cw-straight-key-was-held-down = Düz tuş çok uzun basılı tutuldu — bir el değil, kaybolmuş bir tuş bırakma — bu yüzden taşıyıcı düşürüldü ve yayın kapatıldı. Devam etmek için tuşa yeniden basın.
cw-tx-2 = {"  "}YAYIN AÇIK{"  "}
cw-tx-3 = {"   "}YAYIN{"   "}
cw-send-what-box-now-without = Kutudaki her şeyi, Return'ü beklemeden şimdi gönderin
cw-hold-key-down-between-characters = Karakterler arasında tuşu basılı tutun; böylece yazılan hiçbir şey beklemez
cw-call-cq = {" "}CQ ÇAĞIR{" "}
cw-nocall = NOCALL
cw-clear = {" "}TEMİZLE{" "}
cw-stop-sending-drop-whatever-has = Göndermeyi durdurun ve çıkmamış olanı atın. Düz tuş açıkken, bastığınız şeyin geri okunması da onunla gider.
cw-sidetone = YAN TON
cw-play-keyed-tone-through-computer = Gönderirken anahtarlanan tonu bu bilgisayarın hoparlörlerinden de çalın; böylece gönderdiğinizi duyarsınız. `Sound card (MCW)` üzerinde ton yalnızca telsize gider ve başka hiçbir yere; bu olmadan sessizlikte gönderirsiniz. Telsizin kendi monitörü ya da yayın dışı bir kopya işi zaten yaptığında kapalı.
cw-hold-what-typed-until-return = Yazılanı Return'e dek tutun, sonra satırı her karakteri yazıldıkça tek tek tuşlamak yerine tek parça olarak gönderin. Metinden kendiliğinden tuşlayan bir telsizde değerlidir; çünkü keyer'ına her aktarma, bir yayın-alım döngüsüdür.
cw-your-own-message-buttons-contest = Kendi ileti düğmeleriniz — bir yarışma alışverişi, ad ve QTH yanıtı, TNX 73 GL. Her biri tüm metnini bir seferde gönderir ve F2–F10 ilk dokuzunu basar. İstasyonun yapılandırmasıyla birlikte taşınır, böylece uzak bir istemci de onlara sahip olur.
cw-cw-messages = CW İLETİLERİ
cw-clear-rx = {" "}ALMAYI TEMİZLE{" "}
cw-empty-receive-window-straight-key = Alma penceresini ve düz tuşun geri okumasını boşaltın. Havada olan hiçbir şey durmaz.
cw-lock = KİLİT
cw-decode-speed-set-here-instead = Sinyalden okumak yerine, burada ayarlanan hızda çözün. Hız aramasının oturamayacak kadar zayıf bir sinyale yardımcı olur.
cw-farnsworth-send-characters-full-speed = Farnsworth: karakterleri tam hızda gönderin ve yalnızca aralarındaki boşlukları uzatın; böylece doğru ritimde duyulurlar ama not almak için yeterince yavaş gelirler.
cw-off-normal-spacing = Kapalı — normal aralık
cw-keying-speed = Tuşlama hızı
cw-idle-0 = BOŞTA 0
cw-how-long-transmit-held-after = Son karakterden sonra ya da düz tuş bırakıldıktan sonra, taşıyıcı düşmeden önce yayının ne kadar tutulduğu. Daha uzun, yavaş yazan birinin duraklamalarını köprüler; daha kısa, frekanstan daha çabuk çıkar; Kapalı, sıradaki her şey gittiği anda düşürür.
cw-off-drop-once = Kapalı — hemen düşür

## textmodem (src/app/panels/text_modem.rs)

textmodem-tune-down-10-hz = 10 Hz aşağı ayarla
textmodem-tune-up-10-hz = 10 Hz yukarı ayarla
textmodem-tx = ● TX
textmodem-listening = — dinleniyor —
textmodem-type-line-return-sends = Bir satır yazın, Return gönderir…
textmodem-type-here-transmit = Yayınlamak için buraya yazın…
textmodem-tx-2 = {"  "}YAYIN AÇIK{"  "}
textmodem-tx-3 = {"   "}YAYIN{"   "}
textmodem-call-cq = {" "}CQ ÇAĞIR{" "}
textmodem-nocall = NOCALL
textmodem-clear = {" "}TEMİZLE{" "}
textmodem-hold-what-typed-until-return = Yazılanı Return'e dek tutun, sonra satırı her karakteri yazıldıkça havaya dökmek yerine tek parça olarak gönderin. Bir satırın, herhangi bir kısmı çıkmadan önce geri okunup düzeltilmesini sağlar.
textmodem-your-own-message-buttons-working = Kendi ileti düğmeleriniz — çalışma koşulları, hava durumu, standart bir yanıt. Her biri tüm metnini bir seferde gönderir ve F2–F10 ilk dokuzunu basar. İstasyonun yapılandırmasıyla birlikte taşınır, böylece uzak bir istemci de onlara sahip olur.
textmodem-messages = İLETİLER
textmodem-hell = HELL
textmodem-contrast = Kontrast
textmodem-harder-softer-dots-redraws-whole = Daha sert ya da daha yumuşak noktalar — tüm şeridi yeniden çizer
textmodem-width = Genişlik
textmodem-t-2row = {" "}2SATIR{" "}
textmodem-draw-every-column-twice-stacked = Her sütunu iki kez, üst üste çizin. Hell'in dikey senkronu yoktur, bu yüzden fazı ne olursa olsun metnin tam bir kopyasını okunur tutar.
textmodem-rev = {" "}TERS{" "}
textmodem-reverse-video-light-dots-dark = Ters video — koyu kâğıt üzerinde açık noktalar
textmodem-clear-rx = {" "}ALMAYI TEMİZLE{" "}
textmodem-hold-channel-idle-sends-blank = Kanalı tutun: boşta boş kâğıt gönderir, böylece şerit kaydırmaya devam eder
textmodem-hold-what-typed-until-return-2 = Yazılanı Return'e dek tutun, sonra satırı her karakteri yazıldıkça şeride boyamak yerine tek parça olarak gönderin. Canlı yazılmış bir düzeltme zaten kâğıttadır; Return'den önce yapılan hiçbir zaman olmamıştır.
textmodem-shift = Kayma
textmodem-shift-5-hz = Kayma −5 Hz
textmodem-shift-5-hz-2 = Kayma +5 Hz
textmodem-baud = Baud
textmodem-reverse-swap-mark-space-needed = Ters: işaret ve boşluğu değiştirin. Sinyal, gönderildiği yan bantın karşısında alındığında gerekir; bu, tonları ters çevirir.
textmodem-track-tuning-error-automatically-instead = İmleçte sabit kalmak yerine ayar hatasını otomatik izleyin.
textmodem-tones = Tonlar
textmodem-mode = Mod
textmodem-off-keyed = açık/kapalı tuşlanmış

## ais (src/app/panels/ais.rs)

ais-starting-ais-decoder = AIS çözücüsü başlatılıyor…
ais-tune-between-two-ais-channels = İki AIS kanalının arasına ayarlanın — 161,975 (AIS 1, deniz 87B) ve 162,025 (AIS 2, 88B). Skala ortaya konur, çünkü orada hiçbir şey yayın yapmaz; bu yüzden sıfır-IF bir alıcının DC tepesi ikisine de düşmez.
ais-setup = KURULUM
ais-channels-timeouts-trail-length-how = Kanallar, zaman aşımları, iz uzunluğu ve vektörlerin ne kadar ileriye uzandığı
ais-vessels = GEMİLER
ais-nothing-heard-yet = henüz hiçbir şey duyulmadı
ais-virtual-nothing-there = sanal — orada hiçbir şey yok
ais-position-stale = konum eski
ais-search-rescue-transmitter-man-overboard = bir arama ve kurtarma vericisi, denize düşen adam işareti ya da bir AIS EPIRB
ais-call-sign = çağrı işareti
ais-station-time = istasyon zamanı
ais-dgnss = {" "}(DGNSS)
ais-first-heard = ilk duyuldu
ais-center = ORTALA
ais-put-vessel-middle-chart = Bu gemiyi haritanın ortasına koyun
ais-last-message-station-nmea-sentences = Bu istasyondan gelen son ileti, bir alıcının kabloya koyacağı NMEA cümleleri olarak. sdroxide'ın ondan ne yaptığını denetlemek için herhangi başka bir AIS çözücüsüne yapıştırın.
ais-ais-setup = AIS Kurulumu
ais-channels = Kanallar
ais-drop-chart-after = Haritadan şundan sonra düşür
ais-without-position-report = konum raporu olmadan
ais-five-minutes-default-not-ads = Varsayılan olarak beş dakika, ADS-B'nin on saniyesi değil: demirde bir gemi üç dakikada bir rapor verir ve daha kısa bir pencere, iki mükemmel rapor arasında bir limanın çoğunu boşaltırdı. Bu süreden sonra gemi haritadan çıkar ve satırı griye döner — soluk değildir, çünkü eski bir konumdaki soluk bir sembol hâlâ bir geminin nerede olduğu iddiasıdır, gerçek olanlarla aynı mürekkeple.
ais-drop-list-after = Listeden şundan sonra düşür
ais-nothing-heard-all = hiçbir şey duyulmadan
ais-message-carrying-ship-s-name = Bir geminin adını taşıyan ileti her altı dakikada bir gelir; bu yüzden kısa bir liste penceresi, adlarını söylemeden hemen önce gemileri atmaya devam eder.
ais-trail-length = İz uzunluğu
ais-history-behind-each-target = Her hedefin arkasındaki geçmiş
ais-minutes-rather-than-points-because = Nokta cinsinden değil dakika cinsinden, çünkü AIS raporlama hızları iki kat büyüklük arasında yayılır: sabit bir sayı, bir feribotta seksen saniye ve demirli bir tankerde iki saat olurdu, ikisi de aynı çizilirdi. Sıfır izleri kapatır.
ais-speed-vector = Hız vektörü
ais-how-far-ahead-vector-reaches = vektörün ne kadar ileriye uzandığı
ais-slot-threshold = Yuva eşiği
ais-above-channel-s-learned-noise = kanalın öğrenilmiş gürültü tabanının üstünde
ais-track-most = En fazla izle
ais-vessels-2 = {" "}gemi
ais-ais-receive-only-here-always = AIS burada yalnızca alımdır ve her zaman öyle kalacaktır: bu bir can güvenliği hizmetidir ve üzerine sahte gemi trafiği koymak, bir lisansın kapsadığı bir şey değildir.
ais-fill-my-grid-digimode-setup = Mesafeler ve yönler için dijital mod kurulumunda Lokatörüm'ü doldurun; böylece harita çerçevesini, en uzakta olan şeye göre değil, bulunduğunuz yere göre kurar.
ais-name = AD
ais-mmsi = MMSI
ais-type = TÜR

## qo100 (src/app/qo100.rs)

qo100-receiver-cannot-reach-10489-750 = Bu alıcı, tek başına 10489,750 MHz'e ulaşamaz — önce bir LNB/dönüştürücü kayması ayarlayın (Ayarlar ▸ Radyo)
qo100-run-fast-spectral-tracker-every = Hızlı spektral izleyiciyi çalıştırın: her saniye, bekleme penceresinde işaretin iki simetrik lobunu arar ve taşıyıcının nerede durduğunu bildirir
qo100-telemetry = TELEMETRİ
qo100-also-run-ao-40-uncoded = AO-40 kodlanmamış çerçeve çözücüyü de çalıştırın — senkron sözcüğü, CRC ve telemetri metni — her geçişin ne kadar ilerlediğini adım adım gösteren bir okumayla (taşıyıcı → senkron → CRC)
qo100-auto = OTO
qo100-auto-correct-let-tracker-adjust = Otomatik düzeltme: izleyicinin dönüştürücü/LNB kaymasını kendi başına ayarlamasına izin verin — temiz ve kararlı bir tahminde işareti 10489,750 MHz'e geri iten ve LNB kaydıkça orada tutan yavaş bir kapalı döngü. Her etkinleştiğinde ön ucu yeniden açar.
qo100-narrower-search-smaller-slice = Daha dar — daha küçük bir dilimi arayın
qo100-wider-when-beacon-isn-t = Daha geniş — işaret mevcut genişlikte bulunamadığında
qo100-clear-mark = işareti temizle
qo100-drop-hand-placed-beacon-mark = Elle yerleştirilmiş işaret işaretini bırakın ve çözücünün kendi okumasına dönün
qo100-lo = alt −
qo100-lo-2 = alt +
qo100-hi = üst −
qo100-hi-2 = üst +
qo100-radio-s-own-tuning-range = Bu radyonun kendi ayar aralığı 10489,750 MHz'e ulaşmıyor — bu pencerenin bir şey yapabilmesi için önce bir LNB/dönüştürücü kaymasına (Ayarlar ▸ Radyo ▸ Dönüştürücü) ihtiyacı var.
qo100-open-settings-radio = Ayarlar ▸ Radyo'yu açın
qo100-tune-10489-750-mhz = 10489,750 MHz'e ayarlayın
qo100-tip-if-you-can-see = ipucu: işareti şeritte görebiliyor ama kilitlenmiyorsa, üzerine çift tıklayın — bu onu 10489,750 MHz olarak işaretler ve UYGULA'nın ona düzeltmesine izin verir
qo100-tracker = İZLEYİCİ
qo100-no-twin-lobe-shape-parking = bekleme penceresinde çift lob şekli yok
qo100-shape = ŞEKİL
qo100-drift = KAYMA
qo100-need-longer-run-estimates = — (daha uzun bir tahmin dizisi gerekiyor)
qo100-cycles = DÖNGÜLER
qo100-receiver = ALICI
qo100-target = HEDEF
qo100-measured = ÖLÇÜLEN
qo100-clicked = {"  "}(tıklanan)
qo100-tracker-2 = {"  "}(izleyici)
qo100-not-locked-yet = henüz kilitlenmedi
qo100-converter-offset = DÖNÜŞTÜRÜCÜ KAYMASI
qo100-decode = ÇÖZ
qo100-decode-filling-first-frame-buffer = ÇÖZ — ilk çerçeve arabelleği dolduruluyor…
qo100-decode-off-turn-telemetry-chip = ÇÖZ — kapalı. Taşıyıcı → senkron → CRC okuması için TELEMETRİ çipini (üst satır) açın.
qo100-apply-correction = {" "}DÜZELTMEYİ UYGULA{" "}
qo100-write-converter-lnb-offset-puts = Tıkladığınız yerdeki işareti 10489,750 MHz'e koyan dönüştürücü/LNB kaymasını yazın ve alıcıyı yeniden açın — Ayarlar ▸ Radyo ▸ Uygula'nın yaptığı aynı kısa kesinti
qo100-write-converter-lnb-offset-puts-2 = İzleyicinin tahminini 10489,750 MHz'e koyan dönüştürücü/LNB kaymasını yazın ve alıcıyı yeniden açın — Ayarlar ▸ Radyo ▸ Uygula'nın yaptığı aynı kısa kesinti
qo100-waiting-second-crc-valid-frame = Bunu yazmayı önermeden önce CRC'si geçerli ikinci bir çerçeve bekleniyor — tek bir kilit bir tesadüfi eşleşme olabilir
qo100-write-corrected-converter-lnb-offset = Düzeltilmiş dönüştürücü/LNB kaymasını yazın ve alıcıyı yeniden açın — kısa bir kesinti, Ayarlar ▸ Radyo ▸ Uygula'nın yaptığıyla aynı
qo100-decoder-runs-receiving-station-readout = çözücü, alan istasyonda çalışır — okuması uzak istemcilere gönderilmez
qo100-starting = başlatılıyor…
qo100-searching-first-window-fills-after = aranıyor — ilk pencere yaklaşık 24 sn sonra dolar, sonra tekrarlanır

## js8 (src/app/panels/js8.rs)

js8-multi = ÇOKLU
js8-decode-every-js8-speed-not = Yalnızca yayın yaptığınız hızı değil, her JS8 hızını çözün. Dört hız alt bandı paylaşır ve dört ayrı dalga biçimidir; bu olmadan başka bir hızdaki bir istasyon görünmez — ve iki hız arasında bir alışveriş hiç olamaz. Alma işlemcisi zamanını yaklaşık dört katına çıkarır.
js8-hb-auto = HB OTO
js8-turbo-does-not-beacon-local = Turbo işaret vermez — yerel ve VHF hızıdır
js8-beacon-your-callsign-grid-every = Çağrı işaretinizi ve lokatörünüzü her 15 dakikada bir işaretleyin
js8-until-next-heartbeat = Bir sonraki kalp atışına kadar
js8-setup = ⚙ KURULUM
js8-tx = ● TX
js8-heard = DUYULAN
js8-nothing-heard-yet = — henüz hiçbir şey duyulmadı —
js8-dxcc = DXCC
js8-band = BANT
js8-grid = LOKATÖR
js8-dupe = KOPYA
js8-reply = YANIT
js8-no-messages = — ileti yok —
js8-cq = {" "}CQ{" "}
js8-hb = {" "}HB{" "}
js8-clear = {" "}HEDEFİ TEMİZLE{" "}
js8-forget-selected-station-composer-goes = Seçili istasyonu unutun — yazı alanı @ALLCALL'a döner
js8-already-addressing-allcall = Zaten @ALLCALL'a yazılıyor
js8-stop = {" "}DURDUR{" "}
js8-send = {" "}GÖNDER{" "}
js8-message = İleti…

## Günlük (src/app/logbook.rs)

logbook-logbook = GÜNLÜK
logbook-new-entry = + YENİ KAYIT
logbook-adif = ADIF
logbook-import = İÇE AKTAR
logbook-import-qsos-adif-adi-adif = Bir ADIF (.adi ya da .adif) dosyasından QSO'ları içe aktarın
logbook-new-qso = YENİ QSO
logbook-edit-qso = QSO'YU DÜZENLE
logbook-call = Çağrı
logbook-lookup = SORGULA
logbook-look-up-name-qth-grid = Ad / QTH / lokatörü sorgulayın
logbook-grid = Lokatör
logbook-freq-mhz = Frek MHz
logbook-mode = Mod
logbook-rst-sent = Gönderilen RST
logbook-rst-rcvd = Alınan RST
logbook-name = Ad
logbook-state = Eyalet
logbook-country = Ülke
logbook-date-utc = Tarih UTC
logbook-time = Saat
logbook-pwr-w = Güç W
logbook-contest = Yarışma
logbook-s-sent = Gönderilen S#
logbook-s-rcvd = Alınan S#
logbook-comment = Yorum
logbook-save = {" "}KAYDET{" "}
logbook-cancel = İPTAL
logbook-lotw = LoTW ✓
logbook-eqsl = eQSL ✓
logbook-card = kart ✓
logbook-lotw-2 = LoTW ↑
logbook-eqsl-2 = eQSL ↑
logbook-qrz = QRZ ↑
logbook-hamqth = HamQTH ↑
logbook-club-log = Club Log ↑
logbook-wavelog = Wavelog ↑
logbook-delete-entry = Bu kaydı sil
logbook-edit = DÜZENLE
logbook-upload-qso-configured-logs = Bu QSO'yu yapılandırılmış günlüklere yükleyin
logbook-no-qsos-yet-run-ft8 = henüz QSO yok — FT8/FT4 çalıştırın ya da elle bir kayıt ekleyin

## Tarayıcı (src/app/scanner.rs)

scanner-scanner = Tarayıcı
scanner-work-through-stored-memory-channels = Kayıtlı hafıza kanallarını sırayla çalışın
scanner-work-through-slice-band-channel = Bir bandın bir dilimini kanal ızgarasında sırayla çalışın
scanner-stop = DURDUR
scanner-start = BAŞLAT
scanner-from = Başlangıç
scanner-step = Adım
scanner-mode = Mod
scanner-range-empty-high-edge-has = Bu aralık boş — üst kenar, alt kenarın en az bir kanal üstünde olmalıdır.
scanner-skip-while-holding-pass-over = Bu andan itibaren o kanalın üstünden geçmek için tutarken ATLA
scanner-skipping = Atlanıyor
scanner-clear = TEMİZLE
scanner-stop-passing-over-any-them = Bunların hiçbirinin üstünden geçmeyi bırakın
scanner-stop-passing-over-channel = Bu kanalın üstünden geçmeyi bırakın
scanner-no-memory-channels-stored-yet = Henüz kayıtlı hafıza kanalı yok — MEM penceresinde birkaç tane kaydedin.
scanner-skip-channel-pass-over = Üstünden geçmek için bir kanalı ATLAYIN
scanner-skip = ATLA
scanner-look-all-channels-fall-inside = Panadaptörün yapıldığı aynı dönüşümde, bir alıcı penceresi içine düşen tüm kanalları arayın ve yalnızca üzerinde bir şey olanlara ayarlanın. Böylece bir bandaki bir liste, ne kadar uzun olursa olsun tur başına bir ayar maliyeti taşır; kanal başına bir yerleşme süresi yerine. Tarama yine de durmadan önce her adayı dinler, bu yüzden neyin durdurduğu değişmez — ancak tarama alıcının filtresi üzerinden değil FFT üzerinden ölçtüğü için, hiçbir şeyde durmaya başlarsa eşiği denetleyin.
scanner-radio-hands-over-demodulated-audio = Bu radyo demodüle edilmiş ses verir ve arayacağı kendi spektrumu yoktur; bu yüzden hafıza taraması her iki durumda da her kanalı ziyaret eder.
scanner-fast = HIZLI
scanner-read-list-off-spectrum-instead = Her kanalı ziyaret etmek yerine listeyi spektrumdan okuyun
scanner-folders = Klasörler
scanner-scan-every-folder-every-folder = Her klasörü ve bundan sonra yapılan her klasörü tarayın. Yalnızca bunları taramak için bunun yerine klasörleri seçin.
scanner-unfiled = Klasörsüz
scanner-nothing-filed-under-folders-you = Seçtiğiniz klasörlerin altına hiçbir şey kaydedilmemiş.
scanner-stops = Durduğu yer
scanner-use-receiver-s-own-squelch = Alıcının kendi susturmasını kullanın; böylece tarama, ses tam olarak açılacağı yerde durur
scanner-squelch-off-scan-will-stop = susturma kapalı — tarama baktığı ilk şeyde durur
scanner-channel-power-signal-has-reach = Taramayı durdurmak için bir sinyalin ulaşması gereken kanal gücü
scanner-listens = Dinler
scanner-how-long-stay-candidate-before = Bir adayı yargılamadan önce onda ne kadar kalınacağı. Yaklaşık onda bir saniyenin altında seviye ölçer yerleşmez ve zayıf sinyaller kaçar
scanner-resumes = Devam eder
scanner-carry-once-signal-drops = Sinyal düştükten sonra devam edin
scanner-carry-after-fixed-time-regardless = Sabit bir süre sonra, her durumda devam edin
scanner-stay-until-you-press-next = SONRAKİ'ye basana dek kalın
scanner-how-long-stay = Ne kadar kalınacağı
scanner-how-long-wait-after-signal = Sinyal düştükten sonra ne kadar beklenecek
scanner-move-don-t-stop-channel = Devam edin ve bu kanalda bir daha durmayın
scanner-next = SONRAKİ
scanner-move-now = Şimdi devam et

## adsb (src/app/panels/adsb.rs)

adsb-starting-ads-b-decoder = ADS-B çözücüsü başlatılıyor…
adsb-ads-b = ADS-B
adsb-t-1090-mhz-mode-s = 1090 MHz Mode S
adsb-tune-receiver-1090-000-mhz = Alıcıyı dünya çapındaki ADS-B aşağı bağlantısı olan 1090,000 MHz'e ayarlayın. Yalnızca bir kanal vardır.
adsb-setup = KURULUM
adsb-timeouts-trail-length-how-far = Zaman aşımları, iz uzunluğu ve hız vektörlerinin ne kadar ileriye uzandığı
adsb-aircraft = UÇAKLAR
adsb-nothing-heard-yet = henüz hiçbir şey duyulmadı
adsb-ground = yerde
adsb-position-stale = konum eski
adsb-gnss-altitude = GNSS irtifası
adsb-ground-speed = yer hızı
adsb-vertical-rate = dikey hız
adsb-first-heard = ilk duyuldu
adsb-center = ORTALA
adsb-put-aircraft-middle-map = Bu uçağı haritanın ortasına koyun
adsb-last-frame-accepted-aircraft = Bu uçaktan kabul edilen son çerçeve
adsb-ads-b-setup = ADS-B Kurulumu
adsb-drop-map-after = Haritadan şundan sonra düşür
adsb-without-position-report = konum raporu olmadan
adsb-past-aircraft-comes-off-map = Bundan sonra uçak haritadan çıkar ve satırı griye döner. Soluk değildir: eski bir konumdaki soluk bir kare, hâlâ bir uçağın nerede olduğu iddiasıdır, gerçek olanlarla aynı mürekkeple.
adsb-drop-list-after = Listeden şundan sonra düşür
adsb-nothing-heard-all = hiçbir şey duyulmadan
adsb-trail-length = İz uzunluğu
adsb-points = {" "}nokta
adsb-history-dots-behind-each-target = Her hedefin arkasındaki geçmiş noktaları
adsb-speed-vector = Hız vektörü
adsb-how-far-ahead-leader-line = Öncü çizginin ne kadar ileriye uzandığı
adsb-one-minute-usual-radar-convention = Bir dakika olağan radar geleneğidir: çizgi, uçağın o sürede kat ettiği mesafe kadar uzundur; bu yüzden herhangi bir yakınlaştırmada iki eşit öncü, iki eşit hızdır. Sıfır bunları kapatır.
adsb-track-most = En fazla izle
adsb-aircraft-2 = {" "}uçak
adsb-aircraft-ground-are-placed-against = Yerdeki uçaklar istasyonun kendi konumuna göre yerleştirilir — bir yüzey squitter'ının kendine ait tek anlamlı bir çözümü yoktur — bu yüzden yakındaki havaalanı bir şey göstermezse dijital mod kurulumunda Lokatörüm'ü doldurun.
adsb-call = ÇAĞRI
adsb-icao = ICAO
adsb-v-s = D/H

## drm (src/app/drm.rs)

drm-waiting-receiver = alıcı bekleniyor…
drm-not-drm-set-mode-drm = DRM'de değil. Dijital bir kısa dalga yayınında modu DRM'e ayarlayın — skala bir yan banda değil kanalın merkezine gelir.
drm-no-drm-signal-locked-decoder = Hiçbir DRM sinyali kilitlenmedi. Çözücünün temiz bir taşıyıcıda birkaç saniyeye ihtiyacı vardır: DRM, okunabilmeden önce 400 ms ya da 2 sn boyunca serpiştirir.
drm-station-s-audio-codec-not = Bu istasyonun ses kodeki burada çözülmüyor. xHE-AAC, sistemde libfdk-aac gerektirir — içine derlenemez, çünkü lisansı ile bu programınkiler bağdaşmaz. Kurun (Debian/Ubuntu: libfdk-aac2, Arch: libfdk-aac, macOS: brew install fdk-aac, Windows: sdroxide.exe yanında libfdk-aac-2.dll) ve yeniden başlatın. Bu ekrandaki diğer her şey normal çözülüyor.
drm-quality = KALİTE
drm-last-minute = son dakika
drm-waiting-locked-signal = kilitli bir sinyal bekleniyor
drm-audio-frames-decoding-but-station = Ses çerçeveleri çözülüyor — ancak bu istasyonun kodeki burada çözülemiyor
drm-audio-frames-decoding = Ses çerçeveleri çözülüyor
drm-samples-reaching-decoder = Çözücüye ulaşan örnekler
drm-time = ZAMAN
drm-symbol-timing-recovered = Sembol zamanlaması geri kazanıldı
drm-frame = ÇERÇEVE
drm-transmission-frames-found = Bulunan yayın çerçeveleri
drm-fast-access-channel-what-transmission = Hızlı Erişim Kanalı — yayının ne olduğu
drm-service-description-channel-what-services = Hizmet Tanım Kanalı — hizmetlerin ne olduğu
drm-audio = SES
drm-constellation = KONSTELASYON
drm-waiting-decoded-symbols = çözülmüş semboller bekleniyor
drm-sampled-across-frame = çerçeve boyunca örneklendi
drm-whole-frame = tüm çerçeve
drm-mode = MOD
drm-robustness-mode-channel-width-ground = Dayanıklılık modu ve kanal genişliği. A, yer dalgası yolu içindir ve en çoğunu taşır; D, kötü saçılmış bir gök dalgası yolu içindir ve en azını taşır.
drm-interleave = SERPİŞTİRME
drm-how-far-transmission-spreads-each = Yayının her çerçeveyi zamanda ne kadar yaydığı. Uzun, daha derin sönümlemelere dayanır ve buna uygun olarak edinmesi daha uzun sürer.
drm-protection = KORUMA
drm-offset = KAYMA
drm-residual-sample-clock-error-against = Vericiye karşı artık örnekleme saati hatası. Büyük ve sabitse, alıcının referansı yanlıştır, yayın değil.
drm-doppler = DOPPLER
drm-delay = GECİKME
drm-doppler-delay-spread-path-how = Yolun Doppler ve gecikme yayılımı — ne kadar hızlı hareket ettiği ve yankılarının ne kadar uzak geldiği.
drm-service = HİZMET
drm-decode-service-multiplex = Çoklunun bu hizmetini çözün

## atchat (src/app/panels/atchat.rs)

atchat-starting-atchat-net-station = AtCHAT NET istasyonu başlatılıyor…
atchat-atchat = ATCHAT
atchat-net-multi-station-keyboard-file = NET — çok istasyonlu klavye ve dosya
atchat-net = {" "}AĞDA{" "}
atchat-net-click-leave-state-kept = Ağda — ayrılmak için tıklayın (durum korunur; yeniden katılmak için yeniden tıklayın)
atchat-rejoin = {" "}YENİDEN KATIL{" "}
atchat-off-net-click-rejoin = Ağ dışında — yeniden katılmak için tıklayın
atchat-off-net = ağ dışında
atchat-carrier = ● TAŞIYICI
atchat-another-station-transmitting-channel-busy = Başka bir istasyon yayın yapıyor — kanal meşgul.
atchat-tx = ● TX
atchat-setup = ⚙ KURULUM
atchat-station-callsign-virtual-channel-address = İstasyon çağrı işareti ve sanal kanal adresi
atchat-virtual-channel = SANAL KANAL
atchat-work-net-over-tcp-endpoint = Ağı radyo yerine bir TCP uç noktası üzerinden çalıştırın — radyosuz geliştirme için channel_server uyumlu bir döngü. Kapalıyken istasyonu havaya çıkarır.
atchat-roster = LİSTE
atchat-nobody-heard-yet = henüz kimse duyulmadı
atchat-chat = {" "}SOHBET{" "}
atchat-common-channel-lines-here-go = Ortak kanal — buradaki satırlar herkese gider (ALL)
atchat-log = {" "}GÜNLÜK{" "}
atchat-station-s-own-air-activity = İstasyonun kendi havadaki etkinliği — ana seçim, liste yaşlanması, ARQ yeniden denemeleri
atchat-no-activity-logged-yet = Henüz kayıtlı etkinlik yok.
atchat-no-messages-yet = Henüz ileti yok.
atchat-message-everyone-all = herkese ileti (ALL)
atchat-send = {" "}GÖNDER{" "}
atchat-transfers = AKTARIMLAR
atchat-all = → ALL
atchat-sent-file-follows-active-chat = Gönderilen bir dosya, etkin sohbet sekmesini izler
atchat-send-file = {" "}DOSYA GÖNDER{" "}
atchat-send-file-image-over-air = Havadan bir dosya ya da görüntü gönderin, blok-CRC-ARQ ile
atchat-nothing-flight = uçuşta hiçbir şey yok
atchat-images = GÖRÜNTÜLER
atchat-no-images-received = Alınmış görüntü yok.

## rds (src/app/rds.rs)

rds-station = İSTASYON
rds-diagnostics = TANILAMALAR
rds-which-programme-type-table-read = Bu istasyonun hangi program türü tablosuna göre okunacağı. RDS, Kuzey Amerika dışında her yerde kullanılır; RBDS, bir ABD istasyonunun kimlik kodunun yazdığı çağrı işaretini ekler. Otomatik, istasyonun gönderdiği genişletilmiş ülke kodunu izler ve biri gelene dek bir çağrı işareti yazmaz — okuyacağı kimlik aralıkları dokuz ülkenin RDS kodlarıyla paylaşılır, bu yüzden ülke kodu göndermeyen bir istasyonu adlandırmak için RBDS seçin.
rds-waiting-receiver = alıcı bekleniyor…
rds-no-rds-station-not-every = Bu istasyonda RDS yok. Her verici onu taşımaz ve zayıf bir verici, ulaşmadan taşıyor olabilir.
rds-rds-only-carried-wfm-broadcast = RDS yalnızca WFM yayın istasyonlarında taşınır.
rds-holding = TUTULUYOR
rds-block-sync-has-dropped-last = Blok senkronu düştü — bu okunan son istasyondur.
rds-identity = KİMLİK
rds-programme = PROGRAM
rds-described = ŞÖYLE TANIMLANIR
rds-content = İÇERİK
rds-music = Müzik
rds-speech = Konuşma
rds-traffic = TRAFİK
rds-announcement-air-now = duyuru şu anda havada
rds-carried-station = bu istasyon tarafından taşınıyor
rds-not-carried = taşınmıyor
rds-radio-text = RADYO METNİ
rds-station-clock = İSTASYON SAATİ
rds-country-code = ÜLKE KODU
rds-also = AYRICA YAYINDA
rds-also-tagged = AYRICA ETİKETLENDİ
rds-sync = SENKRON
rds-no-sync = SENKRON YOK
rds-blocks-failed-their-check-plus = Denetimini geçemeyen bloklar, artı hata düzelticinin onarmak zorunda kaldıklarının. Düzeltmeler kasıtlı olarak hata sayılır: artan bir rakam, bir istasyonun çözülmeyi bırakmak üzere olduğunun ilk işaretidir.
rds-front-end-overloading = ÖN UÇ AŞIRI YÜKLENİYOR
rds-converter-clipping-which-puts-distortion = Dönüştürücü kırpıyor; bu, çoklunun tamamına — 57 kHz veri alt taşıyıcısı dahil — bozulma koyar. RF kazancını kısın ya da bir zayıflatıcı devreye alın: RDS, herhangi bir şey duyulmadan yaklaşık bir desibel önce bozulur; bu yüzden mükemmel kulağa gelen bir istasyon, verisinin hayatta kalması için hâlâ çok güçlü olabilir.
rds-group-types = GRUP TÜRLERİ
rds-recent-groups-newest-last = SON GRUPLAR — en yeni en sonda
rds-nothing-decoded-yet = henüz hiçbir şey çözülmedi

## winlink (src/app/winlink.rs)

winlink-mail = POSTA
winlink-abort = İPTAL ET
winlink-stop-session-progress = Devam eden oturumu durdurun
winlink-connect = BAĞLAN
winlink-compose = YAZ
winlink-connecting = bağlanıyor…
winlink-not-connected-yet = henüz bağlı değil
winlink-loading = yükleniyor…
winlink-date = TARİH
winlink-from = GÖNDEREN
winlink-subject = KONU
winlink-no-subject = (konu yok)
winlink-delete-message = Bu iletiyi sil
winlink-arch = ARŞİV
winlink-move-archive = Arşive taşı
winlink-select-message = bir ileti seçin
winlink-reply = YANITLA
winlink-delete = SİL
winlink-to = Kime
winlink-cc = Bilgi
winlink-subject-2 = Konu
winlink-callsign-smtp-someone-example-org = Bir çağrı işareti ya da internet postası için SMTP:someone@example.org. Birkaçını virgülle ayırın.
winlink-file-outbox = GİDEN KUTUSUNA KOY
winlink-discard = AT
winlink-filed-messages-go-out-next = kaydedilen iletiler bir sonraki oturumda gider
winlink-mail-log = POSTA GÜNLÜĞÜ
winlink-session-running = oturum çalışıyor…
winlink-no-session-yet = henüz oturum yok

## wspr (src/app/panels/wspr.rs)

wspr-receptions = ALIMLAR
wspr-save-wspr-reception-list-csv = WSPR alım listesini CSV olarak kaydedin
wspr-wspr = WSPR
wspr-weak-signal-propagation-beacon = zayıf sinyal yayılım işareti
wspr-time-left-two-minute-slot = Bu iki dakikalık yuvada kalan süre. Sonunda kayıt çözücüye gider ve bir sonraki yayın kararı iner.
wspr-tx = ● TX
wspr-decoding = çözülüyor…
wspr-band = Bant
wspr-last-slot = Son yuva
wspr-nothing-heard = hiçbir şey duyulmadı
wspr-receive-only = yalnızca alma
wspr-next-slot = Sonraki yuva
wspr-waiting-radio-take-change = radyonun değişikliği almasını bekleniyor…
wspr-hop-next = Sonraki sıçrama
wspr-transmit = YAYIN
wspr-receive-only-2 = Yalnızca alma.
wspr-power = GÜÇ
wspr-what-you-actually-radiate = gerçekte yaydığınız
wspr-goes-out-message-everyone-who = Bu iletiye konur ve sizi duyan herkes yolu buna göre yargılar — dolayısıyla buradaki iyimser bir rakam, yalnızca sizinkini değil onların ölçümlerini de yanlış yapar.
wspr-roam = DOLAŞ
wspr-pick-different-offset-inside-200 = Her yayın için 200 Hz penceresi içinde farklı bir kayma seçin. Gelenek budur: herkesin paylaştığı iki yüz hertz, kimse ortasında park etmediğinde çalışır. Kapalıyken imlecin olduğu yerde tutar.
wspr-band-hop = BANT SIÇRAMA
wspr-move-dial-band-band-between = Yuvalar arasında skalayı bantlar arasında taşıyın; böylece bir alıcı spektrumun tamamını bir dilim yerine örnekler. VFO'yu kendiniz çevirirseniz durur — devam etmek için bunu iki kez basın.
wspr-upload = YÜKLE
wspr-send-what-station-decodes-wsprnet = Bu istasyonun çözdüğünü wsprnet.org'a gönderin. Duyduğunuzu bildirmek, bir WSPR alıcısını ağın parçası yapan şeydir; havaya hiçbir şey koymaz.
wspr-who-heard-me = BENİ KİM DUYDU
wspr-ask-wsprnet-org-every-few = wsprnet.org'a birkaç dakikada bir, hangi istasyonların bunu çözdüğünü sorun. WSPR'de hiçbir türde onay yoktur; bu yüzden bu, bir işaretin kendi erişimi hakkında bir şey öğrenmesinin tek yoludur.
wspr-bands = BANTLAR
wspr-none-selected-hopping-has-nowhere = hiçbiri seçilmedi — sıçramanın gidecek yeri yok
wspr-set-your-callsign-grid-general = Yayın yapmadan önce Ayarlar'ın Genel sekmesinde çağrı işaretinizi ve lokatörünüzü ayarlayın.

## packet (src/app/panels/packet.rs)

packet-monitor = İZLEYİCİ
packet-busy = MEŞGUL
packet-frames-arrived-but-failed-their = Gelen ama denetim dizisini geçemeyen çerçeveler — bir çarpışma, bir sönüm ya da okunamayacak kadar zayıf bir sinyal.
packet-setup = ⚙ KURULUM
packet-station-callsign-speed-tx-delay = İstasyon çağrı işareti, hız, yayın gecikmesi, digipeater yolu, işaret ve KISS sunucusu
packet-starting-packet-modem = paket modemi başlatılıyor…
packet-nothing-heard-yet = henüz hiçbir şey duyulmadı
packet-terminal = TERMİNAL
packet-connected = {" "}BAĞLI{" "}
packet-calling = {" "}ÇAĞRILIYOR{" "}
packet-idle = {" "}BOŞTA{" "}
packet-extended-sequence-numbers-window-up = Genişletilmiş sıra numaraları — 127 çerçeveye kadar bir pencere.
packet-frames-sent-not-yet-acknowledged = Gönderilmiş ve uzak uç tarafından henüz onaylanmamış çerçeveler.
packet-retries-against-link-s-limit = Bu bağlantının sınırına karşı yeniden denemeler. Onaylanmamış çerçeveler yerinde dururken artan bir sayı, bu taraftan bakıldığında sönen bir yolun görünümüdür.
packet-mail-window-using-link = POSTA penceresi bağlantıyı kullanıyor.
packet-one-radio-one-channel-one = Bir radyo, bir kanal, bir bağlantı. Bir Winlink oturumu ve bu terminal, onu kullanmanın iki yoludur ve önce isteyen onu alır.
packet-digipeaters-nearest-first-separated-commas = Digipeater'lar, en yakın önce, virgülle ayrılmış — OE3XLR-1,OE3XMS-1. Doğrudan duyabildiğiniz bir istasyon için ya da paket ayarlarındaki varsayılan yolu kullanmak için boş bırakın.
packet-disconnect = {" "}BAĞLANTIYI KES{" "}
packet-hang-up-properly-wait-far = Düzgünce kapatın ve uzak ucun kabul etmesini bekleyin. Bunun yerine kipi değiştirmek, oturumu kimseye söylemeden bitirir.
packet-connect = {" "}BAĞLAN{" "}
packet-mail-window-has-link-finish = POSTA penceresi bağlantıya sahip. Önce o oturumu bitirin ya da durdurun.
packet-station-has-no-callsign-yet = Bu istasyonun henüz çağrı işareti yok — KURULUM altında, izleyici bölmesinin üstünde bir tane ayarlayın. O ayarlanana dek hiçbir şey yayınlanmaz.
packet-call-station-connected-mode-node = Bu istasyonu bağlantılı modda çağırın — bir düğüm, bir BBS ya da başka bir operatör.
packet-needs-callsign-call = Aranacak bir çağrı işareti gerekir.
packet-clear = {" "}TEMİZLE{" "}
packet-empty-transcript-link-untouched-connected = Dökümü boşaltın. Bağlantıya dokunulmaz — bağlı bir istasyon bağlı kalır.
packet-not-connected-type-callsign-above = Bağlı değil. Yukarıya bir çağrı işareti yazın ve bir düğümü ya da BBS'yi çağırmak için BAĞLAN'a basın.
packet-type-here = buraya yazın
packet-not-connected = bağlı değil
packet-send = {" "}GÖNDER{" "}
packet-send-line-node-bbs-reads = Satırı gönderin. Bir düğüm ya da BBS bir seferde bir satır okur.

## fsq (src/app/panels/fsq.rs)

fsq-speed = Hız
fsq-call = Çağrı
fsq-callsign-directed-fsqcall-messages-defaults = Yönlendirilmiş (FSQCALL) iletiler için çağrı işareti; varsayılanı çağrı işaretiniz
fsq-tx = ● TX
fsq-contacts = KİŞİLER
fsq-heard = DUYULAN
fsq-none = — yok —
fsq-allcall = ALLCALL
fsq-image = GÖRÜNTÜ
fsq-clear = TEMİZLE
fsq-forget-pictures-received-so-far = Şimdiye dek alınan resimleri unutun
fsq-send-image = Görüntü gönder…
fsq-right-click-remove = Kaldırmak için sağ tıklayın
fsq-remove-picture = Bu resmi kaldır
fsq-listening = — dinleniyor —
fsq-message = İleti…
fsq-send = {" "}GÖNDER{" "}
fsq-call-cq = {" "}CQ ÇAĞIR{" "}
fsq-heard-2 = {" "}? duyuldu{" "}
fsq-clear-2 = {" "}TEMİZLE{" "}
fsq-fsq-contacts = FSQ Kişileri
fsq-add = Ekle

## publicsdr (src/app/publicsdr.rs)

publicsdr-tab = + SEKME
publicsdr-open-receiver-new-radio-own = Bu alıcıyı kendi sekmesinde yeni bir radyo olarak açın
publicsdr-point-radio-receiver-replacing-whatever = *Bu* radyoyu, şu an hangi arabirimde olursa olsun alıcıya yöneltin — bu bir radyonun tüm kurulumu olduğu için önce yeniden sorulur
publicsdr-not-stated = belirtilmemiş
publicsdr-replace = DEĞİŞTİR
publicsdr-point-radio-you-are-receiver = Bulunduğunuz radyoyu bu alıcıya yöneltin
publicsdr-tab-instead = + SEKME YERİNE
publicsdr-open-receiver-another-radio-leave = Alıcıyı başka bir radyo olarak açın ve bunu olduğu gibi bırakın
publicsdr-cancel = İPTAL
publicsdr-public-sdrs = GENEL SDR'LER
publicsdr-available = KULLANILABİLİR
publicsdr-hide-receivers-are-full-ones = Dolu olan alıcıları ve operatörü tarayıcı dışındaki uygulamalara hiç kanal açmamış olanları gizleyin
publicsdr-band = BANTTA
publicsdr-only-receivers-cover-current-dial = Yalnızca geçerli skala frekansını kapsayan alıcılar
publicsdr-low-bw = DÜŞÜK BW
publicsdr-take-spyserver-low-bandwidth-shape = Bir SpyServer'ı düşük bant genişliği biçiminde alın: skalayı izleyen dar bir I/Q penceresi artı sunucunun kendi bant görünümü; megabit genişbant I/Q yerine. Yalnızca bir biçimi olan bir KiwiSDR'ı etkilemez.
publicsdr-refresh = ⟳ YENİLE
publicsdr-fetch-both-directories-again = İki dizini yeniden alın
publicsdr-name-place-antenna-band = ad, yer, anten, bant
publicsdr-clear-search = Aramayı temizle
publicsdr-fetching-receiver-lists = Alıcı listeleri alınıyor…
publicsdr-machine-radio-attached-does-not = Bu radyonun bağlı olduğu makine aygıt sorularını yanıtlamıyor, bu yüzden listeleri de alamaz.
publicsdr-no-match = eşleşme yok
publicsdr-nothing-matches-try-turning-filter = Hiçbir şey eşleşmiyor — bir filtre çipini yeniden açmayı deneyin
publicsdr-radio = bu radyo
publicsdr-radio-s-configuration-has-not = Bu radyonun yapılandırması, bulunduğu makineden henüz gelmedi; bu yüzden alıcıya yöneltilecek bir şey yok — bunun yerine + SEKME ile alın.
publicsdr-position-not-published-so-nothing = {" "}Konumu yayımlanmıyor, bu yüzden duyduğu hiçbir şey bildirilmeyecek.

## hd (src/app/hd.rs)

hd-hd-radio = HD Radio
hd-waiting-receiver = alıcı bekleniyor…
hd-not-hd-radio-set-mode = HD Radio'da değil. Dijital yan taşıyıcıları taşıyan bir FM yayınında modu HD Radio'ya ayarlayın — skala analog taşıyıcının merkezine gelir, yanına değil.
hd-no-hd-radio-lock-digital = HD Radio kilidi yok. Dijital yan bantlar analog taşıyıcının yaklaşık 20 dB altında yayınlanır; bu yüzden dinlenebilir bir istasyon bile çözülemeyecek kadar gürültülü olabilir: çözücünün temiz bir tanesinde birkaç saniyeye ihtiyacı vardır.
hd-sync = SENKRON
hd-ofdm-frame-timing-recovered = OFDM çerçeve zamanlaması geri kazanıldı
hd-audio = SES
hd-audio-frames-decoding = Ses çerçeveleri çözülüyor
hd-quality = KALİTE
hd-lower = ALT
hd-upper = ÜST
hd-last-minute = son dakika
hd-waiting-locked-signal = kilitli bir sinyal bekleniyor
hd-mer-l = MER L
hd-modulation-error-ratio-lower-digital = Alt dijital yan bandın modülasyon hata oranı.
hd-mer-u = MER U
hd-modulation-error-ratio-upper-digital = Üst dijital yan bandın modülasyon hata oranı.
hd-cber = CBER
hd-channel-bit-error-ratio-after = İç kodun ardından kanal bit hata oranı. Yaklaşık 1e-3'ün altında dış kod onu düzeltebilir; bunun üstünde ses bozulmaya başlar.
hd-offset = KAYMA
hd-residual-carrier-frequency-offset-large = Artık taşıyıcı frekans kayması. Büyük ve sabitse, alıcının referansı yanlıştır, yayın değil.
hd-psmi = PSMI
hd-primary-service-mode-indicator-which = Birincil Hizmet Modu Göstergesi: istasyonun yayınladığı FM hibrit hizmet modlarından hangisi.
hd-programme = PROGRAM
hd-restricted-access = {" "}— kısıtlı erişim

## bands (src/app/bands.rs)

bands-bands = BANTLAR
bands-daylight = ☀ GÜNDÜZ
bands-night = ☾ GECE
bands-no-forecast-yet-fetching-hamqsl = · henüz tahmin yok — HAMQSL.com'dan alınıyor
bands-band = BANT
bands-condx = KOŞ
bands-paths = YOLLAR
bands-reach = ERİŞİM
bands-best = EN İYİ
bands-condx-global-forecast-rest-what = KOŞ genel bir tahmindir; gerisi gerçekten duyulanlardır. Boş bir satır, hiçbir şeyin çözülmediği anlamına gelir — bu, bandın kapalı olduğu ya da yalnızca üzerinde kimsenin olmadığı anlamına gelebilir.
bands-ibp-beacons = IBP İŞARETLERİ
bands-meteor-showers = METEOR YAĞMURLARI
bands-peak = TEPE

## spots (src/app/spots.rs)

spots-pota = POTA
spots-sota = SOTA
spots-freedv = FREEDV
spots-spots = SPOTLAR
spots-longwave-shortwave-broadcast-stations-air = Şu anda havada olan uzun dalga ve kısa dalga yayın istasyonları
spots-view = GÖRÜNÜMDE
spots-only-spots-inside-panadapter-span = Yalnızca panadaptör aralığının içindeki spotlar
spots-setup = ⚙ KURULUM
spots-feeds-lookup-upload-settings = Beslemeler, sorgu ve yükleme ayarları
spots-call-station-site-frequency = çağrı, istasyon, yer, frekans
spots-clear-search = Aramayı temizle
spots-no-match = eşleşme yok
spots-no-spots-enable-feed-setup = spot yok — ⚙ KURULUM içinde bir besleme etkinleştirin
spots-nothing-matches-search = Aramayla eşleşen hiçbir şey yok

## ism (src/app/ism.rs)

ism-ism-devices = ISM AYGITLARI
ism-decoding = ÇÖZÜLÜYOR
ism-needs-wideband-iq-source-cat = Geniş bant bir IQ kaynağı gerektirir; bir CAT telsizi yalnızca ses gönderir
ism-how-far-burst-must-stand = Bir patlamanın, çözülmeden önce kanalın kendi gürültü tabanının ne kadar üstünde durması gerektiği
ism-give-each-band-width-normally = Her banda normalde ihtiyaç duyduğu genişliği verin — 315, 345 ve 433 MHz'deki OOK bantları için bir megahertzin dörtte biri, 868 ve 915 için tam bir megahertz.
ism-several-hundred-more-device-decoders = rtl_433 projesinden, içine derlenmiş birkaç yüz aygıt çözücüsü daha. sdroxide'ın kendi çözücülerinin hiç ulaşamadığı 433 MHz OOK aygıtlarını okur. İkisi de bir aygıtı okuyabildiğinde, rtl_433 bu çeşitlerin çok daha fazlasını bilir ve devralır.
ism-t-1-your-own = 1 sizinki
ism-device-decoders-registered-how-many = Kayıtlı aygıt çözücüleri, kaçının rtl433_flex.conf dosyanızdan geldiği ve başladığından beri kaç yayının çözüldüğü.
ism-reload-decoders = ÇÖZÜCÜLERİ YENİDEN YÜKLE
ism-re-read-rtl433-flex-conf = Düzenledikten sonra rtl433_flex.conf dosyasını yeniden okuyun. Duyulmuş aygıtlar listede kalır.
ism-rtl-433-knows-more-variants = rtl_433, bunların çeşitlerini sdroxide'ın kendi çözücülerinden daha fazla bilir; bu yüzden burada dinlerken bunları okur. Başka bir bant seçin, sdroxide'ın kendi çözücüleri yeniden devralır.
ism-decoder-your-rtl433-flex-conf = rtl433_flex.conf dosyanızdaki bir çözücü reddedildi. Diğerleri yine yüklendi. Satırı düzeltin, sonra YENİDEN YÜKLE'ye basın.
ism-waiting-decoder = çözücü bekleniyor…
ism-where-decoder-actually-listening-follows = Çözücünün gerçekte nerede dinlediği. Bu, VFO'yu değil alıcının donanım merkezini izler — geniş bir ön uçta ikisi çok uzakta olabilir ve hangi kanalların ulaşılabilir olduğuna karar veren pencere odur.
ism-transmissions-gate-opened-how-many = Kapının açıldığı yayınlar ve kaçının geçerli bir çerçeve ürettiği. Birçok patlama ve hiç çözüm yok, bandın sdroxide'ın henüz okuyamadığı aygıtlarla meşgul olduğu anlamına gelir. Hiç yoksa, hiçbir şeyin eşiğe ulaşmadığı anlamına gelir — onu düşürün ya da skalanın bantta olduğunu denetleyin.
ism-nothing-heard-yet = henüz hiçbir şey duyulmadı
ism-click-tune-receiver =


    Alıcıyı ona ayarlamak için tıklayın

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

navtex-messages = İLETİLER
navtex-sync = SENKRON
navtex-character-phase-locked-signal-being = Karakter fazı kilitli — bir sinyal okunuyor.
navtex-no-character-phase-yet-no = Henüz karakter fazı yok: sinyal yok ya da bu sinyal değil.
navtex-characters-whose-first-copy-was = İlk kopyası bozuk olan ve beş yuva sonra tekrardan alınan karakterler ve ikisi de bozuk olanlar. Bir NAVTEX karakteri iki kez gönderilir; hata düzeltmesinin tamamı budur.
navtex-swap-mark-space-tones-signal = Diğer yan bantta alınan bir sinyal için işaret ve boşluk tonlarını değiştirin. Kapalıysa kanal frekansında üst yan banttır; hizmet için yayımlanmış her ayar talimatının dediği budur.
navtex-starting-navtex-receiver = NAVTEX alıcısı başlatılıyor…
navtex-nothing-yet-station-transmits-ten = Henüz hiçbir şey yok — bir istasyon dört saatte bir on dakika yayın yapar
navtex-cut-short = kısaltıldı
navtex-message = İLETİ
navtex-time-day-stated-message-utc = İletide belirtilen günün saati (UTC): yalnızca gösterim
navtex-receiving = ALINIYOR
navtex-monitor = İZLEYİCİ
navtex-everything-decoded-message-not = İleti olsun olmasın, çözülen her şey

## rfpaint (src/app/panels/rf_paint.rs)

rfpaint-rf-paint = RF BOYAMA
rfpaint-scan-speed = Tarama hızı
rfpaint-how-fast-text-image-scanned = Metnin/görüntünün şelaleye ne kadar hızlı tarandığı. Düşük olması daha yavaş ve daha okunaklı demektir; %100 = temel hız, %25 (merkez) varsayılandır.
rfpaint-abort = İptal
rfpaint-text-paint = METİN BOYAMA
rfpaint-type-text-paint = Boyamak için metin yazın…
rfpaint-preview-waterfall = ŞELALE ÖNİZLEME
rfpaint-type-text-preview = önizlemek için metin yazın
rfpaint-transmit = {"  "}YAYINLA{"  "}
rfpaint-image-paint = GÖRÜNTÜ BOYAMA
rfpaint-no-image-loaded = yüklenmiş görüntü yok
rfpaint-load-image = Görüntü yükle…
rfpaint-load-image-preview = önizlemek için bir görüntü yükleyin

## hfdl (src/app/hfdl.rs)

hfdl-listen = DİNLE
hfdl-switches-engine-s-hfdl-downconverter = Motorun HFDL aşağı dönüştürücüsünü ve çözücüsünü açar ya da kapatır. Çözme, pencere açık olsun ya da olmasın 24 kHz'lik bir şerit ve bir çalışan iş parçacığı maliyeti taşır.
hfdl-channel-assigned-frequency-listen-anything = Dinlenecek kanal (atanmış frekans). Bantta her şey çalışır — HFDL istasyonları 2,8 ile 22 MHz arasında yayın yapar — buraya ortalanmış sabit 24 kHz'lik bir şeride karşı.
hfdl-save-hfdl-decode-log-file = HFDL çözüm günlüğünü bir dosyaya kaydedin
hfdl-riverhead-northern-atlantic-north-america = Riverhead (kuzey Atlantik/kuzey Amerika) — varsayılan ve çözücünün doğrulandığı kanal.
hfdl-hfdl-assigned-frequency = Bir HFDL atanmış frekansı.
hfdl-one-your-saved-channels-click = Kayıtlı kanallarınızdan biri — ayarlamak için tıklayın, kaldırmak için sağ tıklayın.
hfdl-hfdl-selected-but-decoding-off = HFDL seçili ama çözme kapalı — yukarıdaki DİNLE'ye basın
hfdl-decodes = ÇÖZÜMLER
hfdl-nothing-decoded-yet-ground-station = Henüz hiçbir şey çözülmedi — kanalda bir yer istasyonu olduğunda, bir yer istasyonunun squitter'ı yaklaşık 32 sn'de bir tekrarlanır.
hfdl-no-decode-matches-filter = Filtreyle eşleşen hiçbir çözüm yok.
hfdl-decoding = ÇÖZÜLÜYOR
hfdl-decoding-off = ÇÖZME KAPALI
hfdl-dbfs = — dBFS

## rade (src/app/panels/rade.rs)

rade-rade = RADE
rade-freedv-v1-digital-voice = FreeDV V1 dijital ses
rade-tx = ● TX
rade-mute-analog = ANALOG'U SUSTUR
rade-mute-demodulated-audio-so-only = Demodüle edilmiş sesi susturun; böylece yalnızca çözülmüş konuşma duyulur. Aksi halde modemin çalacak bir şeyi olmadığında ham sinyal geçirilir — o tıslama, bir yayını senkron olmadan önce bulmanın yoludur; bu yüzden ayarlarken bunu kapalı bırakın.
rade-sync = SENKRON
rade-how-far-received-signal-sits = Alınan sinyalin modemin beklediği yerden ne kadar uzakta durduğu. Büyük değerler yine çözülür — edinme döngüsü onları izler — ama bunu sıfıra yakın getirmek için skalayı biraz oynatmak en iyi payı verir.
rade-snr = SNR —
rade-callsign-station-put-end-over =
    İstasyonun Over Sonu çerçevesine koyduğu çağrı işareti — bir RADE yayınında bir tane gönderilen tek nokta. Ayrıca FreeDV Reporter'a bildirilen de budur; böylece duyduğunuz bir istasyon qso.freedv.org üzerinde sizin tarafınızdan duyulmuş olarak görünür.

    Sinyal gittikten sonra bir dakika tutulur ve kendi yayınınızı başlattığınızda düşer: alıcının şimdiye dek duyduğu her istasyonu değil, frekanstaki istasyonu adlandırır.
rade-stop-talking = KONUŞMAYI DURDUR
rade-talk-ptt = KONUŞ (PTT)
rade-open-close-rade-over-modem = Bir RADE yayını açın ya da kapatın. Modem, ilk çerçeve gitmeden önce yaklaşık 120 ms konuşmaya ihtiyaç duyar ve durduğunuzda bir over sonu çerçevesi gönderir; bu yüzden yayın, düğmenin biraz ötesinde sürer.
rade-rade-v1-occupies-roughly-1060 = RADE V1, geçiş bandının yaklaşık 1060–1880 Hz'ini kaplar — üst yan bant, ya da RADE'in telefon pratiği yaptığı ve sdroxide'ın sizin için değiştirdiği 160, 80 ve 40 m'de alt yan bant. Sinyali şelaledeki gölgeli bandın içine koyun; modem onu oradan bulur.

## frame (src/app/frame.rs)

frame-acknowledge = Onayla
frame-dismiss = Kapat
frame-get-sdroxide-com = sdroxide.com'dan edinin
frame-reconnecting = Yeniden bağlanılıyor…
frame-not-connected = Bağlı değil.
frame-reconnect-now = Şimdi yeniden bağlan
frame-transmit-lockout-disabled = ⚠ YAYIN KİLİDİ DEVRE DIŞI
frame-engine-was-started-oob-tx = Bu motor --oob-tx ile başlatıldı. Amatör bant kilidi kapalı: donanımın desteklediği herhangi bir frekansta verici anahtarlanır.
frame-transmitting-outside-your-licence-offence = Lisansınızın dışında yayın yapmak, lisans veren her ülkede bir suçtur. Yalnızca anahtarlamak üzere olduğunuz frekansları kullanmaya yetkiliyseniz devam edin — bir MARS/CAP ya da ticari lisans, deneysel bir izin ya da bir yapay yük.
frame-i-understand = {"  "}ANLIYORUM{"  "}
frame-restart-without-oob-tx-put = Kilidi geri koymak için --oob-tx olmadan yeniden başlatın.

## smeter (src/widgets/smeter.rs)

smeter-alc-how-hard-transmitter-being = ALC — verici, çıkarabileceği en fazlanın yüzdesi olarak ne kadar sürülüyor. %100 tam ölçektir, watt cinsinden tam güç değil: bunun antende ne değerinde olduğu radyoya ve Sürüş'e bağlıdır.
smeter-radio-reports-no-forward-power =


    Bu radyo ileri güç bildirmez; bu yüzden ALC'nin yanındaki rakam watt değil yeniden ALC okumasıdır.
smeter-no-swr-radio-has-no =


    SWR yok: bu radyonun onu ölçecek bir köprüsü yok ve sdroxide bir tane uydurmaz — yalnızca radyonun kendisinin bildirdiğini gösterebilir. Bir SWR metresi olan bir telsiz (CAT, TCI ya da LAN bağlantısı üzerinden) ve bir HPSDR kartı bunu doldurur; düz bir SDR verici doldurmaz.
smeter-no-signal-report-yet-receiver = Henüz sinyal raporu yok — alıcı bir metre okuması vermedi.
smeter-s9-73-dbm-each-s =


    S9, −73 dBm'dir ve altındaki her S birimi 6 dB'dir. dBm rakamı, ön ucun kalibrasyonu kadar dürüsttür — kalibre edilmemiş bir alıcı dBFS'i bir dBm etiketiyle bildirir.
smeter-ovld-converter-into-rails-so =


    OVLD: dönüştürücü sınırına dayanmış, bu yüzden gösterilen seviye sinyali olduğundan az gösterir. Ön uç kazancını geri çekin.
smeter-table-held = , tablo tutuldu
smeter-ps = PS --

## multi (src/multi.rs)

multi-tx = ● TX
multi-link = BAĞLANTI
multi-mute-radio-s-audio = Bu radyonun sesini sustur
multi-close-radio-s-split-view = Bu radyonun bölünmüş görünümünü kapat
multi-open-radio-split-view-own = Bu radyoyu kendi bölünmüş görünümünde açın
multi-already-open-another-split-view = Zaten başka bir bölünmüş görünümde açık
multi-show-radio-here = Bu radyoyu burada göster
multi-add-radio-computer-settings-radio = Bu bilgisayarda bir radyo ekleyin (bir istasyonda eklemek için Ayarlar → Radyo)
multi-add-radio = Bir radyo ekle
multi-asked-station-another-radio = İstasyondan başka bir radyo istendi…
multi-client-cannot-open-connection = Bu istemci bir bağlantı açamaz.

## net (src/app/net.rs)

net-fail = BAŞARISIZ
net-will-retry = {" "}(yeniden denenecek)
net-full-resync-downloading = Tam yeniden eşitleme: indiriliyor…
net-downloading = İndiriliyor…
net-full-resync-log-already-matches = Tam yeniden eşitleme: günlük zaten Wavelog ile eşleşiyor
net-full-resync-review-changes-below = Tam yeniden eşitleme: aşağıdaki değişiklikleri gözden geçirin
net-full-resync-discarded-nothing-changed = Tam yeniden eşitleme atıldı; hiçbir şey değişmedi
net-adif-import-failed-file-could = ADIF içe aktarma başarısız: dosya ayrıştırılamadı
net-channel-import-failed-file-could = Kanal içe aktarma başarısız: dosya okunamadı
net-channel-import-nothing-read-chirp = Kanal içe aktarma: okunacak bir şey yok — bir CHIRP CSV dosyası, sütunlarını adlandıran bir başlık satırıyla başlar ve bunlardan biri Frequency (Frekans) olmalıdır

## widgets (src/app/panels/widgets.rs)

widgets-free-text = serbest metin
widgets-hashed-callsign-not-yet-heard = karıştırılmış çağrı işareti, henüz tam olarak duyulmadı
widgets-entity-unknown = varlık bilinmiyor
widgets-band = bu bant
widgets-new-entity-never-worked-any = Yeni varlık — hiçbir bantta hiç çalışılmamış
widgets-new-grid-square = Yeni lokatör karesi
widgets-not-worked-before = Daha önce çalışılmamış
widgets-worked-before-but-not-band = Daha önce çalışılmış, ama bu bantta değil
widgets-call-queue = Çağrı kuyruğunda
widgets-image = Görüntü

## pi4 (src/app/panels/pi4.rs)

pi4-receptions = ALIMLAR
pi4-save-pi4-reception-list-csv = PI4 alım listesini CSV olarak kaydedin
pi4-next-generation-beacon = Yeni Nesil İşaret
pi4-time-left-one-minute-beacon = Bu bir dakikalık işaret döngüsünde kalan süre. PI4 iletisi, pencere bitişinin ötesine dolacak kadar zaman bulduktan sonra aranır.
pi4-decoding = çözülüyor…
pi4-band = Bant
pi4-last-cycle = Son döngü
pi4-nothing-heard = hiçbir şey duyulmadı
pi4-receive-only-tune-so-beacon = Yalnızca alma. İşaretin CW kimliği 800 Hz ses frekansına oturacak şekilde ayarlayın — ağın kendi geleneği — ve dört PI4 tonu, bu çözücünün varsayılan olarak aradığı yere düşsün.

## awards (src/app/awards.rs)

awards-awards = ÖDÜLLER
awards-band = Bant
awards-all = Tümü
awards-dxcc = DXCC
awards-grids = Lokatörler
awards-worked-all-states = Tüm Eyaletler Çalışıldı
awards-cq-zones-waz = CQ Bölgeleri (WAZ)
awards-dxcc-entities = DXCC varlıkları

## acars (src/app/panels/acars.rs)

acars-acars = ACARS
acars-audio-decoder-s-passband = Çözücünün geçiş bandındaki ses.
acars-blocks-framed-blocks-whose-check = Çerçevelenen bloklar ve denetim dizisi başarısız olan bloklar.
acars-starting-acars-receiver = ACARS alıcısı başlatılıyor…
acars-channel = KANAL
acars-tune-dial-here = Skalayı buraya ayarlayın.
acars-nothing-yet-acars-quiet-between = Henüz hiçbir şey yok. ACARS uçaklar arasında sessizdir — onu bir kanalda bırakın ve bir patlama bekleyin.
acars-check-failed = denetim başarısız
acars-block-check-sequence-did-not = Blok-denetim dizisi eşleşmedi — metin yanlış olabilir.
acars-no-text = (metin yok)

## login (src/login.rs)

login-sdr-oxide = SDR OXIDE
login-username = Kullanıcı adı
login-password = Parola
login-remember-device = Bu cihazda hatırla
login-signs-without-asking-next-time = Bir dahaki sefere sormadan oturum açar ve 3B görünümün sekmesini de hiç sormadan açar. Parola açık olarak tutulur, bu yüzden başkalarının kullandığı bir makinede bunu kapalı bırakın.
login-keeps-password-device-clear = Parolayı bu cihazda açık olarak tutar.
login-remember-station = Bu istasyonu hatırla
login-remember-me = BENİ HATIRLA
login-station-keeps-sign-in = Oturumunuzu bu sayfa değil, istasyon saklar — diğer telsizlerde doldurulacak bir şey yok, ayrılırken unutulacak bir şey yok.
login-not-this-time = Bu sefer değil: tarayıcı kapanınca bunu unutur; bir sonraki oturumda yeniden sorulur ve bu oturumdan cihazda hiçbir şey kalmaz.
login-checking = {" "}DENETLENİYOR…{" "}
login-sign = {" "}OTURUM AÇ{" "}
login-wrong-password-shuts-server-s = Yanlış bir parola, sunucunun kapısını birkaç saniyeliğine kapatır; bu yüzden bir an sürebilir.

## help (src/help.rs)

help-find = BUL
help-find-manual = kılavuzda bul
help-previous-match-shift-enter-shift = Önceki eşleşme (Shift+Enter, Shift+F3)
help-next-match-enter-f3 = Sonraki eşleşme (Enter, F3)
help-no-matches = eşleşme yok
help-clear-search-esc = Aramayı temizle (Esc)
help-sdroxide-manual = SDROXIDE KILAVUZU
help-contents = İÇİNDEKİLER

## aismap (src/ais_map.rs)

aismap-t-1-station-heard-waiting-report = 1 istasyon duyuldu — bir konum bildirmesi bekleniyor
aismap-double-click-reframe = yeniden çerçevelemek için çift tıklayın
aismap-zoom-out-wheel-pinch-do = Uzaklaştırın — tekerlek ya da kıstırma da bunu yapar
aismap-zoom-about-middle-chart = Haritanın ortası etrafında yakınlaştırın
aismap-frame-everything-being-tracked-again = İzlenen her şeyi yeniden çerçeveleyin — çift tıklama da yapar

## specview (src/widgets/spectrum_view.rs)

specview-waiting-spectrum = spektrum bekleniyor…
specview-hounds = HOUNDS

## spectrum (src/app/spectrum.rs)

spectrum-window-has-no-gpu-renderer = Bu pencerenin GPU çizicisi yok, bu yüzden şelale standart genişliğinde kalır.
spectrum-machine-drawing-without-gpu-every = Bu makine bir GPU olmadan çiziyor — şelalenin her sütunu işlemcinin işi ve zaten bunu radyoyla paylaşıyor.
spectrum-window-renders-through-opengl-which = Bu pencere, sdroxide'ın uyumluluk yolu olan OpenGL üzerinden çizer — bir Raspberry Pi, eski bir grafik çipi ya da WebGPU'suz bir tarayıcı. Orada daha geniş bir şelale kare hızına değmez.
spectrum-wider-than-renderer-can-draw = Bu çizicinin çizebileceğinden geniş.

## macros (src/app/panels/macros.rs)

macros-nocall = NOCALL
macros-each-button-sends-whole-text = Her düğme tüm metnini bir seferde gönderir. F2–F10 ilk dokuzunu basar; ekranda klavyeyi tutan bir şey olmadığı sürece.
macros-remove-button = Bu düğmeyi kaldır

## gridtracker (src/app/grid_tracker.rs)

gridtracker-grid-tracker = LOKATÖR İZLEYİCİ
gridtracker-shade-squares-heard-live-decode = Yalnızca günlükteki değil, canlı çözüm listesinde duyulan kareleri gölgeleyin — şu anda havada olan, çalışılmış karelerin altında camgöbeği.
gridtracker-drag-pan-wheel-zoom-amber = Kaydırmak için sürükleyin, yakınlaştırmak için tekerlek. Kehribar = çalışıldı, yeşil = onaylandı, camgöbeği = duyuldu.

## remote (src/remote.rs)

remote-server-busy-another-client-connected = sunucu meşgul — başka bir istemci bağlı
remote-transmitting-but-client-s-microphone = Yayın yapılıyor, ama bu istemcinin mikrofonu hiç ses üretmiyor — hiçbir şey modüle edilmiyor. Bir tarayıcıdaysanız, sayfaya mikrofonun izin verildiğini ve başka bir uygulamanın onu tutmadığını denetleyin.
remote-connection-closed = bağlantı kapandı

## solarwin (src/solar3d/mod.rs)

solarwin-sdroxide-solar-system = sdroxide — güneş sistemi

## solarbtn (src/app/solar.rs)

solarbtn-solar-system-3d-view-sun = Güneş sistemi 3B görünüm — Güneş, Dünya, Ay, güneş lekeleri ve CME'ler (ayrı pencere)
solarbtn-solar-system-3d-view-sun-2 = Güneş sistemi 3B görünüm — Güneş, Dünya, Ay, güneş lekeleri ve CME'ler (yeni tarayıcı sekmesi)

## speech (src/app/speech.rs)

speech-loading-voice = Ses yükleniyor…
speech-browser-client-cannot-speak-yet = tarayıcı istemcisi henüz konuşamıyor

## freqdisplay (src/widgets/freq_display.rs)

freqdisplay-double-click-type-frequency = Bir frekans yazmak için çift tıklayın
freqdisplay-tap-type-frequency = Bir frekans yazmak için dokunun

## solarstate (src/solar3d/state.rs)

solarstate-home = BAŞA
solarstate-dwarf-planets = CÜCE GEZEGENLER

## adsbmap (src/adsb_map.rs)

adsbmap-double-click-reframe = yeniden çerçevelemek için çift tıklayın

## aprsmap (src/aprs_map.rs)

aprsmap-double-click-reframe = yeniden çerçevelemek için çift tıklayın

## solarnet (src/solar3d/net.rs)

solarnet-connection-closed = bağlantı kapandı

## alerts (src/app/alerts.rs)

alerts-could-not-start-alert-thread = uyarı iş parçacığı başlatılamadı

## hfdlmap (src/hfdl_map.rs)

hfdlmap-no-aircraft-located-yet = henüz konumlandırılmış uçak yok

## worldmap (src/widgets/worldmap.rs)

worldmap-double-click-reframe = YENİDEN ÇERÇEVELEMEK İÇİN ÇİFT TIKLAYIN

## duopower (src/app/duo_power.rs)

duopower-sets-fdm-duo-s-own = FDM-DUO'nun kendi güç ayarını (menü 34 TX POWER) belirler. Sürüş ve Ayar, bunun içinde gücü ayarlar; SDRoxide her bağlandığında ayarı yeniden gönderir.

## Bant koşulları penceresi: bir tepe tarihi.

# $month 1–12'dir.
bands-peak-date = { $day } { $month ->
        [1] Oca
        [2] Şub
        [3] Mar
        [4] Nis
        [5] May
        [6] Haz
        [7] Tem
        [8] Ağu
        [9] Eyl
        [10] Eki
        [11] Kas
       *[12] Ara
    }

## download (src/download.rs)

download-browser-could-not-read-file = tarayıcı bu dosyayı okuyamadı

## morse (src/app/morse.rs)

morse-translate = ÇEVİR
morse-practice = ALIŞTIR
morse-learn = ÖĞREN
morse-morse = MORSE
morse-text-morse = Metin → Morse
morse-morse-text = Morse → Metin
morse-per-character-space-between-characters = Karakter başına `.` ve `-`, karakterler arasında bir boşluk, kelimeler arasında ` / `. ITU alfabesi, rakamlar, noktalama işaretleri ve yaygın prosignlar.
morse-reference-translator-available-desktop-build = Başvuru çevirmeni masaüstü sürümünde kullanılabilir.
morse-play-what-you-type-your = Yazdığınızı kendi hoparlörlerinizden çalın
morse-play = ÇAL
morse-stop = DURDUR
morse-every-character-learned-well-done = Her karakter öğrenildi — tebrikler.
morse-drill-needs-sound-which-desktop = Alıştırma sese ihtiyaç duyar; masaüstü sürümünde bu vardır.
morse-reveal = GÖSTER
morse-reset = SIFIRLA
morse-replay = TEKRAR ÇAL
morse-what-did-you-hear = Ne duydunuz?
morse-check = KONTROL ET
morse-press-new-hear-character = Bir karakteri duymak için YENİ'ye basın.
morse-tone = Ton
morse-speed = Hız
morse-spacing = Aralık
morse-characters-go-out-speed-above = Karakterler yukarıdaki hızda gider; bu, *boşlukların* gönderildiği hızdır. Böylece yeni başlayan biri harfi bu hızda gönderebilmeden önce bütün olarak duyar.
morse-playback-available-desktop-build = Oynatma masaüstü sürümünde kullanılabilir.

## ADS-B paneli: uçak ayrıntı satırları.

adsb-row-altitude = irtifa
adsb-row-track = rota
adsb-row-turning = dönüşte
adsb-row-squawk = squawk
adsb-row-range = menzil
adsb-row-position = konum
adsb-row-signal = sinyal
adsb-row-frames = çerçeveler
rade-transmitting = yayında
decodes-odd = TEK
decodes-new = YENİ
js8-new = YENİ

## AIS paneli: gemi ayrıntı satırları.

ais-row-status = durum
ais-row-speed = hız
ais-row-course = rota
ais-row-heading = baş yönü
ais-row-turning = dönüşte
ais-row-altitude = irtifa
ais-row-size = boyut
ais-row-draught = su çekimi
ais-row-destination = varış
ais-row-range = menzil
ais-row-signal = sinyal
ais-row-type = tür
ais-row-position = konum
ais-row-messages = iletiler
ism-off = KAPALI
textmodem-frequency-shifted = frekans kaydırmalı
winlink-to-heading = KİME

## adsbmap: değerli iletiler (src/adsb_map.rs)

adsbmap-ft-min =
    {""}
    { $vr } ft/dk

## aismap: değerli iletiler (src/ais_map.rs)

aismap-heard-no-position-newer-than = { $heard } duyuldu — { $drop_map_s } sn'den daha yeni bir konum yok
aismap-stations-heard-waiting-position-report = { $heard } istasyon duyuldu — bir konum raporu bekleniyor
aismap-km-across = { $lon_span }° · genişliği ~{ $km } km

## alerts: değerli iletiler (src/app/alerts.rs)

alerts-alarming = { $device } üzerinde alarm çalıyor
alerts-alarm-device-unavailable = alarm aygıtı kullanılamıyor: { $error }

## awards: değerli iletiler (src/app/awards.rs)

awards-worked = { $w } çalışıldı
awards-confirmed = { $value } onaylandı

## bands: değerli iletiler (src/app/bands.rs)

bands-forecast-hamqsl-com-old = · HAMQSL.com tahmini, { $value } eski
bands-ncdxf-iaru-next-beacon-s = NCDXF/IARU · sonraki işaret { $now } sn içinde
bands-hearing-means-path-open-each = { $callsign }, { $location } — onu duymak, { $band } yolunun açık olduğu anlamına gelir. Her işaret 10 sn'de bir bir üst banda geçer; bu yüzden bu satır her yuvada değişir.
bands-radiants-placed = { $lat }°, { $lon }° için radyantlar yerleştirildi
bands-zhr = ZHR { $zhr }
bands-radiant = radyant { $alt_deg }° { $az_deg }
bands-radiant-down = radyant aşağıda ({ $alt_deg }°)
bands-km-s-parent-peak-rate =
    { $value } ({ $code }) — { $velocity_kms } km/sn, ana cisim { $parent }. { $peak } civarında en yüksek oran ZHR { $zhr }.

    Ufkun üstündeki bir radyant, izlerin size ulaşabileceği anlamına gelir; hızlı bir yağmur, 6 m ve 2 m'de meteor saçılması için ve kısa 10 m açılımları için daha uzun ömürlü iyonize izler bırakır.

## drm: değerli iletiler (src/app/drm.rs)

drm-not-decoding = { $channel } çözülmüyor
drm-symbols = { $value } sembol · { $frame }
drm-not-decodable = { $value } — çözülemiyor
drm-broadcaster-s-clock-utc = yayıncının saati { $year }-{ $month }-{ $day } { $hour }:{ $minute } UTC

## duopower: değerli iletiler (src/app/duo_power.rs)

duopower-max-fdm-duo-s-full =
    MAX: FDM-DUO'nun tam çıkışı, yaklaşık 8 W. Hattaki HR500 ile kullanılacak ayar.

    { $base }
duopower-t-5-w-hr500-line-will =
    5 W. Hattaki HR500 ile bu ayarda çoğu bantta tam çıkışa ulaşmaz: MAX kullanın ve düşürmek için Sürüş'ü.

    { $base }
duopower-w-running-barefoot-qrp =
    { $code } W, amplifikatörsüz ya da QRP çalışmak için.

    { $base }

## frame: değerli iletiler (src/app/frame.rs)

frame-re-enables-transmit-after-1 = { $swr }:1'den sonra yayını yeniden etkinleştirir. Önce anteni denetleyin: arıza hâlâ varsa, bir sonraki yayın da durur.
frame-sdroxide-has-been-released = SDRoxide { $version } yayımlandı — bu { $sdroxide_version }.
frame-reconnecting-s = { $value } sn içinde yeniden bağlanılacak…

## gridtracker: değerli iletiler (src/app/grid_tracker.rs)

gridtracker-worked = { $worked } çalışıldı
gridtracker-confirmed = { $confirmed } onaylandı
gridtracker-heard = { $heard } duyuldu

## hd: değerli iletiler (src/app/hd.rs)

hd-decode = Çöz { $value } ({ $codec_label })

## hfdl: değerli iletiler (src/app/hfdl.rs)

hfdl-khz-already-has-chip = { $cur_khz } kHz zaten bir çipe sahip
hfdl-save-current-channel-khz-one = Geçerli kanalı ({ $cur_khz } kHz) kendi çiplerinizden biri olarak kaydedin
hfdl-aircraft = { $aircraft } uçak
hfdl-bursts = { $bursts } patlama
hfdl-decodes-2 = { $decodes } çözüm
hfdl-fixed = { $fec } düzeltildi

## ism: değerli iletiler (src/app/ism.rs)

ism-not-decoded-yet = { $hint } — henüz çözülmedi
ism-listen-tunes-radio-there-one = { $value } üzerinde dinle — radyoyu oraya ayarlar. Bir seferde bir bant: aralarındaki mesafe, hiçbir alıcının ikisini birden kapsamasına izin vermeyecek kadar büyük.
ism-watch-hz-around-band-centre = Bandın merkezi etrafında { $value }Hz izle — bandın ne isteyeceği ne olursa olsun. Daha dar, bandın kendi genişliğini sunamayan bir alıcıya uyar; daha geniş, merkezden daha uzakta duran aygıtlara ulaşır. Alıcınızın bundan yaklaşık üçte bir fazlasını vermesi gerekir.
ism-rtl-433 = rtl_433 — { $why }
ism-rtl-433-khz = rtl_433 { $value } · { $khz } kHz
ism-your-own = { $flex } sizinki
ism-decoders-decoded = { $decoders } çözücü ({ $flex }), { $decodes } çözüldü
ism-handled-rtl-433 = rtl_433 tarafından yönetilen: { $names }
ism-flex = esnek: { $error }
ism-window-mhz-mhz-wide = pencere { $mhz } MHz, { $e62 } MHz genişliğinde
ism-bursts-decoded = { $bursts } patlama, { $decodes } çözüldü
ism-devices = { $ism_reports } aygıt
ism-id-mhz-db-heard-first =
    { $protocol } kimlik { $device }
    { $e6 } MHz { $snr_db } dB duyuldu { $value }×
    ilk duyulma { $first_at } önce

## logbook: değerli iletiler (src/app/logbook.rs)

logbook-confirmations-downloaded-newly-confirmed = Onaylar: { $recs } indirildi, { $matched } yeni onaylandı
logbook-qso = { $qso_log } QSO
logbook-worked-before = ⚠ DAHA ÖNCE ÇALIŞILDI ({ $dupe_band })
logbook-utc-qso = { $oldest }–{ $newest } UTC · { $value } QSO

## app: değerli iletiler (src/app/mod.rs)

app-radio = radyo { $id }

## morse: değerli iletiler (src/app/morse.rs)

morse-could-not-start-morse-audio = Morse ses iş parçacığı başlatılamadı: { $error }
morse-no-audio-output = ses çıkışı yok: { $error }
morse-about-s = yaklaşık { $farnsworth_wpm } sn
morse-characters-play = Oynayan karakterler: { $set }
morse-score-run = Puan: { $correct } / { $total }. Seri: { $streak } / { $value }.
morse-correct = doğru — { $value }
morse-was = bu { $value } idi

## net: değerli iletiler (src/app/net.rs)

net-synced-z-new-already-log = { $when }Z eşitlendi: { $added } yeni, { $matched } zaten günlükte
net-wavelog-2 = Wavelog: { $line }
net-full-resync-stopped-nothing-changed = Tam yeniden eşitleme durdu, hiçbir şey değişmedi: { $error }
net-full-resync-applied-added-updated = Tam yeniden eşitleme uygulandı: { $add } eklendi, { $update } güncellendi, { $remove } kaldırıldı, { $mark } sizinkinden Wavelog'da olarak işaretlendi
net-wavelog-3 = Wavelog: { $line }
net-adif-import-failed = ADIF içe aktarma başarısız: { $error }
net-not-unicode-read = {" "}(Unicode değil; { $enc } olarak okundu)
net-received-reports-swl-left-out = , { $n } alınan rapor (SWL) dışarıda bırakıldı
net-adif-import-added-duplicates-skipped = ADIF içe aktarma: { $added } eklendi, { $skipped } yineleme atlandı{ $reports }{ $assumed }
net-channel-import-failed = Kanal içe aktarma başarısız: { $error }
net-line-s-skipped = , { $n } satır atlandı
net-not-unicode-read-2 = {" "}(Unicode değil; { $enc } olarak okundu)
net-channel-import-channel-s-read = Kanal içe aktarma: { $n } kanal okundu{ $skipped }{ $assumed }

## acars: değerli iletiler (src/app/panels/acars.rs)

acars-frames-bad = { $frames } çerçeve · { $bad } hatalı
acars-mode = mod { $mode }

## adsb: değerli iletiler (src/app/panels/adsb.rs)

adsb-aircraft-3 = { $aircraft } uçak
adsb-frames = { $frames } çerçeve
adsb-preambles = { $preambles } ön ek
adsb-bad-crc = { $bad_crc } hatalı CRC
adsb-tune = AYAR { $mhz }
adsb-ft-barometric = { $ft } ft barometrik

## ais: değerli iletiler (src/app/panels/ais.rs)

ais-vessels-3 = { $vessels } gemi
ais-messages = { $messages } ileti
ais-slots = { $bursts } yuva
ais-bad-fcs = { $bad_fcs } hatalı FCS
ais-ais-mhz = AIS { $value } ({ $mhz } MHz) — { $value2 }
ais-ais-mhz-slots-messages-floor = AIS { $value } ({ $mhz } MHz) — { $bursts } yuva, { $messages } ileti, taban { $floor_dbfs } dBFS
ais-ais = AIS { $value }
ais-tune = AYAR { $mhz }
ais-ships-heard-are-khz-off = Duyulan gemiler frekanstan { $khz } kHz kaymış ({ $mhz } ppm, 162 MHz'de) — ön ucun frekans düzeltmesini ayarlayın, yoksa yaklaşık 5 kHz'nin ötesindeki yayınların çözümü durur
ais-over-ground = { $value }° yer üstünde
ais-dbfs-db-snr = { $rssi_dbfs } dBFS, { $snr_db } dB SNR
ais-last-type-ais = { $messages } (son tür { $last_type }, AIS { $channel })
ais-ais-2 = AIS { $ch }
ais-mhz-marine-channel-ship-alternates = { $mhz } MHz — deniz kanalı { $ch }. Bir gemi ikisi arasında gider gelir; birini kapatmak, her geminin ne sıklıkla duyulduğunu yarıya indirir.

## aprs: değerli iletiler (src/app/panels/aprs.rs)

aprs-object = { $reported_by } tarafından bildirilen nesne
aprs-via = { $via } üzerinden
aprs-object-reported = nesne { $reported_by } tarafından bildirildi
aprs-digit-s-blanked = ± { $ambiguity } rakam boş bırakıldı
aprs-wind-m-s = rüzgâr { $value }° { $value2 } m/sn
aprs-gust = esinti { $value }
aprs-rain-mm-h = yağış { $value } mm/sa
aprs-frame-s-last-ago = { $packets } çerçeve, son { $last_heard } önce
aprs-via-2 = { $via } üzerinden
aprs-bad = { $bad_frames } hatalı
aprs-other = { $non_aprs } diğer
aprs-sent = gönderildi ×{ $tries }
aprs-stn = { $stations } ist
aprs-queued = { $tx_queue } sırada

## atchat: değerli iletiler (src/app/panels/atchat.rs)

atchat-master = ana { $value }
atchat-last-heard-s-ago-click = { $status } — son duyulma { $age_s } sn önce · doğrudan ileti sekmesi açmak için tıklayın
atchat-close-tab = { $peer } sekmesini kapat
atchat-direct-message = { $value } ile doğrudan ileti
atchat-cannot-display = { $path } görüntülenemiyor

## cw: değerli iletiler (src/app/panels/cw.rs)

cw-click-decoder-neural-one-copies =
    { $hint }

    { $neural } çözücü için tıklayın. Sinir çözücü daha aşağıda kopyalar ve bir zamanlama uyumunun kabul etmeyeceği elle gönderilmiş CW'yi okur; zamanlama çözücüsü ise vurgulu harfleri — Ä, Ö, Å, Ü, É — kopyalayan tek çözücüdür, çünkü model bunlar için bir çıkış sınıfına sahip değildir.
cw-idle-s = BOŞTA { $idle } sn

## decodes: değerli iletiler (src/app/panels/decodes.rs)

decodes-save-decodes-list-adif-received = Listedeki { $n } çözümlemeyi ADIF olarak kaydedin — alınmış raporlar, temaslar değil. Gönderen adını vermeyen çözümlemeler dışarıda bırakılır.
decodes-save-decodes-list-csv-one = Listedeki { $n } çözümlemeyi CSV olarak kaydedin, her biri bir satır
decodes-session-qso = Oturum: { $session } QSO
decodes-save-whole-logbook-qso-adif = Tüm günlüğü ({ $logged } QSO) ADIF olarak kaydedin
decodes-save-whole-logbook-qso-text = Tüm günlüğü ({ $logged } QSO) metin olarak kaydedin
decodes-hz-slots = { $audio_hz } Hz · { $odd } yuva
decodes-your-slot-timing-against-stations =
    Duyduğunuz istasyonlara karşı yuva zamanlamanız.
    { $after }

    Bu, tüm alma yolunu kapsar; bu yüzden yavaş bir ses ya da ağ zinciri, yanlış bir saat gibi sayılır. 0,5 sn'nin altı rahattır.
decodes-pile-up = YIĞIN { $fox_queue }
decodes-queue = KUYRUK { $call_queue }
decodes-db-hz-click-remove =
    { $waiting } · { $snr_db } dB · { $audio_hz } Hz{ $unwrap_or_default }
    Kaldırmak için tıklayın

## js8: değerli iletiler (src/app/panels/js8.rs)

js8-every-speed-being-decoded-marks = Her hız çözülüyor; ▸ yayın yaptığınız { $speed } hızını işaretler
js8-every-speed-being-decoded-click = Her hız çözülüyor. Bunun yerine { $speed } hızında yayın yapmak için tıklayın
js8-transmit-decode = { $speed } hızında yayınla ve çöz
js8-beaconing-every-min-click-stop = Her { $hb_min } dakikada bir işaret veriliyor — durdurmak için tıklayın
js8-last-beacon-went-out-hz = Son işaret { $hz } Hz'de gitti — { $value }–{ $value2 } Hz kalp atışı alt bandında boş bir yuva; çözülen sinyallerin dışına düşmesi için seçildi.
js8-frames-s = { $tx_frames_pending }/{ $tx_frames_total } çerçeve · { $left } sn
js8-decoded = { $speed } hızında çözüldü
js8-frames = … ({ $frames } çerçeve)
js8-reply-2 = { $value } kişisine yanıt: “{ $value2 }”
js8-address-composer = Yazı alanını { $value } için adresleyin

## macros: değerli iletiler (src/app/panels/macros.rs)

macros-f-sends = F{ $i }: “{ $text }” gönderir
macros-sends = “{ $text }” gönderir
macros-most = { $value } en fazlası

## navtex: değerli iletiler (src/app/panels/navtex.rs)

navtex-repaired-lost = { $repaired } onarıldı · { $lost } kayıp
navtex-characters-lost = { $lost } karakter kayıp

## packet: değerli iletiler (src/app/panels/packet.rs)

packet-bad = { $bad_frames } hatalı
packet-link-layer = Bağlantı katmanı { $link } içinde
packet-via = {" "}üzerinden { $via }
packet-unacked = { $unacked } onaysız
packet-retry = yeniden deneme { $retries }

## pi4: değerli iletiler (src/app/panels/pi4.rs)

pi4-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] işaret
       *[other] işaret
    }
pi4-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } önce

## rade: değerli iletiler (src/app/panels/rade.rs)

rade-snr-db = SNR { $snr_db } dB
rade-offset-hz = kayma { $freq_offset_hz } Hz
rade-heard = duyulan { $call }
rade-samples-dropped-between-receiver-decoder = ⚠ Alıcı ile çözücü arasında { $dropped } örnek düşürüldü — bu makine sinir çözümlemesine yetişemiyor.

## textmodem: değerli iletiler (src/app/panels/text_modem.rs)

textmodem-char-s = { $chars_per_sec } karakter/sn
textmodem-char-s-hz-wide = { $value } — { $chars_per_sec } karakter/sn, { $bandwidth_hz } Hz genişliğinde, { $keyed }

## vdl2: değerli iletiler (src/app/panels/vdl2.rs)

vdl2-frames = { $frames } çerçeve
vdl2-bursts = { $bursts } patlama
vdl2-sync = { $syncs } senkron
vdl2-hdlc-bad = { $hdlc_bad } HDLC hatalı
vdl2-bad-fcs = { $fcs_bad } hatalı FCS
vdl2-mhz-bursts-frames-noise-floor = { $mhz } MHz, { $what } — { $bursts } patlama, { $frames } çerçeve, gürültü tabanı { $floor_dbfs } dBFS
vdl2-tune = AYAR { $mhz }
vdl2-held-2 = TUTULDU +{ $since }
vdl2-use-worldwide-where-every-link = { $what } — dünya çapında kullanılır ve her bağlantının başladığı yerdir. Yalnızca birini tutacaksanız tutulacak olan budur.
vdl2-assigned-which-channels-carry-anything = Bir { $what } olarak atanmış. Hangi kanalların bir şey taşıdığı bulunduğunuz yere bağlıdır: aksini bilmiyorsanız hepsini açık bırakın.
vdl2-mhz-db-snr-dbfs-evm = { $mhz } MHz · { $snr_db } dB SNR · { $rssi_dbfs } dBFS · EVM { $evm_deg }° · { $freq_err_hz } Hz · { $rs_corrected } RS sembolü düzeltildi
vdl2-characters = { $parity_errors } karakter
vdl2-parameter-sdroxide-does-not-read = { $unknown } { $unknown ->
        [one] parametre
       *[other] parametre
    } SDRoxide okumaz — yukarıda tanımlayıcı ve baytlar olarak gösterilir

## wefax: değerli iletiler (src/app/panels/wefax.rs)

wefax-lines = { $lines } satır
wefax-lines-held = { $live_size } satır tutuluyor
wefax-pixels-per-line = satır başına { $width } piksel
wefax-shift-picture-pixels = Resmi { $px } piksel kaydır
wefax-charts-are-saved-click-copy =
    Grafikler { $place } kaydedilir

    Yolu kopyalamak için tıklayın
wefax-finished-charts-are-saved-collect = Bitmiş grafikler { $place } kaydedilir ve burada toplanır.
wefax-pixels-right-click-delete =
    { $when_full }
    { $size } × { $size2 } piksel
    { $value }

    Silmek için sağ tıklayın
wefax-older-store = Depoda { $older } daha eski
wefax-older-load-more = { $older } daha eski — daha fazlasını yükle
wefax-published-carrier-khz-dial-khz = Yayımlanmış taşıyıcı { $f } kHz → USB skalası { $value } kHz
wefax-delete-chart-2 = Bu grafiği sil { $dir }

## widgets: değerli iletiler (src/app/panels/widgets.rs)

widgets-cq-zone-itu-zone = { $continent } · CQ bölgesi { $cq_zone } · ITU bölgesi { $itu_zone }
widgets-new-entity = { $band_label } üzerinde yeni varlık
widgets-worked-before = { $band_label } üzerinde daha önce çalışıldı
widgets-calling-cq-includes-you = CQ { $target } çağrısı — bu sizi de kapsıyor
widgets-calling-cq-not-aimed-you = CQ { $target } çağrısı — size yönelik değil

## wspr: değerli iletiler (src/app/panels/wspr.rs)

wspr-rx-heard-us = { $heard_us } rx · { $heard_us2 } bizi duydu
wspr-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] işaret
       *[other] işaret
    }
wspr-beaconing-duty = işaret veriliyor — { $wspr_power_dbm }, { $wspr_tx_percent }% görev
wspr-listening-duty = dinleniyor ({ $wspr_tx_percent }% görev)
wspr-beacon-one-two-minute-slot = Her { $pct } içinden bir iki dakikalık yuvada işaret — bunların { $pct2 }%'i, eşit aralıklı — { $wspr_power_dbm} gücünde. Döngünün hangi yuvada başlayacağı çağrı işaretinizden çekilir; bu yüzden bu programı çalıştıran iki istasyon birlikte başlamaz.
wspr-transmitting-set-general-tab-settings = { $sent } içinde { $call } olarak yayın yapılıyor — Ayarlar'ın Genel sekmesinde ayarlanır.
wspr-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } önce

## publicsdr: değerli iletiler (src/app/publicsdr.rs)

publicsdr-antenna =
    { $address }
    { $device }
    anten: { $antenna }
    { $e3 }
publicsdr-receivers-fetched-ago = { $entries } alıcı · { $per_net } · { $age } önce alındı
publicsdr-showing-search-narrow = { $n } içinden { $shown } gösteriliyor — daraltmak için arayın
publicsdr-match = { $n } eşleşme
publicsdr-replace-taking-receiver-points-same =
    { $this } yerine { $entry } koyulsun mu?

    { $this2 }, { $iface } üzerinde. Bu alıcıyı almak, aynı sekmeyi bunun yerine { $address } adresine yöneltir, onu “{ $entry2 }” olarak yeniden adlandırır ve skalayı alıcının yayımladığı { $range_label } aralığında tutar — burada hiçbir şey yayın yapmaz.

    { $iface2 } ayarları olduğu yerde kalır; onlar için belirtilen aralıklar da öyle: arayüzü Ayarlar → Radyo'da geri değiştirmek, radyoyu eski haline döndürür.
publicsdr-receptions-will-reported = {" "}Alımlar { $value } konumundan bildirilecek.
publicsdr-pointing-radio = Bu radyo { $entry } alıcısına yöneltiliyor…{ $site }
publicsdr-opening-another-radio = { $entry } başka bir radyo olarak açılıyor…

## qo100: değerli iletiler (src/app/qo100.rs)

qo100-khz-3 = +{ $park_lo_hz } … +{ $park_hi_hz } kHz
qo100-before-switching-tune-so-beacon = AÇMADAN ÖNCE: işaretin iki lobu şeritte +{ $park_lo_hz} ile +{ $park_hi_hz} kHz arasında kalacak şekilde ayarlayın (gölgeli şerit).
qo100-receiver-currently-listening-around-mhz = Alıcı şu anda { $mhz } MHz çevresini dinliyor — 10489,750 MHz işaretinden çok uzakta, bu yüzden burada çizilecek bir şey yok. Çözücü yine de aramaya devam eder.
qo100-receiver-currently-covers-only-khz = alıcı burada yalnızca ±{ $khz } kHz kapsıyor — şeridin geri kalanı boş kalır
qo100-null-db-sym-snr-db = boşluk { $est_null_depth_db } dB sim { $est_symmetry } snr { $est_snr_db } dB
qo100-hz-s-hz-s-de = { $est_drift_hz_s } Hz/sn { $est_drift_accel_hz_s2 } Hz/sn² (çözümden önce düzeltildi)
qo100-hz-s-de-rotated-before = { $est_drift_hz_s } Hz/sn (çözümden önce düzeltildi)
qo100-found-empty = { $est_updates } bulundu, { $est_misses } boş
qo100-last-s-ago = {"   "}son { $auto_last_hz } ({ $auto_last_unix } sn önce)
qo100-over-correction = { $auto_total_hz } toplam, { $auto_applies } { $auto_applies ->
        [one] düzeltme
       *[other] düzeltme
    }{ $last }
qo100-sync-32-err = senkron { $sync_bit_errors } / 32 hata ×{ $sync_matches }
qo100-frame-buffer = çerçeve arabelleği %{ $frame_fill }
qo100-blocks-tried-decoded = { $blocks_tried } blok denendi, { $blocks_locked } çözüldü
qo100-last-applied-s-ago-hz = son uygulama { $ago } sn önce: { $old } → { $new } Hz ({ $old2 } Hz)
qo100-locked-block-tried-locked = kilitli — { $blocks_tried } { $blocks_tried ->
        [one] blok
       *[other] blok
    } denendi, { $blocks_locked } kilitlendi
qo100-searching-block-tried-none-locked = aranıyor — { $blocks_tried } { $blocks_tried ->
        [one] blok
       *[other] blok
    } denendi, henüz hiçbiri kilitlenmedi

## rds: değerli iletiler (src/app/rds.rs)

rds-auto = Otomatik ({ $standard })
rds-type = tür { $class }
rds-groups = · { $groups } grup
rds-blocks-error = · blokların %{ $ber }'i hatalı
rds-clean-corrected-lost = { $blocks_ok } temiz · { $blocks_corrected } düzeltildi · { $blocks_bad } kayıp

## sat: değerli iletiler (src/app/sat.rs)

sat-utc-local = { $unix } UTC (yerel { $offset })
sat-aos = AOS { $rise_unix } { $rise_unix2 }
sat-los = LOS { $set_unix } { $set_unix2 }
sat-above-horizon = { $rise_unix } ufkun üstünde
sat-min = { $value } dk içinde
sat-h-min = { $value } sa { $value2 } dk içinde
sat-searching = aranıyor… { $entries } içinden { $next }
sat-locked-2 = ● KİLİTLİ — { $t }
sat-pass-until-max = { $set_unix }'e kadar geçiş · en yüksek { $max_el }°
sat-next-pass-rises-max = Sonraki geçiş { $rise_unix } { $rise_unix2 } ({ $now }) · { $rise_az }° { $rise_az2 } yönünde yükselir · en yüksek { $max_el }°
sat-rigctld-client-connected-satellite-software = ⚠ { $clients } rigctld { $clients ->
        [one] istemcisi
       *[other] istemcisi
    } bağlı — oradaki skalayı yöneten uydu yazılımı Doppler'i iki kez düzeltir
sat-rotator-antenna-az-el = Döndürücü: anten { $az }° az { $el }° el
sat-rotator-2 = Döndürücü: { $error }
sat-up-now-2 = ● şu an yukarıda, { $el }°
sat-inv = { $mode } · ters
sat-norad-2 = NORAD { $norad_id }

## scanner: değerli iletiler (src/app/scanner.rs)

scanner-holding-mhz = { $here } MHz'de tutuluyor
scanner-scanning-mhz = taranıyor · { $here } MHz

## spectrum: değerli iletiler (src/app/spectrum.rs)

spectrum-renderer-will-not-hold-texture = Bu çizici, { $max_texture_dim } pikselden geniş bir doku tutmaz.
spectrum-mem = Hafıza: { $f } / { $value }
spectrum-mem-2 = Hafıza: { $value }

## speech: değerli iletiler (src/app/speech.rs)

speech-voice-2 = Ses: { $voice }
speech-speech-unavailable = Konuşma kullanılamıyor: { $error }

## spots: değerli iletiler (src/app/spots.rs)

spots-transmissions = { $stations } yayın

## winlink: değerli iletiler (src/app/winlink.rs)

winlink-last-session-failed = son oturum başarısız — { $err }
winlink-last-session-received-sent = son oturum: { $last_received } alındı, { $last_sent } gönderildi
winlink-empty = { $to_lowercase } boş
winlink-showing = { $total } içinden { $entries } gösteriliyor
winlink-fetching = { $mid } alınıyor…
winlink-from-2 = { $msg } kimden { $date }
winlink-attachment-bytes = ek: { $att } ({ $data } bayt)

## aprsmap: değerli iletiler (src/aprs_map.rs)

aprsmap-object =
    {""}
    { $reported_by } tarafından bildirilen nesne
aprsmap-heard-ago =
    {""}
    { $age } önce duyuldu

## download: değerli iletiler (src/download.rs)

download-reading = { $path } okunuyor: { $error }

## help: değerli iletiler (src/help.rs)

help-missing-image = {"["}eksik görüntü: { $path }]

## hfdlmap: değerli iletiler (src/hfdl_map.rs)

hfdlmap-mhz =
    {""}
    { $e3 } MHz
hfdlmap-fix =
    {""}
    { $fixes } konum{ $es }
hfdlmap-heard-ago =
    {""}
    { $last_at } önce duyuldu

## multi: değerli iletiler (src/multi.rs)

multi-radio = Radyo { $n }
multi-could-not-add-radio = Radyo eklenemedi: { $error }
multi-was-closed-station = { $value } istasyonda kapatıldı.
multi-could-not-open = { $peer } açılamadı: { $error }
multi-has-tab-own-now = { $value } artık kendi sekmesine sahip.

## remote: değerli iletiler (src/remote.rs)

remote-protocol-error = protokol hatası: { $error }

## solarnet: değerli iletiler (src/solar3d/net.rs)

solarnet-bad-message = hatalı ileti: { $error }
solarnet-solar-protocol-mismatch-server-client = güneş protokolü uyuşmuyor: sunucu { $proto }, istemci { $value }

## smeter: değerli iletiler (src/widgets/smeter.rs)

smeter-forward-power-w-measured-radio =
    {""}

    İleri güç — { $w } W, radyonun kendi sensörüyle ölçüldü.
smeter-po-radio-s-own-power =
    {""}

    PO — radyonun kendi çıkış gücü metresi, tam ölçeğinin %{ $clamp }'inde. Radyonun bildirdiği gibi bir iğne konumu, kalibre edilmiş bir watt değeri değil; izlenecek şey, Sürüş yerinde dururken bunun düşüp düşmediğidir.
smeter-swr-1-match-antenna-presenting =
    {""}

    SWR — { $swr }:1, antenin sunduğu uyum, radyonun ölçtüğü gibi. 2:1'in altı olağandır; 3:1'in üstünde ölçek kırmızıya döner.
smeter-received-signal-strength-s9-db = Alınan sinyal gücü: S9 + { $over } dB (geçiş bandında { $s_dbm } dBm).
smeter-received-signal-strength-s-dbm = Alınan sinyal gücü: S{ $value } (geçiş bandında { $s_dbm } dBm).
smeter-radio-reports-c =
    {""}

    Radyo { $t } °C bildiriyor.
smeter-ps-puresignal-correcting-db-compression =
    {""}

    PS — PureSignal, { $correction_db } dB sıkıştırmayı düzeltiyor{ $held } (geri besleme, yayınla { $score } eşleşti).
smeter-ps-puresignal-switched-but-has =
    {""}

    PS −− — PureSignal açık ama geri besleme yolunda yayını bulamadı (en iyi eşleşme { $score }), bu yüzden verici düzeltilmemiş çıkıyor. Amplifikatör çıkışından bir örneğin, T/R anahtarının yayında almadığı bir alma girişine gerçekten ulaştığını ve zayıflatıcının onu yutmadığını denetleyin. Döngü yalnızca bu alıcıyı dinler — ikinci bir alıcıya (RX2, ADC2) bağlanmış bir kuplaj hiçbir zaman geri besleme olarak okunmaz.

## specview: değerli iletiler (src/widgets/spectrum_view.rs)

specview-rit-hz = RIT { $hz } Hz
specview-xit-hz = XIT { $hz } Hz

## decodes: iç içe iletiler

decodes-you-transmit-s-everyone-else = { $off } sn { $after } yayın yapıyorsunuz — çağrıların yanıtsız kalmasının olağan nedeni. Bilgisayarınızın saatini eşitleyin.

## publicsdr: iç içe iletiler

publicsdr-noise-floor-score = gürültü tabanı puanı { $snr }
publicsdr-up-khz-i-q = { $khz } kHz I/Q'ya kadar

## txaudio: daha fazla ileti

txaudio-gr-gate-de-ess-comp = GR kapı { $gate_gr_db } de-ess { $deess_gr_db } komp { $comp_gr_db } sınır { $limiter_gr_db }

## amp: daha fazla ileti

amp-h-step = ~{ $l_step } µH · adım { $l_step2 }
amp-pf-step = ~{ $c_step } pF · adım { $c_step2 }

## sstv: daha fazla ileti

sstv-bit-octets-chunks-first-pass = { $from } · { $filename } · { $width }×{ $height } { $bits_per_pixel } bit · { $media_type } / { $content_encoding } · { $chunk_count } parçada { $encoded_size } oktet ({ $chunks_first_pass } ilk geçiş) · oturum { $session }
sstv-baud-cpfsk-hz-khz-occupied = { $value } — { $symbol_rate } baud CPFSK, ±{ $deviation_hz } Hz, { $bandwidth_hz } kHz kaplanan bant genişliği

## Saat ve değerli birkaç kısa etiket.

time-utc-and-local = { $utc } UTC · { $local } yerel
sat-in-seconds = { $s } sn içinde
winlink-to-list = kime { $to }
winlink-cc-list = bilgi { $cc }
spots-n-of-total = { $total } içinden { $n }
wefax-n-of-total = { $total } içinden { $n }
adsb-range-bearing = { $bearing }° yönünde { $km } km
ais-range-bearing = { $bearing }° yönünde { $km } km

## sdroxide-types değerleri, seçicilerin adlandırdığı biçimde (src/i18n/labels.rs).
rtl-sdr-hf-mode-off = Kapalı (yalnızca tuner, 24 MHz'den yukarı)
rtl-sdr-hf-mode-auto = Otomatik
rtl-sdr-hf-mode-direct-q = Doğrudan örnekleme (Q dalı)
rtl-sdr-agc-manual = Elle (AGC yok)
rtl-sdr-agc-tuner = Tuner AGC
rtl-sdr-agc-rtl = RTL dijital AGC
rtl-sdr-agc-both = Tuner + RTL AGC
mode-control-cat = CAT
mode-control-radio = Radyo denetimli
line-state-none = Yok
line-state-high = Yüksek
line-state-low = Düşük
elad-tx-input-usb-audio = USB ses
elad-tx-input-mic = Mikrofon
elad-tx-input-auto = Otomatik
elad-tx-input-leave = Radyoda ayarlandığı gibi bırak
tr-usdx-audio-one-cable = Tek kablo (CAT akışındaki ses)
tr-usdx-audio-sound-card = USB ses kartı (3,5 mm jak)
spy-server-format-uint8 = 8 bit (en düşük bant genişliği)
spy-server-format-int16 = 16 bit
spy-server-format-float32 = 32 bit kayan nokta
sound-format-iq = IQ (stereo)
sound-format-demod-audio = Demodüle ses
sdr-play-duo-tuner-tuner1 = Tuner 1 (50 Ohm / Hi-Z)
sdr-play-duo-tuner-tuner2 = Tuner 2 (50 Ohm)
sdr-play-duo-role-diversity = İkinci bir anten (çeşitlilik / QRM bastırma)
sdr-play-duo-role-second-radio = Kendi frekansında ikinci bir radyo
sdr-play-agc-off = Kapalı (elle IF kazancı)
sdr-play-agc-hz5 = 5 Hz
sdr-play-agc-hz50 = 50 Hz
sdr-play-agc-hz100 = 100 Hz
rotator-protocol-rotctld = Hamlib rotctld (TCP)
rotator-protocol-pst-rotator = PSTRotator (UDP)
pluto-ptt-off = Kapalı
pluto-ptt-gpo01 = GPO0 = RX, GPO1 = TX
pluto-ptt-gpo23 = GPO2 = RX, GPO3 = TX
pluto-duplex-fdd = FDD (ikisi birden)
pluto-duplex-tdd = TDD (biri biri ardına)
pluto-agc-manual = Elle
pluto-agc-slow-attack = Yavaş atak
pluto-agc-fast-attack = Hızlı atak
pluto-agc-hybrid = Hibrit
parity-none = Yok
parity-even = Çift
parity-odd = Tek
panadapter-tap-antenna = Anten (radyoyla paylaşılır)
panadapter-tap-if-output = Radyonun I.F. çıkışı
panadapter-audio-attached = Bağlı alıcı
panadapter-audio-transceiver = Telsiz
kenwood-send-ts2000 = TS-2000 tarzı (TX;)
kenwood-send-ts590 = TS-590 tarzı (TX1;)
digi-mode-usb = USB
digi-mode-data = DIGI
digi-mode-radio = Radyo denetimli
cw-keying-cat = Telsiz keyer'ı (CAT)
cw-keying-audio = Ses kartı (MCW)
audio-route-both = İki kanal
audio-route-left = Yalnızca sol
audio-route-right = Yalnızca sağ
amp-model-hr500 = Hardrock-500
amp-model-hr50 = Hardrock-50 (test edilmemiş)
alert-sound-ding = Ding
alert-sound-two-tone = İki ton
alert-sound-triplet = Üçlü
alert-sound-warble = Titreme
alert-sound-digital = Dijital
alert-reply-tone = Ton
alert-reply-voice = Ses
alert-reply-both = Ton + ses
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
icom-model-other = Diğer (adresi elle ayarla)
backend-auto = Otomatik algıla (SoapySDR / CAT)
backend-soapy = SoapySDR
backend-cat = CAT / Ses
backend-hpsdr = HPSDR (ağ)
backend-tci = TCI (ağ)
backend-icom-net = Icom LAN (ağ)
backend-smart-sdr = SmartSDR / FlexRadio (ağ)
backend-pluto = PlutoSDR (ağ)
backend-rtl-sdr = RTL-SDR (USB)
backend-rtl-tcp = rtl_tcp üzerinden RTL-SDR (ağ)
backend-spy-server = SpyServer (ağ)
backend-spy-server-vfo = SpyServer VFO+FFT, düşük bant genişliği (ağ)
backend-kiwi-sdr = KiwiSDR / Web-888 (ağ)
backend-rx888 = RX-888 (USB)
backend-airspy-hf = Airspy HF+ (USB)
backend-airspy = Airspy R2 / Mini (USB)
backend-hack-rf = HackRF One / Pro (USB)
backend-sdr-play = SDRplay RSP (USB)
backend-elad = ELAD FDM-DUO / FDM-S (USB)
backend-lime = LimeSDR + LimeRFE (LimeSuite)
backend-hydra-sdr = HydraSDR RFOne (USB)
backend-fobos = RigExpert Fobos SDR (USB)
backend-usb-audio = USB ses radyosu (ses kartı)
backend-none = Yapılandırılmadı
cat-family-xiegu = Xiegu
cat-family-icom = Icom
cat-family-yaesu = Yaesu
cat-family-kenwood = Kenwood
cat-family-elecraft = Elecraft
cat-family-elad = ELAD
cat-family-qrp-labs = QRP Labs
cat-family-rs-hfiq = RS-HFIQ
cat-family-tr-usdx = (tr)uSDX
cat-family-rigctld = Hamlib rigctld (ağ)
cat-family-flrig = flrig (ağ)
publicsdr-its-interface = arayüzü

## sdroxide-types değerleri, çiplerde, menülerde ve listelerde gösterilenler (src/i18n/labels.rs).
agc-mode-off = Kapalı
agc-mode-slow = Yavaş
agc-mode-med = Orta
agc-mode-fast = Hızlı
airspy-gain-linearity = Doğrusallık (güçlü sinyal işlemede en iyi)
airspy-gain-sensitivity = Hassasiyet (zayıf sinyalde en iyi)
ais-kind-class-a = A sınıfı
ais-kind-class-b = B sınıfı
ais-kind-base-station = baz istasyonu
ais-kind-sar-aircraft = SAR uçağı
ais-kind-aid-to-navigation = seyir yardımcısı
ais-kind-sart = SART / MOB / EPIRB
ais-kind-craft = bağlı araç
alert-event-called = Bir istasyon beni çağırıyor
alert-event-cq = Yanıtlayabileceğim bir CQ
alert-event-new-dxcc = Yeni DXCC varlığı
alert-event-new-dxcc-band = Bu bantta yeni varlık
alert-event-new-grid = Yeni lokatör karesi
audio-routing-both = Sol + sağ
audio-routing-left = Yalnızca sol
audio-routing-right = Yalnızca sağ
button-mode-momentary = Basılı tut
button-mode-toggle = Aç / kapat
contest-mode-none = Yok
contest-mode-eu-vhf = EU VHF Yarışması
converter-tx-off = Dönüştürürken kapalı
converter-tx-transverter = Aynı dönüştürücü üzerinden
converter-tx-own = Kendi kayması
cw-engine-neural = SİNİR
cw-engine-timing = ZAMANLAMA
decode-sort-none = Yok
decode-sort-signal = SNR
decode-sort-distance = Uzaklık
decode-sort-country = Ülke
diversity-mode-cancel = İptal — bir gürültü kaynağını sıfırla
diversity-mode-combine = Birleştir — çeşitlilik alımı
dxped-mode-normal = Normal
dxped-mode-hound = Hound
dxped-mode-fox = Fox
fail-safe-refuse-tx = Yayını reddet
fail-safe-warn-only = Yine de yayınla, uyar
ftp-security-plain = FTP (şifreleme yok)
ftp-security-explicit-tls = FTPS açık (AUTH TLS)
ftp-security-implicit-tls = FTPS örtük
hpsdr-filter-board-none = Yok — çıkışlar kapalı kalır
hpsdr-filter-board-n2adr = N2ADR filtre kartı
hpsdr-filter-board-alex = Alex / Hermes bant kodu (Zeus, HiQSDR, Quisk)
hpsdr-filter-board-custom = Özel — aşağıdaki kendi tablom
hpsdr-io-rx-input-radio = Radyonun kendi girişi
hpsdr-io-rx-input-io-board = IO kartı J9
hpsdr-io-rx-input-io-board-pure-signal = IO kartı J9, yayında PureSignal
hydra-sdr-gain-linearity = Doğrusallık (güçlü sinyal işlemede en iyi)
hydra-sdr-gain-sensitivity = Hassasiyet (zayıf sinyalde en iyi)
icom-rx-source-af = AF — radyo demodüle eder
icom-rx-source-if12k = 12 kHz IF — sdroxide demodüle eder
icom-scope-span-radio = Radyoda ayarlandığı gibi
icom-scope-span-khz2-5 = 5 kHz (±2,5k)
icom-scope-span-khz5 = 10 kHz (±5k)
icom-scope-span-khz10 = 20 kHz (±10k)
icom-scope-span-khz25 = 50 kHz (±25k)
icom-scope-span-khz50 = 100 kHz (±50k)
icom-scope-span-khz100 = 200 kHz (±100k)
icom-scope-span-khz250 = 500 kHz (±250k)
icom-scope-span-khz500 = 1 MHz (±500k)
ism-family-weather = HAVA
ism-family-meter = SAYAÇ
ism-family-home-auto = EV
ism-family-lo-ra = LORA
ism-family-unidentified = BİLİNMİYOR
ism-protocol-la-crosse-it = LaCrosse IT+
ism-protocol-fine-offset = Fine Offset
ism-protocol-bresser = Bresser
ism-protocol-z-wave = Z-Wave
ism-protocol-homematic = Homematic
ism-protocol-rtl433 = rtl_433
ism-protocol-unidentified = tanımlanmamış
js8-speed-normal = NORMAL
js8-speed-fast = HIZLI
js8-speed-turbo = TURBO
js8-speed-slow = YAVAŞ
lime-aux-role-off = Kullanılmıyor
lime-aux-role-diversity = İkinci bir anten (çeşitlilik / QRM bastırma)
lime-aux-role-pure-signal = Yayın geri beslemesi (PureSignal ön bozma)
lookup-provider-none = Kapalı
lookup-provider-qrz = QRZ.com
lookup-provider-ham-qth = HamQTH
mail-folder-inbox = GELEN KUTUSU
mail-folder-outbox = GİDEN KUTUSU
mail-folder-sent = GÖNDERİLEN
mail-folder-archive = ARŞİV
memory-sort-name = Ad
memory-sort-freq = Frek
memory-sort-band = Bant
memory-sort-stored = Kaydedilen
mouse-button-middle = Orta
mouse-button-extra1 = Geri (ekstra 1)
mouse-button-extra2 = İleri (ekstra 2)
nr-level-off = Kapalı
nr-level-low = Düşük
nr-level-medium = Orta
nr-level-high = Yüksek
nr-level-rnn-low = RNN Düşük
nr-level-rnn-med = RNN Orta
nr-level-rnn-high = RNN Yüksek
nr-level-spec-low = SPEC Düşük
nr-level-spec-med = SPEC Orta
nr-level-spec-high = SPEC Yüksek
nr-level-nr2-low = NR2 Düşük
nr-level-nr2-med = NR2 Orta
nr-level-nr2-high = NR2 Yüksek
nr-level-df-low = DFNR Düşük
nr-level-df-med = DFNR Orta
nr-level-df-high = DFNR Yüksek
nr-strength-low = Düşük
nr-strength-med = Orta
nr-strength-high = Yüksek
orbit-rings-none = yok
orbit-rings-curated = küratörlü
orbit-rings-all = tümü
qso-step-idle = Boşta
qso-step-wait-cq = CQ bekle
qso-step-calling-cq = CQ çağrısı
qso-step-tx-grid = Yayın Lokatör
qso-step-tx-report = Yayın Rapor
qso-step-tx-r-report = Yayın R+Rapor
qso-step-tx-rr73 = Yayın RR73
qso-step-tx73 = Yayın 73
qso-step-confirming = Onaylanıyor
rds-standard-auto = Otomatik
rds-standard-rds = RDS
rds-standard-rbds = RBDS
region-r1 = 1. Bölge — Avrupa, Afrika, Orta Doğu, Kuzey Asya
region-r2 = 2. Bölge — Amerikalar
region-r3 = 3. Bölge — Güney ve Doğu Asya, Avustralasya, Pasifik
relative-mode-absolute = Mutlak (fader)
relative-mode-sign-magnitude = Göreli: işaret/büyüklük
relative-mode-twos-complement = Göreli: iki tümleyen
relative-mode-binary-offset = Göreli: 64 merkezli
relay-link-off = Yok
relay-link-serial = USB röle kartı (seri)
relay-link-serial-lines = Seri RTS/DTR hattı
relay-link-hid = USB röle kartı (HID)
relay-link-cm108 = CM108/CM119 ses kartı GPIO
relay-link-gpio = Linux GPIO hattı
relay-link-command = Harici komut
relay-role-unused = Kullanılmıyor
relay-role-sdr-antenna = SDR anteni topraklansın
relay-role-amplifier = Bir amplifikatörü / T-R rölesini anahtarla
relay-role-aux = Yardımcı
relay-role-band-decoder = Bant çözücü (filtre / transverter)
rfe-channel-wb1000 = Geniş bant 1 – 1000 MHz
rfe-channel-wb4000 = Geniş bant 1 – 4 GHz
rfe-channel-ham0030 = HF, 30 MHz'e kadar
rfe-channel-ham0070 = 6 m / 4 m (50 – 70 MHz)
rfe-channel-ham0145 = 2 m (140 – 150 MHz)
rfe-channel-ham0220 = 1,25 m (220 – 225 MHz)
rfe-channel-ham0435 = 70 cm (400 – 450 MHz)
rfe-channel-ham0920 = 33 cm (902 – 928 MHz)
rfe-channel-ham1280 = 23 cm (1220 – 1420 MHz)
rfe-channel-ham2400 = 13 cm (2,3 – 2,5 GHz)
rfe-channel-ham3500 = 9 cm (3,3 – 3,7 GHz)
rfe-channel-cell-band01 = Hücresel bant 1
rfe-channel-cell-band02 = Hücresel bant 2
rfe-channel-cell-band03 = Hücresel bant 3
rfe-channel-cell-band07 = Hücresel bant 7
rfe-channel-cell-band38 = Hücresel bant 38
rfe-channel-not-selected = Hiçbiri seçilmedi
rfe-link-off = Bağlı değil
rfe-link-serial = Kendi USB kablosu (seri)
rfe-link-board = LimeSDR üzerinden (GPIO / I²C)
rfe-mode-control-auto = Otomatik (kablolamayı izle)
rfe-mode-control-rx = Her zaman al
rfe-mode-control-tx = Her zaman yayınla
rfe-mode-control-tx-rx = Her zaman ikisi (yalnızca ayrı konektörler)
rifp-encoding-auto = Otomatik
rifp-encoding-png = PNG
rifp-encoding-jpeg = JPEG
rifp-encoding-raw = Ham
rifp-encoding-rle8 = RLE8
rifp-encoding-zlib = Zlib
rifp-encoding-group4 = G4
rifp-encoding-group3 = G3
scan-kind-memories = HAF
scan-kind-range = ARALIK
scan-resume-carrier = TAŞIYICI
scan-resume-timed = ZAMANLI
scan-resume-manual = ELLE
sense-line-off = Bağlı değil
sense-line-cts = CTS
sense-line-dsr = DSR
sense-line-dcd = DCD (taşıyıcı algılama)
shift-simplex = SİMPLEKS
shift-minus = −
shift-plus = +
speed-slow = Yavaş
speed-medium = Orta
speed-fast = Hızlı
speed-faster = Daha hızlı
speed-fastest = En hızlı
tls-verify-system = Sistem güven deposu
tls-verify-pinned = Sabitlenmiş parmak izi
tls-verify-off = Doğrulama
tone-mode-off = KAPALI
tone-mode-ctcss = CTCSS
tone-mode-dcs = DCS
tx-voice-stage-gate = Kapı
tx-voice-stage-eq = EQ
tx-voice-stage-de-ess = De-esser
tx-voice-stage-comp = Kompresör
upload-naming-sequence = image1 … imageN (döngüsel)
upload-naming-original = Özgün dosya adı
wheel-action-zoom = Yakınlaştır
wheel-action-tune = Ayarla
wheel-action-none = Hiçbiri
