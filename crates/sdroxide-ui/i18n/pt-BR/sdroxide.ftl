# sdroxide — o texto da interface em inglês, a referência com que cada tradução
# é conferida. Veja crates/sdroxide-ui/src/i18n.rs para as regras.
#
# Os ids são agrupados pelo lugar onde aparecem, e nomeados segundo ele:
# settings-ui-* é a aba UI da janela de Configurações, e assim por diante. Uma mensagem
# mostrada em maiúsculas é escrita em maiúsculas aqui; o código não converte
# o texto traduzido para maiúsculas.

## O nome do próprio idioma, como o seletor de idiomas o lista.
language-name = Português (Brasil)

## Valores de sdroxide-types, como a tela os nomeia (src/i18n/labels.rs).

layout-mode-auto = Automático
layout-mode-desktop = Desktop
layout-mode-tablet = Tablet
layout-mode-small = Tela pequena
layout-mode-phone = Celular

ui-theme-default = Padrão
ui-theme-light = Claro
ui-theme-high-contrast = Alto contraste
ui-theme-green-phosphor = Fósforo verde
ui-theme-amber-phosphor = Fósforo âmbar
ui-theme-teal-orange = Verde-azulado / laranja
ui-theme-rainbow = Arco-íris
ui-theme-nord = Nord
ui-theme-nord-dark = Nord escuro
ui-theme-gruvbox = Gruvbox
ui-theme-everforest = Everforest
ui-theme-solarized-dark = Solarized escuro
ui-theme-solarized = Solarized
ui-theme-dracula = Dracula
ui-theme-catppuccin-mocha = Catppuccin mocha
ui-theme-catppuccin-latte = Catppuccin latte
ui-theme-modern-minimalist = Minimalista moderno

chrome-style-angled = Angular
chrome-style-rectangular = Retangular
chrome-style-rounded = Arredondado
chrome-style-gradient = Gradiente
chrome-style-bevel = Bisel 3D
chrome-style-terminal = Terminal

font-size-small = Pequeno
font-size-medium = Médio
font-size-large = Grande

bandplan-kind-ham = Amador
bandplan-kind-cw = CW
bandplan-kind-digi = Digital
bandplan-kind-phone = Voz
bandplan-kind-beacon = Balizas
bandplan-kind-broadcast = Radiodifusão
bandplan-kind-am = AM / OL / OM
bandplan-kind-cb = CB

speech-verbosity-terse = Conciso
speech-verbosity-normal = Normal
speech-verbosity-full = Completo

# Os exemplos entre parênteses são o que a voz (em inglês) diz.
speech-freq-style-dial = Dial (14 point 074)
speech-freq-style-digits = Cada dígito
speech-freq-style-full = Dial com unidades

speech-callsign-style-phonetic = Fonético (kilo one alpha)
speech-callsign-style-letters = Letras (kay one ay)
speech-callsign-style-as-is = Como escrito

## Janela de Configurações, aba UI (src/app/settings/ui_tab.rs).

settings-ui-display = Tela
settings-ui-moved-note = O detalhe do panadaptador, a reação do espectro e a velocidade de rolagem da cascata são definidos pelo botão SPEC na barra de exibição — o menu DISP em uma janela estreita.
settings-ui-language = Idioma
    .hint = O idioma da interface. O manual aberto com F1 e os anúncios de voz continuam em inglês, qualquer que seja a opção escolhida aqui.
settings-ui-language-system = Sistema ({ $language })
settings-ui-layout = Layout
settings-ui-theme = Tema
settings-ui-button-style = Estilo dos botões
settings-ui-window-style = Estilo das janelas
settings-ui-frame-rate = Taxa de atualização da tela
settings-ui-palette = Paleta da cascata
settings-ui-tuning-buttons = Botões de sintonia
settings-ui-tuning-buttons-show = Mostrar em celular e tablet
    .hint = Uma linha de botões diminuir / passo / aumentar abaixo da barra de controles em uma tela sensível ao toque. Não há roda para girar um dígito nem dial, então, sem eles, a única forma de mover um passo conhecido é digitar a frequência inteira. Nunca aparece em desktop.
settings-ui-tuning-step = Passo: { $step }
settings-ui-tuning-snap = O primeiro toque ajusta ao passo
    .hint = Um dial parado entre dois múltiplos do passo vai para o próximo na direção pressionada — 7 074 300 sobe para 7 075 000 com passo de 1 kHz, desce para 7 074 000. Depois disso, os botões se movem pelo passo normalmente. Desligado, cada toque move exatamente o passo.
settings-ui-smoothing = Suavização da cascata
settings-ui-smoothing-interpolate = Interpolar
    .hint = Mistura cada pixel da tela com as raias e linhas vizinhas, para que um sinal pareça contínuo quando a tela é mais larga que a transformada. Desligue para uma cascata retangular — um bloco por raia, um por linha —, que é o necessário para ler a forma de um sinal na imagem: um sinal interpolado não se distingue de um genuinamente mais largo. Uma FFT maior (o chip FFT) é a outra metade disso.
settings-ui-clock = Relógio
settings-ui-clock-show = UTC + local no panadaptador
    .hint = Um relógio no canto superior direito do panadaptador, em UTC e na sua hora local.
settings-ui-background = Fundo do espectro
settings-ui-background-gradient = Gradiente
settings-ui-background-top = topo
settings-ui-background-bottom = base
settings-ui-spot-colours = Cores dos rótulos de spots
    .hint = A cor que cada fonte de spots assume: as caixas ao longo da base da cascata, os distintivos na lista SPOTS e os pontos no mapa-múndi.
settings-ui-spot-colour = Cor para spots de { $kind }
settings-ui-spot-colours-reset = Devolver todas as cores de spots ao padrão
settings-ui-reset = Redefinir
settings-ui-bandplan-colours = Cores do plano de faixas
    .hint = O tom em que cada classe de alocação é pintada na faixa do plano de faixas ao longo da base da cascata. Os blocos são desenhados semitransparentes sobre a cascata, então ficam mais escuros lá do que na amostra aqui.
settings-ui-bandplan-colour = Cor para alocações de { $kind }
settings-ui-bandplan-colours-reset = Devolver todas as cores do plano de faixas ao padrão
settings-ui-skimmer-font = Tamanho da fonte do skimmer
settings-ui-waterfall-font = Tamanho da fonte da cascata / espectro
settings-ui-interface-font = Tamanho da fonte da interface
    .hint = Escala a interface inteira — menus, diálogos, janelas, as abas de rádio, a barra superior e seus botões. Os dois tamanhos acima são relativos a ele.
settings-ui-cities = Cidades nos mapas
    .hint =
        Desenha as cidades do mundo — um ponto por lugar, com o nome ao lado quando há espaço — nos mapas planos: FT8/WSPR, APRS, ADS-B e AIS.

        Elas são a maior parte do que diz *onde* está um ponto em um mapa do mundo inteiro, e também são a coisa mais carregada dele. Desligue se os nomes estiverem atrapalhando as estações que você está lendo. O globo 3D não é afetado — as cidades dele são luzes do lado noturno, não marcadores, e nada ali é escrito sobre um contato.
settings-ui-cities-show = mostrar cidades e seus nomes
settings-ui-update-check = Verificar atualizações ao iniciar
    .hint = Pergunta ao sdroxide.com, uma vez por inicialização, se uma versão mais nova foi publicada, e informa isso em um aviso dispensável acima da cascata. Nada é enviado além da própria requisição.
settings-ui-band-conditions-download = Baixar a previsão de condições das bandas
    .hint = Busca a previsão de condições das bandas de HF no HAMQSL.com, no máximo uma vez por hora, para colorir o menu de bandas. Desligado, o menu de bandas não mostra previsão, a menos que a visão 3D esteja aberta. Nada é enviado além da própria requisição.
settings-ui-broadcast-schedule-download = Baixar a grade de radiodifusão
    .hint = Baixa a grade de radiodifusão em ondas curtas EiBi (cerca de 1 MB) do eibispace.de quando começa uma nova temporada. Desligado, continua em uso a grade embutida no sdroxide (ou a última baixada), e Configurações → Spots → Baixar programação agora ainda a baixa sob demanda.
settings-ui-3d-view = Visão 3D
settings-ui-clouds = Renderização de nuvens
settings-ui-clouds-layered = Em camadas
settings-ui-clouds-volumetric = Volumétrica
settings-ui-clouds-note = Como a camada CLOUDS da visão 3D desenha o tempo. Em camadas empilha fatias da troposfera e é a opção leve. Volumétrica percorre um raio por ela, então o Sol projeta os topos das nuvens sobre a camada de baixo e os relâmpagos brilham de dentro da tempestade que os produz, em vez de apenas clarear o seu exterior — a um custo por pixel de várias vezes maior.

## Janela de Configurações, aba UI: anúncios de voz.

speech-title = Anúncios de voz
speech-enabled = Falar as mudanças do rádio
    .hint = Lê em voz alta o que mudou, para que o rádio possa ser operado sem ser visto
speech-voice = Voz
speech-voice-shipped = Voz padrão
speech-speed = Velocidade
    .hint = A voz estica ou comprime a própria entonação, então o tom não muda. Acima de cerca de 2×, ela deixa de encurtar.
speech-volume = Volume
speech-output = Saída
speech-detail = Detalhe
speech-duck = Reduzir o receptor
speech-duck-while = Enquanto fala
speech-test = Testar
speech-what = O que anunciar
speech-cat-frequency = Frequência
speech-cat-mode-band = Modo e faixa
speech-cat-vfo-split = VFO e split
speech-cat-agc-gain = AGC e ganho
speech-cat-levels = Excitação, sintonia e microfone
speech-cat-ptt = Transmissão e recepção
speech-cat-rit-xit = RIT e XIT
speech-cat-memory-scan = Memórias e varredura
speech-cat-band-edge = Saída de uma faixa de amador
speech-cat-notices = Avisos e mensagens
speech-cat-filters = Filtros, squelch e redução de ruído
    .hint = Desligado por padrão: estes mudam o tempo todo ao seguir um sinal
speech-decoded = Mensagens decodificadas
speech-ft8-to-me = Chamadas FT8 para mim
speech-ft8-cq = CQs FT8 que eu poderia responder
    .hint = Uma noite movimentada em vinte metros são uma centena destes por minuto
speech-js8 = Mensagens JS8 para mim
speech-js8-allcall = JS8 @ALLCALL também
speech-fsq = Mensagens FSQ para mim
speech-include-snr = Incluir o relatório
    .hint = Só onde a mensagem não traz um próprio — uma decodificação que já informa um número não recebe também o nosso
speech-ft8-qso = Minha própria troca FT8
    .hint = O que o sequenciador está prestes a enviar, e o término do contato
speech-reading-text = Ler em voz alta o texto decodificado
speech-text-cw = CW
speech-text-rtty-psk = RTTY, PSK, Olivia, THOR, FSQ
speech-text-cw-locked = CW somente enquanto o decodificador estiver travado
    .hint = Ler a saída de um decodificador destravado é pior que silêncio
speech-text-note = Ambos vêm desligados por padrão. Um decodificador produz texto mais depressa do que a fala consegue lê-lo, então tudo o que fica muito atrás do áudio ao vivo é descartado em vez de enfileirado.
speech-tuning-up = Ajustando
speech-swr-while-tuning = Dizer a ROE enquanto TUNE estiver pressionado
speech-swr-every = A cada
speech-swr-summary = Informar a melhor casamento ao soltar
speech-swr-alarm = Avisar sobre ROE alta durante qualquer transmissão
speech-how-read = Como as coisas são lidas
speech-frequencies = Frequências
speech-callsigns = Indicativos
speech-quiet-on-tx = Ficar em silêncio durante a transmissão
    .hint = A fala sai pelos seus alto-falantes e, portanto, entra no seu microfone. Os avisos de ROE alta continuam passando.
speech-device-note = Os anúncios tocam no próprio dispositivo de som, então nunca são gravados nem enviados a quem estiver ouvindo remotamente. As teclas para falar o status, repetir o último anúncio e interromper no meio de uma frase ficam na aba Controles.

## Janela de Configurações (src/app/settings/mod.rs)

settings-waiting-station-s-network-configuration = Aguardando a configuração de rede da estação…
settings-device-reports = como o dispositivo informa
settings-transverters = Transverters
settings-one-row-per-box-while = Uma linha por caixa. Enquanto o dial estiver dentro da faixa de uma linha, o rádio é sintonizado em dial + deslocamento e valem a regra de transmissão e o limite de excitação dessa linha; fora de todas as linhas, o rádio fica nas próprias faixas. As linhas são testadas de cima para baixo.
settings-name = Nome
settings-band-low = Início da faixa
settings-band-high = Fim da faixa
settings-offset = Deslocamento
settings-transmit = Transmissão
settings-max-drive = Excitação máxima
settings-off-takes-transverter-out-line = Desligado tira este transverter da linha sem perder a linha — a faixa volta ao que o rádio alcança por conta própria.
settings-t-2-m = 2 m
settings-what-you-call-shown-log = Como você o chama. Mostrado no log quando o dial o seleciona.
settings-bottom-band-transverter-works-dial = O início da faixa em que este transverter trabalha, no dial. 144 para 2 m.
settings-top-band-dial-148-2 = O fim da faixa, no dial. 148 para 2 m nas Regiões 2 e 3, 146 na Região 1.
settings-radio-ends-up-dial-offset = O rádio acaba em dial + deslocamento, então um transverter que leva uma faixa para um F.I. é negativo: 2 m em um F.I. de 28 MHz é -116, porque 144 - 116 = 28. Arraste para ajustar um hertz por vez, para um oscilador um pouco fora de frequência.
settings-off-while-converting-receive-only =
    Desligado enquanto converte: somente recepção — a linha é um conversor, não um transverter, e nada é transmitido nesta faixa.

    Pelo mesmo conversor: funciona nos dois sentidos, e a transmissão usa o mesmo deslocamento.
settings-ceiling-transmit-drive-while-transverter =
    Um teto para a excitação de transmissão enquanto este transverter estiver selecionado, em porcentagem da potência total. A entrada de F.I. de um transverter aceita miliwatts, e a excitação certa para as faixas próprias do rádio o destruiria, então este é o campo mais importante da linha. 100% significa sem limite.

    Sua configuração de Excitação fica abaixo deste teto em vez de ser alterada, então o valor que você usa em HF volta quando o dial sai da faixa.
settings-remove = REMOVER
settings-add-transverter = ADICIONAR TRANSVERTER
settings-takes-effect-apply-reconnect-like = Tem efeito ao Aplicar / reconectar, como o deslocamento do conversor acima.
settings-maximum-transmit-drive = Excitação máxima de transmissão
settings-hard-limit-drive-tune-controls = Um limite rígido que os controles de Excitação e TUNE não podem ultrapassar, aplicado depois da calibração de faixa abaixo para que nada possa elevar a excitação acima dele. Defina-o no ponto em que o rádio entrega a potência nominal, e todo o controle de Excitação fica utilizável — em um transmissor que modula as próprias amostras (HPSDR, LimeSDR, PlutoSDR, HackRF), a excitação total é o transmissor no máximo, o que, em um amplificador de alto ganho, fica muito acima do que os seus finais suportam. Desligado, os controles chegam à excitação total.
settings-limit-drive = Limitar a excitação a
settings-per-cent-full-drive-key = Porcentagem da excitação total. Coloque o rádio em uma carga fantasma e reduza até que o medidor mostre a potência para a qual o amplificador é especificado.
settings-applies-immediately-every-band-tune = Vale imediatamente, em todas as faixas e também para TUNE.
settings-transmit-drive-band = Excitação de transmissão por faixa
settings-every-amplifier-has-different-gain = Todo amplificador tem ganho diferente em cada faixa, então uma mesma configuração de Excitação produz uma potência diferente em cada uma. Meça a saída em cada faixa e depois ajuste as faixas que saem altas até que todas combinem: defina a Excitação para a faixa que mais precisa (geralmente a mais alta) e reduza as outras até ela. Decibéis de potência de saída, aplicados igualmente a voz, digital e TUNE. Zero em todas — o padrão — não muda nada.
settings-other = Outras
settings-clear-all = LIMPAR TUDO
settings-back-no-calibration-drive-setting = Voltar a nenhuma calibração: a configuração de Excitação alcança cada faixa integralmente.
settings-applies-immediately-band-you-would = Vale imediatamente, e para a faixa em que você transmitiria — atrás de um transverter, é a faixa no dial, e o limite de excitação do próprio transverter ainda prevalece sobre qualquer coisa definida aqui.
settings-settings = Configurações
settings-enter-server-s-address-first = informe primeiro o endereço do servidor
settings-enter-receiver-s-address-first = informe primeiro o endereço do receptor, ou escolha um em SDRs públicos
settings-no-radio-selected-press-discover = nenhum rádio selecionado — pressione Descobrir ou informe um endereço
settings-enter-radio-s-address-first = informe primeiro o endereço do rádio
settings-general = Geral
settings-radio = Rádio
settings-ui = UI
settings-alerts = Alertas
settings-controls = Controles
settings-spots = Spots
settings-freedv = FreeDV
settings-uploads = Envios
settings-winlink = Winlink
settings-servers = Servidores
settings-t-r-switch = Chave T/R
settings-amplifier = Amplificador
settings-remote = Remoto
settings-tle = TLE
settings-profiles = Perfis
settings-station = Estação
settings-enter-digital-mode-ft8-sstv = Informe um modo digital (FT8 / SSTV / …) uma vez para carregar os valores salvos.
settings-callsign = Indicativo
settings-grid-square = Quadrado locator
settings-your-callsign-grid-shared-across = Seu indicativo e locator, compartilhados entre FT8/FT4/FT2, cabeçalhos de imagem SSTV e o diário de contatos. Também editável pela caixa de configuração FT8 / SSTV.
settings-iaru-region = Região IARU
settings-which-radio-interface-server-uses = A interface de rádio que o servidor usa é definida na máquina onde ele roda.
settings-radio-interface = Interface do rádio
settings-waiting-machine-radio-attached-interface = Aguardando a máquina à qual o rádio está conectado. As configurações da interface dela estão abaixo e podem ser alteradas daqui enquanto isso.
settings-converter = Conversor
settings-manual-type-offset-below = Manual — digite um deslocamento abaixo
settings-what-box-reads-when-offset = O que esta caixa mostra quando o deslocamento não é um dos predefinidos. Defina-o digitando no campo Deslocamento abaixo; não há nada a selecionar aqui.
settings-frequency-converter-front-receiver-pick = Um conversor de frequência na frente do receptor. Escolha um, ou digite um deslocamento ao lado.
settings-how-far-converter-moves-signal =
    Quanto um conversor desloca o sinal no caminho até o receptor, em Hz — o mesmo número e sinal que a documentação de todo conversor e de todo outro programa SDR informa. O rádio acaba em dial + deslocamento: positivo para um upconverter (um Ham It Up é 125000000), negativo para qualquer coisa que leva uma faixa a um F.I. — um LNB de satélite, ou um transverter de 2 m em um F.I. de 28 MHz, que é -116000000 (28 − 144). 0 = sem conversor.

    Arraste para ajustar um hertz por vez, o que um conversor com oscilador um pouco fora de frequência precisa.

    Este é o caminho de recepção. O que entra na linha de transmissão é a linha abaixo.

    Tem efeito ao Aplicar.
settings-what-transmit-line-while-converter =
    O que entra na linha de transmissão enquanto um conversor estiver definido.

    Desligado enquanto converte: nada é transmitido — o padrão seguro, e o certo para um acessório só de recepção.

    Pelo mesmo conversor: uma caixa converte nos dois sentidos, um transverter. A transmissão usa o deslocamento acima e o acompanha quando ele é ajustado.

    Deslocamento próprio: a linha de transmissão é diferente da de recepção. Zero é o caso comum — um conversor (um LNB, um Ham It Up) na recepção com o transmissor na própria antena, que é a estação QO-100.

    A verificação de faixa de amador ainda se aplica, na frequência em que você está transmitindo.

    Tem efeito ao Aplicar.
settings-how-far-transmit-line-moves =
    Quanto a linha de transmissão desloca o sinal, em Hz, com a mesma regra de sinal do deslocamento de recepção: o rádio transmite em dial + deslocamento.

    0 = o transmissor está ligado à própria antena e trabalha na frequência que o dial indica — o uplink de 2,4 GHz de uma estação QO-100.

    Um conversor de transmissão que leva um F.I. *para cima* é um número negativo: o rádio trabalha abaixo do dial.
settings-rx-range = Faixa de RX
settings-which-frequencies-radio-receives-mhz =
    Quais frequências este rádio recebe, em MHz: 144-146, 430-440. Deixe vazio para usar o que o dispositivo informa sobre si mesmo.

    Os botões de faixa fora do intervalo ficam acinzentados, e o dial não vai até lá.

    São frequências de dial. Com um conversor definido, informe a faixa que você sintoniza — 144-148 para um transverter de 2 m — e não o F.I. em que o rádio realmente está.

    Tem efeito ao Aplicar.
settings-tx-range = Faixa de TX
settings-which-frequencies-radio-transmits-mhz =
    Em quais frequências este rádio transmite, em MHz: 144-146, 430-440. Deixe vazio para usar o que o dispositivo informa — e, se ele não informar nada, o driver é aceito pela palavra e qualquer frequência é permitida.

    Frequências de dial, como a faixa de recepção acima.

    Este é um limite que você define, não uma licença: transmitir fora das faixas de amador é recusado de qualquer forma, a menos que você tenha desligado isso no config.toml. Também não dá um transmissor a um dispositivo só de recepção.

    Tem efeito ao Aplicar.
settings-antenna = A antena está
settings-where-radio-listens-station-antenna =
    De onde este rádio escuta.

    Na estação: a antena é sua, então seu próprio locator diz onde ela está. O padrão, e o certo para tudo na estação.

    Em outro lugar: um receptor online, ou o seu montado em um morro. O que ele ouve é informado ao PSK Reporter, WSPRnet e FreeDV Reporter a partir do locator ao lado, nunca a partir do seu — e o ADS-B também posiciona aeronaves com base nele.

    Escolher um receptor em "SDRs públicos" preenche isto para você.

    Tem efeito ao Aplicar.
settings-receiver-s-maidenhead-locator-jn88ec =
    O locator Maidenhead do receptor — JN88ec, DO30db. Quatro ou seis caracteres.

    Deixe vazio se não souber onde o receptor está: nada então é informado, que é a única resposta honesta. Informar a partir do seu próprio quadrado colocaria a recepção de outra pessoa no continente errado.
settings-audio = Áudio para
settings-gain-per-band = Ganho por faixa
settings-remember-receive-gain-per-band = Lembrar o ganho de recepção por faixa
settings-each-band-keeps-receive-gain = Cada faixa mantém o ganho de recepção que você definiu por último nela e o recupera quando o dial volta — um botão de faixa, um arraste na borda de uma faixa ou o botão próprio do rádio. A cobertura geral é dividida em HF, VHF, UHF e acima. Uma faixa em que você não definiu ganho mantém o que estiver em vigor. Desligado: um só ganho em todas.
settings-s-meter-calibration = Calibração do S-metro
settings-added-radio-s-s-meter =
    Somado à leitura do S-metro deste rádio. O medidor de um SDR começa lendo dBFS — nível em relação à escala total do conversor — então um receptor com bastante ganho pode mostrar S9 sem antena conectada. Defina isto como a diferença entre o que o medidor mostra e um nível que você conhece: um gerador de sinais, ou uma faixa silenciosa em que você confie para ficar por volta de S2–S3. Vale imediatamente.

    Somado ao cal_offset_db do config.toml, que é um valor único para a estação inteira.
settings-t-0-db = 0 dB
settings-antenna-somewhere-else-no-locator = A antena está em outro lugar e nenhum locator está definido, então nada que este rádio ouve é informado ao PSK Reporter, WSPRnet ou FreeDV Reporter. Digite o locator do receptor acima para informar a partir dele.
settings-ranges-are-mhz-low-high = Os intervalos estão em MHz, inicial-final, separados por vírgulas: 144-146, 430-440 — ou seja, 144000000-146000000 Hz e 430000000-440000000 Hz. São frequências de dial: um deslocamento de conversor não as move. O deslocamento de conversor acima é o campo em hertz. Deixe um intervalo vazio para usar o que o dispositivo informa sobre si mesmo; um dispositivo que não informa nada é aceito pela palavra.
settings-careful-rtl-sdr-blog-v4 = Cuidado com um RTL-SDR: o Blog V4 faz upconversão por conta própria abaixo de 28,8 MHz, então um deslocamento negativo que leve o hardware para lá desloca duas vezes.
settings-transmit-audio = Áudio de transmissão
settings-use = em uso
settings-every-stage-off = todos os estágios desligados
settings-open-tx-audio = Abrir Áudio TX…
settings-microphone-processing-chain-gate-eq = A cadeia de processamento do microfone — gate, EQ, de-esser, compressor e limitador — com uma gravação de teste para ajustar de ouvido sem transmitir. Somente modos de voz. Também aberta ao clicar em Mic na caixa TX.
settings-pick-radio-interface-above-configuration = Escolha uma interface de rádio acima (esta configuração usava o modo de detecção automática, que foi removido).
settings-configuration-used-removed-auto-detect = Esta configuração usava o modo de detecção automática, que foi removido. Escolha uma interface acima quando a máquina à qual o rádio está conectado tiver respondido.
settings-radio-has-no-interface-yet = Este rádio ainda não tem interface — escolha uma acima e pressione Aplicar / reconectar.
settings-radio-has-no-interface-yet-2 = Este rádio ainda não tem interface. Uma pode ser escolhida acima quando a máquina à qual o rádio está conectado tiver respondido.
settings-apply-reconnect = Aplicar / reconectar
settings-switch-interface-now-no-restart = Trocar para esta interface agora — sem precisar reiniciar
settings-switch-server-s-radio-interface = Trocar agora o rádio do servidor para esta interface — sem precisar reiniciar. A antiga continua rodando se a nova não puder ser aberta.
settings-switches-live-radio-without-restarting = Troca o rádio em uso sem reiniciar.
settings-everything-above-server-s-own = Tudo acima é a configuração própria do servidor, e as alterações são salvas lá. A maioria das configurações vale enquanto você as altera; as que são fixadas quando o dispositivo é aberto — a taxa de amostragem, um endereço, a própria interface — precisam deste botão.
settings-dx-cluster-telnet = Cluster DX (telnet)
settings-enabled = Ativado
settings-host = Host
settings-port = Porta
settings-login-call = Indicativo de login
settings-commands = Comandos
settings-one-per-line-e-g = um por linha, ex.: SET/FT8
settings-reverse-beacon-network = Reverse Beacon Network
settings-read-world-s-cw-rtty = Lê os skimmers de CW/RTTY do mundo e alimenta com eles o mapa de propagação. É isso que faz o mapa mostrar faixas que este rádio não está ouvindo. Ligado por padrão: não coloca nada no ar, não precisa de conta e usa o indicativo da aba Geral. Os spots do RBN não aparecem na lista de spots — são medições, não convites.
settings-t-7000-cw-rtty-feed-7001 = 7000 é o feed de CW/RTTY, 7001 o de FT8/FT4
settings-one-per-line-e-g-2 = um por linha, ex.: set/filter cont=eu
settings-sent-after-login-place-narrow = Enviado após o login. O lugar para restringir o feed — sem um filtro, são todos os skimmers da Terra.
settings-rbn-paths-are-placed-country = Os caminhos do RBN são posicionados a partir de centros de países, não de locators — precisos para um país pequeno, e errados por muito para um grande. Eles são uma camada separada do mapa de propagação e podem ser desligados ali.
settings-pota-sota-psk-reporter = POTA / SOTA / PSK Reporter
settings-pota-activator-spots = Spots de ativadores POTA
settings-sota-spots = Spots SOTA
settings-psk-reporter-current-band = PSK Reporter (faixa atual)
settings-upload-my-ft8-ft4-ft2 = Enviar minhas decodificações FT8/FT4/FT2/JS8
settings-report-what-station-hears-pskreporter = Informa ao pskreporter.info o que esta estação ouve, para que ela apareça lá como um receptor. Usa o indicativo e o locator da aba Geral.
settings-antenna-2 = Antena
settings-collector = Coletor
settings-t-4739-live-collector-14739-test = 4739 é o coletor ao vivo, 14739 o de teste
settings-max-age-s = Idade máxima (s)
settings-wsprnet = WSPRnet
settings-upload-my-wspr-decodes = Enviar minhas decodificações WSPR
settings-send-every-wspr-reception-wsprnet = Envia cada recepção WSPR ao wsprnet.org. Ligado por padrão: não coloca nada no ar, e informar o que você ouve é o que torna um receptor WSPR parte da rede em vez de uma curiosidade privada. Um intervalo que não decodificou nada também é informado, que é como a rede distingue uma faixa fechada de um receptor desligado.
settings-download-who-heard-me = Baixar quem me ouviu
settings-ask-wsprnet-org-which-stations = Pergunta ao wsprnet.org quais estações decodificaram esta. O WSPR não tem nenhum tipo de confirmação, então esta é a única forma de um balizador transmissor saber algo sobre o próprio alcance.
settings-ask-every = Perguntar a cada
settings-looking-back = olhando para trás
settings-apply = {" "}APLICAR{" "}
settings-persist-re-connect-feeds = Salvar e (re)conectar os feeds
settings-broadcast-stations = Emissoras de rádio
settings-winlink-account = Conta Winlink
settings-password = Senha
settings-winlink-account-password-not-gateway = A senha da conta Winlink, não a senha do gateway. Diferencia maiúsculas e minúsculas — digite exatamente como foi emitida.
settings-locator = Locator
settings-how-connect = Como conectar
settings-route = Rota
settings-internet = Internet
settings-forward-cms-over-internet = Encaminhar com o CMS pela internet
settings-radio-packet = Rádio (pacote)
settings-call-rms-gateway-air-radio = Chama um gateway RMS pelo ar. O rádio precisa estar em PACKET ou PACKET-HF.
settings-gateway = Gateway
settings-via = Via
settings-digipeaters-order-separated-spaces-usually = Digipeaters, em ordem, separados por espaços. Geralmente vazio — um gateway que você ouve diretamente é um gateway que você deve chamar diretamente.
settings-speed = Velocidade
settings-most-rms-gateways-answer-1200 = A maioria dos gateways RMS responde a 1200. Chamar um gateway de 1200 a 9600 soa exatamente como um gateway fora do ar, então defina isto a partir do que o dono do gateway publica, em vez de chutar. 9600 também exige a porta de dados do rádio nas duas pontas. 300 é HF.
settings-frequency = Frequência
settings-clear = LIMPAR
settings-zero-leaves-dial-alone-which = Zero deixa o dial em paz, o que você quer quando fica em um canal só. Qualquer outro valor sintoniza o rádio quando a sessão começa.
settings-my-gateways = Meus gateways
settings-winlink-s-published-gateway-list = A lista de gateways publicada pelo Winlink exige uma chave de API que o sdroxide não tem, então mantenha a sua. Os dois ou três gateways alcançáveis a partir de um local são descobertos tentando, e raramente mudam.
settings-use-2 = USAR
settings-call-one-next-connect = Chamar este no próximo connect
settings-current-dial = dial atual
settings-forget = ESQUECER
settings-forget-gateway = Esquecer este gateway
settings-add-gateway = + ADICIONAR GATEWAY
settings-save-gateway-above-list = Salvar o gateway acima na lista
settings-internet-gateway = Gateway de internet
settings-cms-address = Endereço do CMS
settings-client-name = Nome do cliente
settings-winlink-s-production-servers-only = Os servidores de produção do Winlink só aceitam nomes de cliente que conhecem, e respondem a um desconhecido com "Unknown client types are not allowed on production servers". Até que o sdroxide seja registrado junto à Winlink Development Team, conectar exige um nome que eles reconheçam.
settings-automatic-connection = Conexão automática
settings-connect-timer = Conectar por temporizador
settings-every = A cada
settings-persist-winlink-account = Salvar a conta Winlink
settings-callsign-lookup = Consulta de indicativo
settings-provider = Provedor
settings-auto-fill-name-qth-grid = Preencher automaticamente nome/QTH/locator ao clicar em spot e QSO
settings-choose-provider-enter-login = Escolha um provedor para informar o seu login.
settings-qrz-user = Usuário QRZ
settings-qrz-pass = Senha QRZ
settings-hamqth-user = Usuário HamQTH
settings-hamqth-pass = Senha HamQTH
settings-one-hamqth-account-does-both = Uma conta HamQTH faz as duas coisas — este login é o que a aba de envio HamQTH usa.
settings-upload = Enviar
settings-auto-upload-each-new-qso = Enviar automaticamente cada novo QSO
settings-master-switch-which-services-qso = A chave geral. Para quais serviços um QSO vai é definido em cada aba abaixo.
settings-turn-auto-upload-each-new = Ative primeiro "Enviar automaticamente cada novo QSO" acima — um serviço marcado aqui não é enviado até que isso esteja ativo.
settings-auto-upload-off-above-so = O envio automático está desligado acima, então as marcações de serviço ficam desativadas e nada é enviado automaticamente ainda — o botão ENVIAR por QSO no diário continua funcionando.
settings-qrz-log-key = Chave de log QRZ
settings-qrz-logbook-api-key-your = A chave de API do diário QRZ, das suas configurações do diário QRZ — não o login da consulta XML.
settings-eqsl-user = Usuário eQSL
settings-eqsl-pass = Senha eQSL
settings-eqsl-qth = QTH eQSL
settings-qth-nickname-eqsl-profile-upload = O apelido de QTH do perfil eQSL para enviar e baixar — necessário só quando a conta tem mais de um; caso contrário, deixe em branco.
settings-same-login-hamqth-callsign-lookup = O mesmo login da consulta de indicativo HamQTH — preencher um preenche os dois.
settings-club-log-email = E-mail do Club Log
settings-club-log-pass = Senha do Club Log
settings-club-log-key = Chave do Club Log
settings-wrl-api-key = Chave de API WRL
settings-developer-api-key-world-radio = A chave de API de desenvolvedor, em World Radio League → Integrations → Developer API. Ela é mostrada uma só vez, quando você a gera, então copie-a nesse momento. Os contatos vão para o seu diário padrão — defina um no WRL se tiver mais de um.
settings-confirmations-download = Confirmações (download)
settings-lotw-user = Usuário LoTW
settings-lotw-pass = Senha LoTW
settings-lotw-upload-uses-tqsl-export = O envio ao LoTW usa o TQSL — exporte o ADIF do diário e assine-o. As confirmações LoTW/eQSL são baixadas aqui para marcar trabalhado versus confirmado.
settings-sync-confirmations = SINCRONIZAR CONFIRMAÇÕES
settings-fetching-subscriptions = Buscando assinaturas…
settings-spectrum-waterfall-everything-read-off =
    O espectro, a cascata e tudo o que é lido deles ficam na aba daquele rádio, por isso ele não tem nenhum próprio. As configurações de interface acima ainda são as deste rádio e continuam valendo — pressione Aplicar / reconectar depois de alterá-las e ambos os rádios voltam juntos.

    Para usá-lo sozinho de novo, limpe a caixa Receptor do panadaptador na página daquele rádio.
settings-panadapter = Panadaptador
settings-give-radio-wideband-panadapter-borrowing = Dê a este rádio um panadaptador de banda larga emprestando o receptor de outro rádio — um SDR na mesma antena, ou um alimentado pela saída de F.I. deste rádio. O espectro, a cascata, o sub receptor, os modos digitais e o skimmer vêm todos desse receptor; o dial, o modo, o filtro e o transmissor continuam aqui.
settings-receiver = Receptor
settings-none = Nenhum
settings-which-radio-receives-one-leaves =
    Qual rádio recebe para este. Ele sai da barra de abas enquanto está emprestado — seu front end pertence ao motor deste rádio — e volta logo que isto é definido como Nenhum.

    Tem efeito ao Aplicar.
settings-connected = Conectado a
settings-how-far-above-radio-s =
    Quanto acima do dial deste rádio fica o receptor, em hertz — 0 para um receptor na mesma antena. Para uma tomada de F.I., a frequência intermediária em que o receptor está parado: 8215000 para o KXV3B de um K3, 9000000 para um F.I. de 9 MHz.

    Isto não é o campo Conversor acima, que sintoniza o rádio, nem o deslocamento de centro I/Q da aba CAT, que diz respeito à placa de som deste rádio. Nada digitado aqui é enviado a nenhum dos dois rádios.

    Tem efeito ao Aplicar.
settings-audio-2 = Áudio de
settings-follow-dial = Seguir o dial
settings-keep-receiver-radio-s-dial = Mantém o receptor no dial deste rádio, e move o dial quando você sintoniza no panadaptador. Desligado deixa o receptor onde está — uma vigília fixa em um segmento enquanto o rádio vai para outro lugar.
settings-invert-spectrum = Inverter o espectro
settings-mirror-receiver-s-span-about = Espelha a largura do receptor em torno do centro, para uma tomada cujo oscilador fica acima do sinal e entrega a faixa ao contrário. Deixe desligado a menos que a cascata se encha de sinais que estão todos do lado errado do dial.
settings-mute-transmit = Silenciar na transmissão
settings-silence-receive-audio-while-radio = Silencia o áudio de recepção enquanto este rádio está transmitindo. Ligado por padrão: com o receptor na mesma antena, ou no F.I. deste rádio, o que ele ouve durante uma transmissão é o seu próprio transmissor. Transmissões acionadas no próprio rádio — o PTT do microfone, um pedal, MOX — também contam, assim que o link CAT informa que o equipamento foi para transmissão.
settings-blank-transmit = Apagar na transmissão
settings-stop-panadapter-waterfall-while-radio = Para o panadaptador e a cascata enquanto este rádio estiver transmitindo. Ligado por padrão: um transmissor pintado em toda a largura apaga a faixa atrás dele. Desligue para acompanhar a faixa — ou o seu próprio sinal — durante uma transmissão, o que vale ter com o receptor em uma antena separada. Vale para as transmissões que o sdroxide aciona; uma acionada no próprio rádio é recebida sem interrupção, então a imagem continua rodando independentemente do que isto diz.
settings-per-mode-offsets-radio-whose = Deslocamentos por modo, para um rádio cujo F.I. muda com o modo. Um modo deixado vazio usa o deslocamento acima.
settings-computer = Neste computador
settings-add-radio-computer = Adicionar um rádio neste computador
settings-radio-s-settings-are-below = As configurações deste rádio estão abaixo. Arraste para movê-lo ao longo da barra.
settings-switch-radio-dialog-follows-drag = Trocar para este rádio (a caixa acompanha). Arraste para movê-lo ao longo da barra.
settings-tx = ● TX
settings-station-radio = Estação:
settings-station-radio-tip = O rádio da estação. Ele executa os serviços de rede da estação (feeds de spots, rotor, amplificador, chave TX/RX) e recebe as configurações da linha de comando, por isso permanece na lista. Para deixá-lo em repouso, desligue-o com LINK; para usar outro hardware, escolha outra interface abaixo.
settings-mute-radio-s-audio = Silenciar o áudio deste rádio
settings-link = LINK
settings-close-radio-configuration-kept = Fechar este rádio (a configuração é mantida)
settings-close-tab-close-radio-station = Fechar esta aba, ou fechar o rádio na sua estação
settings-close-tab = Fechar esta aba
settings-transverters-full = { $max } é o máximo que esta tabela comporta
settings-dialling = Discando { $url }…
settings-region-note = Onde fica a estação. Define todos os planos de faixas: as bordas das faixas, os subsegmentos de CW / dados / fonia na barra da cascata, onde os skimmers escutam, as frequências de chamada oferecidas e o que conta como fora da faixa para transmissão. Na Região { $region }, isso faz com que 70 cm { $region ->
        [1] 430–440 MHz — então 446 está fora da faixa, e 40 m para em 7.200
        [2] 420–450 MHz, e 40 m vai até 7.300
       *[3] 430–450 MHz, e 80 m para em 3.900
    }. Vale imediatamente, e se aplica a todos os rádios desta estação.
settings-rx-grid-reported = As recepções por este rádio são informadas a partir de { $grid }, não a partir do seu próprio locator.
settings-rx-grid-invalid = { $grid } não é um locator Maidenhead, então nada que este rádio ouve é informado. Quatro ou seis caracteres: duas letras, dois dígitos e, opcionalmente, mais duas letras.
settings-bypassed = desviado
# $state é um de settings-bypassed, settings-use ou settings-every-stage-off.
settings-tx-audio-chain = Cadeia { $state }.
settings-gateway-via = { " " }via { $via }
settings-auto-upload-to = Enviar automaticamente cada novo QSO para { $service }
settings-tle-updated = Atualizado: { $n } { $n ->
        [one] assinatura
       *[other] assinaturas
    }: { $total } satélites.
settings-tle-updated-failed = Atualizado: { $ok } de { $n }; { $failed } falharam — veja as linhas acima.
settings-panadapter-of = Este rádio é o panadaptador de { $radio }
settings-radio-n = rádio { $n }
settings-on-station = Em { $station }
settings-add-radio-on-station = Adicionar um rádio em { $station } — a estação o atende imediatamente
settings-panadapter-receiver-for = Receptor do panadaptador para { $radio }
settings-hang-up-stays-on = Desligar. O rádio continua em { $station }.
settings-close-on-station = Fechar { $radio } em { $station }? A configuração dele continua naquela máquina; a aba aqui fecha junto, e também a de todos os outros.
settings-close-it-on = Fechar em { $station }
settings-locator-hint = locator

## Janela de Configurações, Alertas (src/app/settings/alerts.rs)

settings-alerts-audible-alerts = Alertas sonoros
settings-alerts-sound-alarm-when-decode-matters = Tocar um alarme quando uma decodificação importa
settings-alerts-plays-over-own-audio-output = Toca pela própria saída de áudio, então é ouvido mesmo quando a faixa está em um alto-falante diferente desta tela.
settings-alerts-volume = Volume
settings-alerts-output = Saída
settings-alerts-test = Testar
settings-alerts-what-sound = O que soar
settings-alerts-each-station-quiet-while-after = Cada estação fica quieta por um tempo depois de um alerta, para que uma faixa movimentada não toque a cada intervalo. Voz e Tom + voz são lidos pela voz de anúncios falados — ative-a em Configurações → UI.

## Janela de Configurações, Amplificador (src/app/settings/amp.rs)

settings-amp-amplifier = Amplificador
settings-amp-waiting-station-s-amplifier-configuration = Aguardando a configuração do amplificador da estação…
settings-amp-hobbypcb-hardrock-500-hardrock-50 = Um HobbyPCB Hardrock-500 ou Hardrock-50 na sua porta serial USB. O SDRoxide lê os seus medidores, troca a faixa, o modo e a antena dele, e controla o sintonizador a partir da janela AMP. O amplificador continua sendo acionado pela linha PTT do rádio, como antes.
settings-amp-enable = Ativar
settings-amp-model = Modelo
settings-amp-untested-panel-has-only-been = Não testado: este painel só foi testado com um Hardrock-500. Os limites de medidor deste modelo são escalonados a partir do Hardrock-500 e podem estar errados. Acompanhe o display do próprio amplificador.
settings-amp-port = Porta
settings-amp-choose = — escolha —
settings-amp-baud = Baud
settings-amp-must-match-amplifier-s-menu = Deve coincidir com o menu do amplificador. O painel de controle da HobbyPCB exige 115200.
settings-amp-follow-radio-s-band = Seguir a faixa do rádio
settings-amp-sends-transmit-frequency-amplifier-every = Envia a frequência de transmissão ao amplificador a cada troca de faixa, para que os filtros dele acompanhem o rádio sem um cabo CAT próprio. Nunca é enviado durante uma transmissão.
settings-amp-show-power-swr-tx-meter = Mostrar a potência e a ROE dele no medidor TX
settings-amp-only-where-radio-does-not = Só onde o rádio não as mede por conta própria, e só enquanto o amplificador estiver na linha.
settings-amp-off = Desligado
settings-amp-connecting = Conectando…
settings-amp-status = Estado
settings-amp-apply = {" "}APLICAR{" "}
settings-amp-save-re-open-amplifier-s = Salvar e (re)abrir a porta do amplificador

## Janela de Configurações, Controles (src/app/settings/controls.rs)

settings-controls-keyboard = Teclado
settings-controls-click-shortcut-rebind-then-press = Clique em um atalho para redefini-lo, depois pressione a combinação de teclas (Esc cancela). As associações são ignoradas enquanto você digita em um campo de texto.
settings-controls-shortcut = Atalho
settings-controls-does = Faz
settings-controls-step-mode = Passo / modo
settings-controls-accel = Acel
settings-controls-on = Ligado
settings-controls-press-key = pressione uma tecla…
settings-controls-remove-binding = Remover esta associação
settings-controls-add-shortcut = + Adicionar atalho
settings-controls-restore-defaults = Restaurar padrões
settings-controls-bind-hold-talk-space = Associar falar-segurando à Barra de espaço
settings-controls-hold-space-transmit-releasing-losing = Segure a Barra de espaço para transmitir; soltá-la — ou perder o foco da janela — desacopla
settings-controls-unkey-held-ptt-after = Desacoplar um PTT segurado após
settings-controls-backstop-against-stuck-key-controller = Proteção contra uma tecla presa ou um controle que para de responder. 0 desativa.
settings-controls-panadapter-mouse = Mouse do panadaptador
settings-controls-wheel = Roda
settings-controls-wheel-shift = Roda + Shift
settings-controls-tune-step = Passo de sintonia
settings-controls-zoom-rate = Velocidade do zoom
settings-controls-click-tune-rounding = Arredondamento do clique para sintonizar
settings-controls-invert-wheel-direction = Inverter a direção da roda
settings-controls-left-drag-tunes-well-pans = Arrastar com o botão esquerdo sintoniza além de deslocar
settings-controls-off-makes-left-drag-pan = Desligado faz o arraste com o botão esquerdo apenas deslocar a visão, como o direito.
settings-controls-scroll-digit-frequency-readout-tune = Rolar um dígito no visor de frequência para sintonizá-lo
settings-controls-both-wheel-actions-are-tune = As duas ações da roda são Sintonizar — não sobra nenhuma forma de dar zoom.
settings-controls-restore-mouse-defaults = Restaurar padrões do mouse
settings-controls-mouse-buttons = Botões do mouse
settings-controls-left-right-buttons-are-reserved = Os botões esquerdo e direito são reservados para sintonia e deslocamento; os botões do meio e extras estão livres. Um botão lateral segurado para PTT se comporta como um pedal.
settings-controls-add-mouse-button = + Adicionar botão do mouse
settings-controls-f1-always-opens-manual-even = F1 sempre abre este manual, mesmo enquanto você digita, então não pode ser redefinido.
settings-controls-midi-controller = Controlador MIDI
settings-controls-midi-controllers-need-native-app = Controladores MIDI precisam do aplicativo nativo — o cliente de navegador não tem acesso a MIDI.
settings-controls-any-class-compliant-midi-surface = Qualquer superfície MIDI compatível com a classe funciona: a roda de jog de um controlador de DJ vira um bom botão de VFO, seus pads acionam PTT e botões de faixa, seus faders controlam ganhos.
settings-controls-enable = Ativar
settings-controls-controller = Controlador
settings-controls-feedback = Retorno para
settings-controls-rescan-ports = Buscar portas novamente
settings-controls-not-connected = Não conectado.
settings-controls-move-control-see-here = Mova um controle para vê-lo aqui.
settings-controls-control = Controle
settings-controls-reads = Lê como
settings-controls-led = LED
settings-controls-move = mova-o…
settings-controls-click-then-move-control-you = Clique e depois mova o controle que você quer associar
settings-controls-speed-sensitivity-spin-faster-tune = Sensibilidade de velocidade: gire mais rápido para sintonizar mais depressa
settings-controls-send-current-value-back-light = Envia o valor atual de volta, para acender um LED ou mover um fader
settings-controls-add-midi-control = + Adicionar controle MIDI
settings-controls-clear-all = Limpar tudo
settings-controls-endless-jog-encoders-send-relative = Encoders sem fim (jog) enviam um passo relativo em vez de uma posição, em uma de três codificações que parecem iguais com pequenos movimentos. APRENDER adivinha a partir de uma volta no sentido horário; se o botão então sintonizar no sentido errado, marque “inverter”.
settings-controls-none = — nenhum —

## Janela de Configurações, Geral (src/app/settings/general.rs)

settings-general-system-default = Padrão do sistema
settings-general-remote-access = Acesso remoto
settings-general-signed-in-station = Estação conectada
settings-general-sign-out = {" "}SAIR{" "}
settings-general-sign-out-forgets-device = Esquece o login deste dispositivo. A estação vai pedir novamente na próxima conexão; esta continua aberta até você fechá-la.
settings-general-password-kept-no-cookie = Uma senha está sendo lembrada neste navegador para { $stations }, porque nenhum cookie de login foi fornecido. Estações mais antigas fazem isso; não é a mesma coisa.
settings-general-what-remote-client-browser-page = O que um cliente remoto — a página do navegador, ou outro sdroxide iniciado com --connect — precisa apresentar antes que esta estação o deixe operar. Vale no modo servidor (--server); o próximo login pega uma alteração, sem reinício.
settings-general-username = Usuário
settings-general-password = Senha
settings-general-clients-must-give-password-leaving = Os clientes precisam informar a senha. Deixar o usuário vazio é aceitável.
settings-general-clients-must-sign = Os clientes precisam fazer login.
settings-general-empty-anyone-who-can-reach = ⚠ Vazio: qualquer um que alcance a porta do servidor pode operar este rádio, no seu indicativo. Defina uma senha antes de encaminhar a porta.
settings-general-stored-clear-config-toml-like = Armazenada em texto puro no config.toml, como as outras senhas que o sdroxide guarda.
settings-general-receive-audio-gain = Ganho do áudio de recepção
settings-general-t-0-db = 0 dB
settings-general-extra-gain-everything-radio-sends =
    Ganho extra sobre tudo o que este rádio envia aos alto-falantes, além do controle de volume. Deixe em 0 dB a menos que o rádio esteja baixo com o volume no máximo: o topo do controle de volume é o áudio como chega, então ele pode diminuir um rádio mas nunca aumentá-lo, e a saída de som USB de alguns transceptores fica bem abaixo da escala total.

    Suba de 6 dB em 6 dB. Demais causa corte — o áudio é limitado na escala total em vez de deixar dar a volta, então exagerar soa áspero, não alto. As gravações são feitas antes disto e não são afetadas.
settings-general-reload-band-plan = RECARREGAR PLANO DE FAIXAS
settings-general-re-read-bandplan-json-machine = Relê o bandplan.json na máquina à qual o rádio está conectado e o aplica — bordas de faixa, subsegmentos e janelas de skimmer — sem reiniciar.
settings-general-running-station-s-own-band = Rodando com o plano de faixas próprio da estação.
settings-general-running-built-iaru-tables = Rodando com as tabelas IARU embutidas.
settings-general-settings-file = Arquivo de configurações
settings-general-settings-are-machine-radio-attached = As configurações ficam na máquina à qual o rádio está conectado. Exporte-as lá.
settings-general-export = EXPORTAR…
settings-general-write-every-setting-station-radios = Grava toda configuração desta estação — os rádios, os modos, os servidores, as memórias, o plano de faixas — em um arquivo que você pode levar para outra instalação. Seu diário de contatos e qualquer senha de servidor salva ficam aqui.
settings-general-import = IMPORTAR…
settings-general-replace-station-s-settings-ones = Substitui as configurações desta estação pelas de um arquivo exportado de outra instalação. Reinicie o sdroxide depois.
settings-general-import-overwrites-what-here-file = Uma importação sobrescreve o que está aqui, arquivo por arquivo, e tem efeito na próxima vez que o sdroxide iniciar — caso contrário, as configurações já na memória seriam gravadas de volta por cima. O que o arquivo não menciona fica como está, então um pacote de uma estação com um rádio não remove um segundo rádio daqui. Seu diário de contatos nunca está no arquivo: exporte-o como ADIF pela janela LOG se quiser levá-lo também.
settings-general-sdroxide-settings = Configurações do sdroxide
settings-general-swr-guard = Proteção de ROE
settings-general-stop-transmitting-high-swr = Parar a transmissão com ROE alta
settings-general-trip = Disparar em
settings-general-stops-transmission-when-radio-reports =
    Interrompe a transmissão quando o rádio informa uma ROE igual ou acima deste valor, e mantém a transmissão bloqueada até que você a reconheça. Pega uma antena desconectada, uma linha de alimentação com falha, ou uma chave deixada na porta errada.

    A sintonia é tratada de forma diferente, porque alimentar um descasamento é o objetivo dela: um sintonizador de antena recebe o dobro do limite e cerca de cinco segundos antes de a proteção começar a valer. Um sintonizador manual que demore mais que isso precisa ter a proteção desligada para a sessão.

    Exige um rádio que informe a ROE pelo CAT. Ignora o primeiro quinto de segundo de cada transmissão e não espera a potência alta.
settings-general-per-mode-settings = Configurações por modo
settings-general-reset-every-mode = REDEFINIR TODOS OS MODOS
settings-general-forget-every-mode-s-remembered = Esquece o AGC, o squelch, a redução de ruído, o notch e as chaves de estéreo lembrados de cada modo, e devolve os padrões próprios de cada modo.
settings-general-selecting-mode-lays-mode-s = Selecionar um modo aplica os valores iniciais próprios desse modo para AGC, squelch, redução de ruído, notch e chaves de estéreo no receptor, e alterar um deles o lembra apenas para esse modo. O chip de seta circular no fim da linha de filtro/ruído do receptor lista o que foi alterado e devolve o modo atual aos padrões.
settings-general-your-audio-speakers-microphone = Seu áudio (alto-falantes / microfone)
settings-general-output = Saída
settings-general-channels = Canais
settings-general-which-speaker-channel-s-received = Quais canais de alto-falante recebem o áudio recebido. Esquerdo ou direito coloca tudo, mixado em mono, somente naquele lado.
settings-general-input = Entrada

## Janela de Configurações, serviços de rede (src/app/settings/net.rs)

settings-net-operator = Operador
settings-net-callsign-grid-are-set-general = O indicativo e o locator são definidos na aba Geral.
settings-net-set-your-callsign-grid-general = ⚠ Defina seu indicativo e locator na aba Geral.
settings-net-freedv-reporter = FreeDV Reporter
settings-net-enable = Ativar
settings-net-connects-whenever-enabled-your-station = Conecta sempre que ativado. Sua estação só aparece para os outros enquanto o rádio estiver em RADE — em qualquer outro modo você continua conectado, mas oculto.
settings-net-station = Estação
settings-net-message = Mensagem
settings-net-receive-only-i-cannot-transmit = Somente recepção (não posso transmitir)
settings-net-server = Servidor
settings-net-host = Host
settings-net-port = Porta
settings-net-tls-wss = TLS (wss://)
settings-net-not-yet-implemented-freedv-gui = Ainda não implementado — a GUI do FreeDV também usa ws:// simples.
settings-net-reporting = Informes
settings-net-report-stations-i-decode = Informar as estações que eu decodifico
settings-net-sends-rx-report-each-callsign = Envia um rx_report para cada indicativo recuperado de um quadro End-of-Over do RADE.
settings-net-show-other-reporter-stations-spots = Mostrar outras estações reporter como spots
settings-net-adds-them-panadapter-overlay-world = Adiciona-as à sobreposição do panadaptador, ao mapa-múndi e à janela SPOTS sob o filtro FREEDV.
settings-net-set-your-callsign-grid-general-2 = ⚠ Defina seu indicativo e locator na aba Geral. Sem os dois, a conexão é apenas de visualização: você verá outras estações mas não aparecerá você mesmo.
settings-net-apply = {" "}APLICAR{" "}
settings-net-persist-re-connect = Salvar e (re)conectar
settings-net-longwave-shortwave-stations-labelled-waterfall = As estações de ondas longas e curtas identificadas na cascata. A programação de ondas curtas é baixada de eibispace.de para a temporada de radiodifusão atual e baixada de novo quando a temporada muda.
settings-net-season = Temporada
settings-net-downloading = baixando…
settings-net-using-built-copy = usando a cópia embutida
settings-net-your-own-stations-corrections-go = Suas próprias estações e correções vão neste arquivo. O sdroxide nunca o grava, e uma entrada aqui substitui uma programada com o mesmo nome e frequência.
settings-net-reload = Recarregar
settings-net-re-read-your-station-file = Relê seu arquivo de estações depois de editá-lo
settings-net-download-schedule-now = Baixar programação agora
settings-net-fetch-season-s-schedule-again = Busca de novo a programação desta temporada, substituindo a cópia em cache
settings-net-broadcast-stations-labelled-waterfall-come = As emissoras de rádio identificadas na cascata vêm da lista embutida nesta versão. Editá-las exige o aplicativo de desktop.
settings-net-ask-service-whether-these-credentials = Pergunta ao serviço se estas credenciais funcionam. Aplica primeiro as configurações acima, e não registra nada: apenas lê.
settings-net-checking = verificando…

## Janela de Configurações: mensagens com valores preenchidos.

settings-amp-connected-to = Conectado ao { $model }
settings-controls-last-midi = Última mensagem: { $message }  valor { $value }
settings-general-band-plan-file = Toda borda de faixa e subsegmento vem de { $path }, gravado a partir das tabelas IARU embutidas na primeira vez e seu para editar depois — estreite uma faixa para a sua licença e o sdroxide recusará transmitir fora dela. As frequências estão em MHz; apague o arquivo para uma cópia nova dos padrões. Isto é a alocação regional, não a sua licença: suas condições próprias podem ser mais restritas, e os planos nacionais diferem dentro de uma região.
settings-general-exported = { $note } exportado — escolha onde salvar.
settings-general-export-failed = Falha na exportação: { $error }
settings-general-import-failed = Falha na importação: { $error }
settings-general-imported = { $summary } — reinicie o sdroxide para usá-los.
settings-general-import-skipped = { $path } ignorado: { $why }
settings-general-swr-tripped = ⚠ Atualmente disparado em { $swr }:1 — a transmissão está bloqueada.
settings-net-call-grid-set-on-general = { $call } / { $grid } — definido na aba Geral
settings-net-reporting-as = Informando como { $call } / { $grid } — SDRoxide { $version }
settings-net-schedule-download-failed = ⚠ Falha no download: { $error }. Usando a programação embutida.
settings-net-test-service = Testar { $service }

## Janela de Configurações (src/app/settings/mod.rs), continuação

settings-transmit-off-while-converter-set = A transmissão fica desligada enquanto um conversor estiver definido — diga o que há na linha de transmissão acima para religá-la.
settings-transmit-goes-through-same-converter = A transmissão passa pelo mesmo conversor, então o rádio trabalha o mesmo deslocamento em relação ao dial nos dois sentidos.
settings-transmit-not-converted-radio-transmits = A transmissão não é convertida: o rádio transmite na frequência que o dial mostra, enquanto a recepção passa pelo conversor.
settings-transmit-takes-own-offset-separate = A transmissão usa um deslocamento próprio, separado do de recepção.
settings-no-enabled-subscriptions-update = Não há assinaturas ativas para atualizar.
settings-add-radio-here-station-you = Adicionar um rádio — aqui, ou em uma estação à qual você esteja conectado

## Janela de Configurações, Geral, continuação

settings-general-every-band-edge-sub-segment = Toda borda de faixa e subsegmento vem do bandplan.json na máquina à qual o rádio está conectado. Isto é a alocação regional, não a sua licença: suas condições próprias podem ser mais restritas, e os planos nacionais diferem dentro de uma região.

## Janela de Configurações, envio de imagens (src/app/settings/picup.rs)

settings-picup-received-pictures-ftp = Imagens recebidas → FTP
settings-picup-uploads-every-received-sstv-picture = Envia cada imagem SSTV recebida (e, se marcado, cada carta meteorológica) assim que é salva, organizada da forma que o QSSTV faz, para que páginas web escritas para o QSSTV funcionem sem alteração.
settings-picup-upload-received-pictures = Enviar imagens recebidas
settings-picup-server = Servidor
settings-picup-port = Porta
settings-picup-security = Segurança
settings-picup-user = Usuário
settings-picup-password = Senha
settings-picup-certificate = Certificado
settings-picup-trusted-when-computer-s-operating = Confiável quando o sistema operacional deste computador confia nele, como um navegador faria aqui. Uma CA privada pode ser adicionada abaixo (PEM).
settings-picup-extra-ca = CA extra
settings-picup-only-certificate-fingerprint-accepted-way = Só é aceito o certificado com esta impressão digital — a forma de confiar em um servidor autoassinado. TESTAR mostra a do servidor.
settings-picup-any-certificate-accepted-link-encrypted = ⚠ Qualquer certificado é aceito. A conexão é criptografada, mas qualquer um no caminho pode se passar pelo servidor e ler a senha.
settings-picup-plain-ftp-sends-password-pictures = ⚠ FTP simples envia a senha e as imagens sem criptografia.
settings-picup-sstv = SSTV
settings-picup-sstv-dir = Pasta SSTV
settings-picup-weather-fax = Fax meteorológico
settings-picup-wefax-dir = Pasta WEFAX
settings-picup-empty-login-directory = Vazio é o diretório de login.
settings-picup-file-names = Nomes de arquivo
settings-picup-format = Formato
settings-picup-add-extension = adicionar extensão
settings-picup-image1-png-rather-than-qsstv = image1.png em vez do image1 simples do QSSTV
settings-picup-caption-bar = Barra de legenda
settings-picup-time-frequency-mode-sender-s = Hora, frequência, modo e a identificação FSK do remetente no topo, como o QSSTV desenha. Só a cópia enviada é marcada.
settings-picup-fit-960-768 = Ajustar a 960 × 768
settings-picup-qsstv-s-heartbeat-tiny-picture = O heartbeat do QSSTV: uma imagem minúscula cuja altura é o minuto, enviada junto a cada uma, e feita com dois pixels de largura quando o programa é fechado.
settings-picup-nat-workaround = Correção de NAT
settings-picup-ignore-private-address-server-s = Ignora um endereço privado na resposta do modo passivo do servidor e usa o próprio — para um servidor atrás de NAT que não conhece o seu endereço público.
settings-picup-apply = {" "}APLICAR{" "}
settings-picup-test = TESTAR
settings-picup-connect-settings-above-applied-not = Conecta com as configurações acima — aplicadas ou não — faz login, e grava e apaga um pequeno arquivo em cada pasta de envio.
settings-picup-checking = verificando…
settings-picup-trust-certificate = Confiar neste certificado
settings-picup-pin-switch-pinned-fingerprint-one = Fixe-o: mude para "Impressão digital fixada" com esta. Confira-a com o servidor antes — isto é o que o servidor informou, não prova de quem ele é. Depois, APLICAR.
settings-picup-last-upload = Último envio

## Janela de Configurações, Perfis (src/app/settings/profiles.rs)

settings-profiles-profiles-station-s-saved-setups = Perfis: as configurações salvas da estação
settings-profiles-profile-captures-whole-working-setup = Um perfil captura toda a configuração de trabalho — os dials e VFOs, o modo e os filtros, os ganhos, a excitação e as antenas, seu indicativo e modelos de mensagem, e os registros por faixa — e a aplica de volta com um clique. O hardware (back end, dispositivos de áudio, conversores) fica deliberadamente de fora: um perfil é a forma como você opera a estação, não o rádio em que ela roda.
settings-profiles-save-current-setup = Salvar a configuração atual
settings-profiles-profile-name = nome do perfil
settings-profiles-save = Salvar
settings-profiles-applies-whatever-sdroxide-attached-dials = Aplica-se ao que o sdroxide estiver conectado: dials e VFOs, modo e filtros, níveis, ganhos, excitação, antenas, a identidade digital e as pilhas de faixas.
settings-profiles-no-profiles-saved-yet-name = Nenhum perfil salvo ainda — dê um nome acima e pressione Salvar.
settings-profiles-saved-profiles = Perfis salvos
settings-profiles-apply = Aplicar

## Janela de Configurações, chave T/R (src/app/settings/relay.rs)

settings-relay-transmit-receive-switch = Chave de transmissão/recepção
settings-relay-waiting-station-s-t-r = Aguardando a configuração da chave T/R da estação…
settings-relay-closes-contact-while-station-transmits = Fecha um contato enquanto esta estação transmite — para desconectar e aterrar a entrada de antena do SDR, e para acionar um amplificador ou um relé T/R externo em sequência com ela. Funciona com as placas de relé USB baratas (LCUS, KMtronic, Numato), com uma linha serial RTS/DTR em qualquer interface que aceite um fechamento de PTT, e com qualquer outra coisa por meio de um gancho de comando.
settings-relay-hardware = Hardware
settings-relay-board = Placa
settings-relay-lcus-four-byte-a0-command = A LCUS é a placa de comando A0 de quatro bytes vendida sob uma dúzia de nomes; a KMtronic e a Numato também podem ser consultadas sobre como estão configuradas
settings-relay-port = Porta
settings-relay-choose = — escolha —
settings-relay-device = Dispositivo
settings-relay-gpio-pin = Pino GPIO
settings-relay-pin-3-every-homebrew-plan = Pino 3 em todo plano caseiro e em toda placa comercial — é o que fica no fim do encapsulamento, onde um fio pode ser soldado. Se o relé não estalar, a placa é um clone que nem implementa esses pinos.
settings-relay-gpio-chip = Chip GPIO
settings-relay-linux-only-gpiodetect-lists-chips = Somente Linux. `gpiodetect` lista os chips da máquina.
settings-relay-transmit = Na transmissão
settings-relay-receive = Na recepção
settings-relay-nothing-switched = Nada é comutado.
settings-relay-starting-program-costs-unpredictable-amount = Iniciar um programa custa um tempo imprevisível, e nada disso é contado na antecipação — então deixe uma margem maior aqui do que deixaria para uma placa de relé. Os números da tabela de contatos não significam nada para este link: seus comandos são executados uma vez ao pressionar a tecla e uma vez ao soltá-la.
settings-relay-contacts = Contatos
settings-relay-contact-1-rts-contact-2 = O contato 1 é RTS e o contato 2 é DTR.
settings-relay-no = Nº
settings-relay-gpio = GPIO
settings-relay-name = Nome
settings-relay-job = Função
settings-relay-tx-closes = TX fecha
settings-relay-lead = Antecipação
settings-relay-hold = Retenção
settings-relay-line-s-offset-chip-bcm = O deslocamento da linha neste chip — o número BCM em um conector de Raspberry Pi, que não é o número físico do pino
settings-relay-sdr-antenna = Antena do SDR
settings-relay-decides-default-timings-what-log = Define os tempos padrão e o nome que o log usa para ele — e, para um decodificador de faixa, que o contato segue a tabela de faixas abaixo
settings-relay-transmitting-energises-coil-off-transmitting =
    Ligado: transmitir energiza a bobina. Desligado: transmitir a libera.

    Isto é uma decisão de fiação antes de ser uma configuração — veja a nota abaixo.
settings-relay-how-long-before-rf-contact = Quanto tempo antes da RF este contato passa da palavra RX da sua faixa para a palavra TX. Uma resintonia o move de imediato; isto serve para o momento da tecla, quando um filtro só de transmissão precisa estar no circuito antes de a potência estar.
settings-relay-closed-long-before-rf-small = Fechado este tempo antes da RF. Um pequeno relé de coaxial comuta em 5 a 15 ms. A transmissão espera pelo maior destes, então um valor grande é uma falha audível no áudio de recepção; qualquer valor acima de 250 ms é ignorado.
settings-relay-how-long-after-rf-stops = Quanto tempo depois de a RF parar este contato volta da palavra TX para a palavra RX.
settings-relay-opened-long-after-rf-stops = Aberto este tempo depois de a RF parar. Mais longo que a antecipação de propósito: deixar a antena voltar com um momento de atraso não custa nada, e deixá-la voltar cedo custa um front end. Zero é uma resposta válida, e a certa para a linha de tecla de um amplificador.
settings-relay-test = {" "}TESTAR{" "}
settings-relay-not-while-station-air = Não enquanto a estação estiver no ar
settings-relay-close-contact-half-second-so = Fecha este contato por meio segundo, para que você possa ouvir o relé e conferir a fiação com o transmissor desligado
settings-relay-remove = Remover
settings-relay-add-contact = + adicionar um contato
settings-relay-if-will-not-answer = Se não responder
settings-relay-switch-exists-protect-receiver-not = Uma chave que existe para proteger um receptor não vale a pena se deixar de protegê-lo em silêncio — por isso recusar é o padrão. Escolha a outra opção quando um cabo USB solto encerrando o seu concurso for o pior desfecho.
settings-relay-order-timings-longest-lead-closes = A ordem são os tempos: a maior antecipação fecha primeiro, e a maior retenção abre por último. Então dê à chave da antena a antecipação e a retenção maiores, e ao amplificador as menores de cada uma — assim a antena é sempre comutada antes de o amplificador ser acionado, e o amplificador sempre é desacionado antes de a antena voltar.
settings-relay-transmit-sense-input = Entrada de detecção de transmissão
settings-relay-when-transceiver-keys-itself-own = Quando um transceptor se aciona sozinho — o próprio botão do microfone, pedal, VOX ou manipulador — o sdroxide descobre perguntando pelo CAT, e essa resposta chega algumas centenas de milissegundos depois do início da transmissão. Ligue a linha SEND do rádio, por meio de um optoisolador, a uma entrada de handshake desta mesma porta, e ela é vista em poucos milissegundos. O medidor e o intertravamento de transmissão ficam mais rápidos com isso.
settings-relay-line = Linha
settings-relay-transmitting = Transmitindo quando
settings-relay-high-line = a linha está em nível alto
settings-relay-most-opto-isolated-interfaces-pull = A maioria das interfaces optoisoladas puxa a linha para baixo quando o rádio aciona, então isto costuma ficar desligado
settings-relay-belongs-radio = Pertence ao rádio
settings-relay-which-radio-tab-sensed-transceiver = Em qual aba de rádio está o transceptor detectado. Em uma estação com um rádio, isto é 1 e nada depende disso.
settings-relay-needs-serial-port-handshake-lines = Exige uma porta serial — as linhas de handshake estão nela.
settings-relay-wiring-only-fail-safe-survives = A fiação é a única proteção de segurança que sobrevive a este programa.
settings-relay-choose-tx-closes-so-de = Escolha “TX fecha” para que o contato *desenergizado* seja o estado desejado quando a placa for desconectada, o computador estiver desligado ou o sdroxide não estiver rodando. Ligado de modo que desenergizado signifique aterrado, um relé morto deixa o SDR surdo — incômodo, e seguro. Ligado do outro jeito, um relé morto mais uma transmissão é um front end morto. Com um amplificador real atrás, escolha a primeira.
settings-relay-cannot-protect-first-instant-over = Isto não consegue proteger o primeiro instante de uma transmissão que você inicia no rádio. Ao acionar o rádio a partir do sdroxide, os contatos sempre antecedem a RF; com a entrada de detecção acima ligada, eles a seguem em milissegundos. Para um front end realmente digno de proteção, use também uma chave T/R de hardware com detecção de RF — nenhum programa em um computador pode ser a resposta completa.
settings-relay-contacts-transmit = contatos em TRANSMISSÃO
settings-relay-not-open = Não aberto.
settings-relay-status-unknown-press-apply = Estado desconhecido — pressione APLICAR.
settings-relay-station-air-apply-will-take = A estação está no ar — APLICAR terá efeito depois desta transmissão.
settings-relay-band-decoder-outputs = Saídas do decodificador de faixa
settings-relay-one-control-word-per-band = Uma palavra de controle por faixa: o bit 0 é o contato 1, o bit 1 o contato 2, e assim por diante, seguindo a numeração acima. RX segue a faixa do dial de recepção; TX a faixa do rádio que acionou — comutados na antecipação de cada contato antes da RF e de volta depois da retenção. Dê o mesmo valor aos dois para um filtro que não se importa com o sentido da RF, e valores diferentes para um desvio de pré-amplificador só de recepção ou um banco de LPF só de transmissão. Faixas deixadas em 0 não afirmam nada. Vale ao Aplicar / reconectar.
settings-relay-band = Faixa
settings-relay-rx = RX
settings-relay-tx = TX
settings-relay-outputs-asserted = Saídas acionadas
settings-relay-other = Outras
settings-relay-everywhere-outside-amateur-bands-short = Em todo lugar fora das faixas de amador — escuta em ondas curtas, e qualquer coisa em que o dial de um transverter caia e que nenhuma faixa cubra.
settings-relay-apply = {" "}APLICAR{" "}
settings-relay-persist-re-open-switch = Salvar e (re)abrir a chave

## Janela de Configurações, Remoto (src/app/settings/remote.rs)

settings-remote-connect-server = Conectar a um servidor
settings-remote-drive-sdroxide-running-somewhere-else = Controla um sdroxide rodando em outro lugar — a máquina da estação, um local remoto — pela rede. Inicie-o lá com --server e informe o endereço aqui. O rádio, os decodificadores e o diário de contatos ficam nessa máquina; esta tela recebe a cascata e o áudio, e usa o seu alto-falante e microfone.
settings-remote-address = Endereço
settings-remote-host-name-ip = nome do host ou IP
settings-remote-machine-running-sdroxide-server-host = A máquina que roda sdroxide --server: um nome de host, um endereço IPv4 ou IPv6, ou uma URL ws:// (ou wss://) completa se o servidor ficar atrás de um proxy reverso — uma URL é usada exatamente como digitada e ignora a porta e a chave abaixo.
settings-remote-port = Porta
settings-remote-port-server-listens-server-port = A porta em que esse servidor escuta — server_port no config.toml dele. 4950, a menos que tenha recebido --port.
settings-remote-secure = Seguro
settings-remote-wss-tls = wss:// (TLS)
settings-remote-tick-where-server-sits-behind = Marque isto quando o servidor estiver atrás de algo que termina o HTTPS por ele — um proxy reverso na porta 443, que é a forma usual de chegar a uma estação pela internet aberta. O servidor próprio do sdroxide fala ws:// simples e não quer isto.
settings-remote-connect = {" "}CONECTAR{" "}
settings-remote-open-station-radio-tab-own = Abrir esta estação como uma aba de rádio própria
settings-remote-client-cannot-hold-second-connection = Este cliente não pode manter uma segunda conexão.
settings-remote-station-appears-tab-beside-radios =
    A estação aparece como uma aba ao lado dos rádios já abertos aqui, e a barra de abas no topo da janela alterna entre eles (⊞ coloca dois lado a lado). Feche-a pela lista no topo da aba Rádio — isso desconecta; nada no servidor é alterado.

    Se o servidor pedir usuário e senha, a tela de login dele aparece na nova aba. Um link ws:// simples não leva nada disso criptografado, então pela internet aberta ou passe por uma VPN ou um túnel SSH, ou termine o HTTPS na frente do servidor e marque Seguro acima.

## Janela de Configurações, Servidores (src/app/settings/servers.rs)

settings-servers-wsjt-x-udp-broadcast = Transmissão UDP do WSJT-X
settings-servers-waiting-station-s-wsjt-x = Aguardando a configuração de transmissão do WSJT-X da estação…
settings-servers-sends-decodes-station-status-logged = Envia decodificações, estado da estação e QSOs registrados da mesma forma que o WSJT-X, para que GridTracker, JTAlert, N1MM+ e Log4OM funcionem com o sdroxide sem alteração. Somente saída — nada neste socket pode tocar no rádio.
settings-servers-enable = Ativar
settings-servers-send = Enviar para
settings-servers-t-127-0-0-1-reaches = 127.0.0.1 alcança os clientes desta máquina; um endereço da LAN ou um grupo multicast (224.0.0.1) alcança os outros
settings-servers-port = Porta
settings-servers-t-2237-port-every-client-defaults = 2237 é a porta que todo cliente usa por padrão
settings-servers-identify = Identificar-se como
settings-servers-name-clients-see-some-loggers = O nome que os clientes veem. Alguns loggers só aceitam tráfego que se identifique como WSJT-X.
settings-servers-n1mm-contactinfo-broadcast = Transmissão contactinfo do N1MM+
settings-servers-same-news-n1mm-s-words = As mesmas notícias nas palavras do N1MM: um datagrama XML por contato registrado, para os loggers que escutam o N1MM em vez do WSJT-X — a ponte da World Radio League aceita qualquer um. Somente contatos; não há fluxo de decodificações neste protocolo. Tem chave e porta próprias, porque uma estação pode querer as duas.
settings-servers-t-127-0-0-1-reaches-2 = 127.0.0.1 alcança os clientes desta máquina. A orientação do próprio N1MM para uma rede de concurso é o endereço de broadcast desta sub-rede — 192.168.1.255 para uma rede 192.168.1.n — que alcança todas as posições de uma vez.
settings-servers-t-12060-port-n1mm-s-documentation = 12060 é a porta que a documentação do N1MM recomenda
settings-servers-station-name = Nome da estação
settings-servers-what-n1mm-calls-stationname-name = O que o N1MM chama de StationName: o nome do computador que enviou o pacote. Os loggers o mostram para distinguir uma posição de operação de outra.
settings-servers-decoded-cw-over-udp = CW decodificado por UDP
settings-servers-cw-panel-s-copy-plain = A cópia do painel de CW como texto simples, um datagrama UDP por trecho à medida que se fixa — para um controlador de estação, a janela de CW de um logger ou um script. Sem protocolo: um ouvinte apenas anexa o que chega (tente nc -ul 9999). Só o texto fixado é enviado, nunca o final que o decodificador ainda está revisando. Cada rádio tem a sua própria chave.
settings-servers-t-127-0-0-1-reaches-3 = 127.0.0.1 alcança os programas desta máquina; um endereço da LAN ou o de broadcast desta sub-rede alcança os outros.
settings-servers-t-9999-default = 9999 por padrão
settings-servers-apply = {" "}APLICAR{" "}
settings-servers-persist-re-open-broadcast-sockets = Salvar e (re)abrir os sockets de transmissão
settings-servers-hamlib-rigctld-server = Servidor rigctld do Hamlib
settings-servers-waiting-station-s-rigctld-configuration = Aguardando a configuração do rigctld da estação…
settings-servers-listen = Escutar em
settings-servers-t-127-0-0-1-machine = 127.0.0.1 (esta máquina)
settings-servers-t-0-0-0-0-whole = 0.0.0.0 (rede inteira)
settings-servers-t-4532-port-every-rigctld-client = 4532 é a porta que todo cliente rigctld assume
settings-servers-rig-name = Nome do equipamento
settings-servers-reported-clients = informado aos clientes
settings-servers-max-clients = Máximo de clientes
settings-servers-allow-clients-transmit = Permitir que os clientes transmitam
settings-servers-off-refuses-every-key-request = Desligado recusa todo pedido de tecla e deixa de anunciar uma faixa de transmissão, então o próprio Hamlib se recusa a acionar.
settings-servers-rigctld-has-no-authentication-0 = ⚠ O rigctld não tem autenticação. Em 0.0.0.0, qualquer um que alcance esta porta pode sintonizar e acionar o seu transmissor.
settings-servers-t-1-client = 1 cliente
settings-servers-not-running = Não está rodando.
settings-servers-status-unknown-press-apply = Estado desconhecido — pressione APLICAR.
settings-servers-persist-re-bind-server = Salvar e (re)vincular o servidor
settings-servers-lets-any-hamlib-capable-program = Permite que qualquer programa compatível com Hamlib controle este rádio: frequência, modo, PTT, split, RIT e potência. No WSJT-X, fldigi ou CQRLOG escolha o equipamento “Hamlib NET rigctl” (modelo 2) e aponte para este endereço; no GPredict e no N1MM informe o host e a porta diretamente. Ao contrário do servidor TCI, carrega somente controle — sem áudio, sem IQ.
settings-servers-antenna-rotator = Rotor de antena
settings-servers-waiting-station-s-rotator-configuration = Aguardando a configuração do rotor da estação…
settings-servers-points-motorized-antenna-satellite-you = Aponta uma antena motorizada para o satélite em que você está travado, por meio de um daemon rotctld do Hamlib (rode, por exemplo, “rotctld -m 603 -r /dev/ttyUSB0” para um rotor GS-232B, ou “rotctld -m 1” para testar sem hardware) — um só daemon cobre todo rotor que o Hamlib controla — ou por meio do PSTRotator, com UDP Control ativado na configuração dele.
settings-servers-program = Programa
settings-servers-host = Host
settings-servers-t-4533-rotctld-s-default-pstrotator = 4533 é o padrão do rotctld. A porta de controle UDP do PSTRotator é 12000 por padrão, e ele responde a consultas de posição na porta seguinte, 12001 — que precisa estar livre nesta máquina.
settings-servers-min-elevation = Elevação mínima
settings-servers-below-rotator-parks-instead-tracking = Abaixo disto o rotor estaciona em vez de rastrear — defina-o para o seu horizonte local ou a linha do telhado
settings-servers-azimuth-offset = Deslocamento de azimute
settings-servers-added-every-command-rotator-whose = Somado a cada comando, para um rotor cujo norte está desalinhado
settings-servers-min-movement = Movimento mínimo
settings-servers-steps-smaller-than-are-not = Passos menores que isto não são enviados — perseguir décimos de grau gasta os motores à toa
settings-servers-park-position = Posição de estacionamento
settings-servers-drive-position-when-idle = levar a uma posição quando ocioso
settings-servers-off-leaves-antenna-wherever-last = Desligado deixa a antena onde a última passagem terminou
settings-servers-not-connected = Não conectado.
settings-servers-persist-re-connect-daemon = Salvar e (re)conectar ao daemon
settings-servers-waiting-station-s-tci-server = Aguardando a configuração do servidor TCI da estação…
settings-servers-built-tci-server = Servidor TCI embutido
settings-servers-device-name = Nome do dispositivo
settings-servers-off-leaves-control-receive-streams = Desligado mantém o controle e os fluxos de recepção funcionando, mas todo pedido de tecla é recusado.
settings-servers-tci-has-no-authentication-0 = ⚠ O TCI não tem autenticação. Em 0.0.0.0, qualquer um que alcance esta porta pode sintonizar e acionar o seu transmissor.
settings-servers-tci-client = Cliente TCI
settings-servers-when-you-tune-send-radio = Ao sintonizar, envia o VFO deste rádio a outro servidor TCI para que esse equipamento acompanhe. Somente controle — sem áudio, sem IQ, e não aciona o transmissor.
settings-servers-server = Servidor
settings-servers-same-address-server-above-client = É o mesmo endereço do servidor acima. O cliente só conecta quando esse servidor não é o que está escutando — deixe-o desligado, ou em outra porta, quando o 590Ctrl estiver usando esta porta.
settings-servers-tci-client-connected = ● Cliente TCI conectado
settings-servers-tci-client-connecting = Cliente TCI conectando…
settings-servers-tci-client-press-apply = Cliente TCI — pressione APLICAR.
settings-servers-persist-re-bind-server-connect = Salvar e (re)vincular o servidor, e conectar o cliente TCI
settings-servers-server-lets-tci-capable-programs = O servidor permite que programas compatíveis com TCI controlem este rádio: controle de frequência e modo, IQ de banda larga, áudio de recepção e áudio de transmissão. No WSJT-X escolha o tipo de equipamento SunSDR (TCI), aponte para este endereço e defina o PTT como TCI. O Cliente TCI é o sentido oposto: o dial do sdroxide é enviado como comando de VFO ao 590Ctrl (ou a qualquer servidor TCI) no endereço acima. Não passa áudio.

## Janela de Configurações, TLE (src/app/settings/tle.rs)

settings-tle-satellites-element-sets-frequencies = Satélites: conjuntos de elementos e frequências
settings-tle-waiting-station-s-satellite-configuration = Aguardando a configuração de satélites da estação…
settings-tle-tracker-already-fetches-celestrak-s = O rastreador já busca por conta própria o grupo de radioamador do CelesTrak. Isto serve para todo o resto: os satélites meteorológicos da NOAA, um cubesat novo demais para estar no grupo, ou um conjunto de elementos mais recente que o que chegou. As listas são buscadas — e mantidas — onde o motor de rádio roda, então é o mesmo conjunto de satélites onde quer que o aplicativo esteja aberto.
settings-tle-subscriptions = Assinaturas
settings-tle-listings-fetched-kept-current-same = Listas buscadas e mantidas atualizadas, no mesmo ciclo de seis horas do conjunto de radioamador. Atualizadas enquanto a janela solar estiver aberta, e por ATUALIZAR AGORA aqui.
settings-tle-fetch-track-listing = Buscar e rastrear esta lista
settings-tle-remove-subscription = Remover esta assinatura
settings-tle-orbits = Órbitas
settings-tle-which-satellites-listing-get-orbit = Quais satélites desta lista recebem um anel de órbita e um rótulo. Um grupo inteiro pede “curado”: noventa anéis de uma vez é ilegível, e nenhum deixa noventa pontos anônimos.
settings-tle-nothing-listing-sdroxide-s-curated = Nada nesta lista está na lista curada do sdroxide — essa lista tem dez satélites de radioamador, então isto se comportaria exatamente como “nenhum”.
settings-tle-all-satellites = todos os satélites
settings-tle-catalogue-numbers-keep-comma-separated = Números de catálogo a manter, separados por vírgula. Vazio rastreia tudo o que a lista traz.
settings-tle-not-fetched-yet = ainda não buscado
settings-tle-subscription = + Assinatura
settings-tle-update-now = {" "}ATUALIZAR AGORA{" "}
settings-tle-ask-radio-engine-fetch-every = Pede ao motor de rádio que busque agora todas as assinaturas ativas
settings-tle-celestrak-groups = Grupos do CelesTrak
settings-tle-pasted-element-sets = Conjuntos de elementos colados
settings-tle-one-off-these-do-not = Para uso pontual. Estes não se atualizam sozinhos, e o SGP4 deixa de propagar um conjunto de elementos quando ele passa de uma quinzena da sua época — assine em vez disso o que pretende manter.
settings-tle-track-one = Rastrear este
settings-tle-show-two-element-lines = Mostrar as duas linhas de elementos
settings-tle-remove = Remover
settings-tle-paste-two-three-line-element = Cole aqui conjuntos de elementos de duas ou três linhas
settings-tle-add-pasted = + Adicionar colado
settings-tle-nothing-paste-box-looked-like = Nada na caixa de colagem parecia um conjunto de elementos.
settings-tle-clear-box = Limpar caixa
settings-tle-frequencies = Frequências
settings-tle-shown-under-pass-table-solar = Mostrado sob a tabela de passagens na janela solar. Uma entrada aqui substitui por completo a embutida para aquele número de catálogo, então parta de uma cópia dela, a menos que pretenda descartar o restante.
settings-tle-remove-satellite-s-entry = Remover a entrada deste satélite
settings-tle-link = ENLACE
settings-tle-downlink = DESCIDA
settings-tle-uplink = SUBIDA
settings-tle-mode = MODO
settings-tle-note = OBS
settings-tle-fm-repeater = Repetidora FM
settings-tle-fm = FM
settings-tle-ctcss-67-0-hz = CTCSS 67,0 Hz
settings-tle-link-2 = + Enlace
settings-tle-copy-built = Copiar embutido
settings-tle-frequency-either-one-number-145 = Uma frequência é um número só (145.800) ou uma faixa de passagem de transponder escrita 145.950-145.970. Deixe uma direção em branco para uma baliza.
settings-tle-norad = NORAD
settings-tle-satellite = + Satélite
settings-tle-frequency-entry-needs-satellite-s = Uma entrada de frequência exige o número de catálogo NORAD do satélite.

## Janela de Configurações, Wavelog (src/app/settings/wavelog.rs)

settings-wavelog-address = Endereço
settings-wavelog-api-key = Chave de API
settings-wavelog-your-wavelog-s-front-page = O endereço da página inicial do seu Wavelog — https://log.example.org ou http://192.168.1.10/wavelog — e uma chave da página de API dele. Enviar exige uma chave de leitura e escrita.
settings-wavelog-plain-http-address-internet-api = http:// simples para um endereço na internet: a chave de API vai sem criptografia. Tudo bem dentro da sua própria rede; use https:// pela internet.
settings-wavelog-trusted-cert = Certificado confiável
settings-wavelog-forget-certificate = Esquecer este certificado
settings-wavelog-go-back-ordinary-checking-against = Volta à verificação comum com as autoridades certificadoras públicas — para um servidor que agora tem um certificado adequado.
settings-wavelog-server-presented-certificate-sha-256 = O servidor apresentou este certificado (SHA-256):
settings-wavelog-compare-your-server-s-e = Compare com o do seu servidor — por exemplo, `openssl x509 -noout -fingerprint -sha256 -in cert.pem` — e confie só se coincidirem. A partir daí este certificado exato é aceito e qualquer outro é recusado.
settings-wavelog-trust-certificate = Confiar neste certificado
settings-wavelog-station-location = Local da estação
settings-wavelog-none = — nenhum —
settings-wavelog-press-test-above-list-station = Pressione Testar acima para listar os locais de estação definidos no seu Wavelog.
settings-wavelog-default = Padrão
settings-wavelog-where-qsos-go-when-operating = Para onde vão os QSOs quando o perfil de operação não indica um local.
settings-wavelog-profile = Este perfil
settings-wavelog-use-default = Usar o padrão
settings-wavelog-not-set = não definido
settings-wavelog-no-station-location-chosen-uploads = Nenhum local de estação escolhido — os envios falharão até que um seja.
settings-wavelog-qsos-not-wavelog = QSOs que não estão no Wavelog
settings-wavelog-every-qso-logbook-wavelog = Todo QSO do diário está no Wavelog.
settings-wavelog-everything-logbook-wavelog-has-not = Tudo no diário que o Wavelog não confirmou — inclusive contatos anteriores ao envio automático. Os que ele já tem voltam como duplicados e são simplesmente marcados como enviados.
settings-wavelog-set-address-key-station-location = Defina primeiro o endereço, a chave e um local de estação.
settings-wavelog-yes-send-them = Sim, enviar
settings-wavelog-cancel = Cancelar
settings-wavelog-download-into-logbook = Baixar para este diário
settings-wavelog-downloading-runs-where-radio-attached = O download roda onde o rádio está conectado, para o diário daquela máquina — configure-o lá.
settings-wavelog-download-qsos-start-every = Baixar QSOs ao iniciar e a cada
settings-wavelog-downloaded-qsos-join-logbook-so = Os QSOs baixados entram no diário, então o FT8 e as listas de spots sabem o que você já trabalhou por outros programas e rádios. Cada download busca só o que é novo.
settings-wavelog-press-test-above-choose-which = Pressione Testar acima para escolher quais locais de estação baixar.
settings-wavelog-sync-now = Sincronizar agora
settings-wavelog-fetch-what-wavelog-has-added = Busca o que o Wavelog acrescentou desde o último download. Aplica primeiro as configurações acima.
settings-wavelog-full-resync = Ressincronização completa…
settings-wavelog-download-ticked-locations-whole-compare = Baixa inteiros os locais marcados e os compara com o diário: QSOs editados ou apagados no Wavelog, ou de um local que não está mais marcado, são postos de acordo. Você vê as mudanças antes de qualquer coisa acontecer.
settings-wavelog-full-resync-changes-make = Ressincronização completa — alterações a fazer
settings-wavelog-qsos-logged-here-imported-file = QSOs registrados aqui ou importados de um arquivo nunca são alterados nem removidos — apenas os que foram baixados do Wavelog.
settings-wavelog-apply = Aplicar
settings-wavelog-discard = Descartar
settings-wavelog-live-radio-status = Estado do rádio ao vivo
settings-wavelog-send-frequency-mode-wavelog-they = Envia frequência e modo ao Wavelog conforme mudam
settings-wavelog-radio-name = Nome do rádio
settings-wavelog-pick-radio-wavelog-s-live = Escolha este rádio no registro ao vivo do Wavelog e o formulário acompanha o dial. Exige uma chave de leitura e escrita. Somente a partir do rádio principal da estação.

## Janela de Configurações, envio de imagens: mensagens com valores

settings-picup-server-fingerprint = SHA-256 do certificado do servidor: { $fingerprint }

## Janela de Configurações, Perfis: mensagens com valores

settings-profiles-apply-hint = Devolve a estação a “{ $name }”: dials, VFOs, modo, filtros, ganhos, excitação, antenas, identidade e pilhas de faixas.
settings-profiles-delete = Excluir “{ $name }”

## Janela de Configurações, chave T/R: mensagens com valores

settings-relay-not-usable = Ainda não utilizável: { $why }
settings-relay-receiving = recebendo
settings-relay-rx-tx-words = { $rx } / TX { $tx }

## Janela de Configurações, Servidores: mensagens com valores

settings-servers-n-clients = { $n ->
        [one] { $n } cliente
       *[other] { $n } clientes
    }
settings-servers-listening = ● Escutando em { $addr } — { $clients }
settings-servers-failed = Falhou: { $error }
settings-servers-rotator-at = ● Conectado — antena em { $az }° az, { $el }° el
settings-servers-tci-client-error = Cliente TCI: { $error }

## Janela de Configurações, TLE: mensagens com valores

settings-tle-subscription-age = { $count } satélites · { $age } de idade
settings-tle-subscribed = Assinado em { $name }. Pressione ATUALIZAR AGORA para buscá-lo.
settings-tle-norad-too-stale = NORAD { $norad } · { $age } de idade — velho demais para propagar
settings-tle-norad-age = NORAD { $norad } · { $age } de idade
settings-tle-norad-n = NORAD { $norad }
settings-tle-added = { $n } { $n ->
        [one] conjunto de elementos adicionado
       *[other] conjuntos de elementos adicionados
    }.
settings-tle-added-refreshed = { $added } adicionado(s) e { $r } { $r ->
        [one] conjunto de elementos atualizado
       *[other] conjuntos de elementos atualizados
    }.
settings-tle-links = { $n } { $n ->
        [one] enlace
       *[other] enlaces
    }
settings-tle-replace-links = Substituir estes enlaces pelos embutidos de { $name }

## Janela de Configurações, Wavelog: mensagens com valores

settings-wavelog-call-mismatch = O local "{ $location }" é { $station_call }, mas você registra como { $call } — o Wavelog recusará estes QSOs.
settings-wavelog-grid-mismatch = O local "{ $location }" fica em { $station_grid }, mas o seu locator é { $grid } — o Wavelog recusará estes QSOs.
settings-wavelog-location-n = local nº { $id }
settings-wavelog-saved-with-callsign = Salvo com o indicativo ({ $call }) em Configurações → Perfis, para que um perfil portátil possa registrar em um local portátil.
settings-wavelog-owed = { $n } { $n ->
        [one] QSO registrado
       *[other] QSOs registrados
    } desde que o envio automático foi ativado ainda por enviar — nova tentativa a cada 5 minutos.
settings-wavelog-send-unsent = Enviar { $n } { $n ->
        [one] QSO ainda não presente
       *[other] QSOs ainda não presentes
    } no Wavelog…
settings-wavelog-send-confirm = Enviar { $n } QSOs para { $location }? As configurações são aplicadas primeiro.
settings-wavelog-plan-add = • { $n } QSOs do Wavelog a acrescentar ao diário
settings-wavelog-plan-update = • { $n } QSOs baixados alterados no Wavelog a atualizar
settings-wavelog-plan-remove = • { $n } QSOs baixados que não estão mais no Wavelog (ou de um local desmarcado) a remover
settings-wavelog-plan-mark = • { $n } dos seus próprios QSOs a marcar como presentes no Wavelog

## Janela de Configurações, aba Rádio (src/app/settings/radio.rs)

settings-radio-antenna = Antena
settings-radio-which-socket-back-radio-using =
    Qual conector na parte traseira o rádio está usando — o próprio comando ANT dele, a mesma configuração do botão ANT do painel frontal.

    Vale imediatamente, e é a configuração do próprio rádio, não uma cópia guardada aqui: o conector em que o rádio está é lido de volta quando a sessão abre.

    A escolha é lembrada por faixa. Troque para a antena direcional em 2 m e para o fio em 40, e cada uma volta na próxima vez que o dial cruzar para aquela faixa — e na próxima vez que o sdroxide iniciar. Uma memória guardada aqui leva consigo o conector dela.
settings-radio-receive-antenna = Antena de recepção
settings-radio-radio-s-separate-receiving-antenna =
    A antena de recepção separada do rádio — RX ANT na traseira — ligada ao caminho de recepção ou desligada dele. Configuração própria, a mesma do botão RX ANT do rádio.

    A antena no conector principal continua na transmissão de qualquer forma: esta é uma entrada extra para recepção, uma alça ou uma beverage, não uma escolha de conector.

    Diferentemente da antena acima, isto *não* é lembrado aqui. O próprio rádio o mantém por faixa, então o sdroxide o lê de volta após cada troca de faixa e mostra o que o rádio informa; marcar esta caixa é a única coisa que o move.
settings-radio-receive-path = No caminho de recepção
settings-radio-radio-power = Energia do rádio
settings-radio-switch-radio-itself-off-back =
    Desliga o próprio rádio e liga-o de novo, pelo link de controle. Esta é a única liga/desliga real do programa — não a chave LINK do próprio sdroxide, que fecha a ponta do sdroxide e deixa o rádio funcionando.

    Para que o religamento alcance alguma coisa, a ponta de controle do rádio precisa ficar acordada enquanto ele está desligado. Pela rede, isso é o que o Network Control faz. Por cabo serial, é a porta CI-V do rádio, que continua alimentada pela rede em um equipamento desligado pelo painel frontal em vez de desconectado da tomada; o sdroxide envia antes do ligar o comando de despertar que a própria documentação da Icom pede.

    Desligar encerra o áudio e os medidores — não há rádio por trás deles — e o sdroxide mantém aberto o link de controle para que o religamento tenha por onde chegar.
settings-radio-on = Ligado
settings-radio-off = Desligado
settings-radio-waiting-configuration-machine-radio-attached = Aguardando a configuração da máquina à qual o rádio está conectado.
settings-radio-sound-format = Formato de som
settings-radio-invert-spectrum = Inverter o espectro
settings-radio-swap-i-q = Trocar I/Q
settings-radio-mirror-panadapter-about-tuned-frequency =
    Espelha o panadaptador em torno da frequência sintonizada, para um equipamento que traz I e Q invertidos na própria placa de som. O indício é uma cascata cheia de sinais convincentes, todos do lado errado do dial, com a SSB saindo na banda lateral oposta — trocar os dois cabos na placa de som resolveria do mesmo jeito.

    Somente recepção: a transmissão entrega ao rádio um sinal de áudio real, que não tem banda lateral para inverter.
settings-radio-i-q-sample-rate = Taxa de amostragem I/Q
settings-radio-how-fast-radio-s-i =
    Em que velocidade a placa de som I/Q do rádio é executada — e, portanto, quanta faixa o panadaptador mostra, porque um fluxo em quadratura abrange toda a sua taxa de amostragem: 48 kHz dá ±24 kHz de cada lado do dial, 192 kHz dá ±96.

    A placa tem a palavra final. Uma que não consiga a taxa escolhida aqui abre na mais próxima que consegue, e o panadaptador fica tão mais estreito — o log informa isso na inicialização quando acontece.

    Uma placa mais rápida também dá mais trabalho: a máquina tem proporcionalmente menos tempo para esvaziá-la, e uma que não acompanha descarta amostras. Isso aparece como áudio falhando enquanto a cascata continua perfeita, e também fica registrado no log. Se você notar, volte um degrau.
settings-radio-t-48-khz-24-khz = 48 kHz (±24 kHz)
settings-radio-t-96-khz-48-khz = 96 kHz (±48 kHz)
settings-radio-t-192-khz-96-khz = 192 kHz (±96 kHz)
settings-radio-t-384-khz-192-khz = 384 kHz (±192 kHz)
settings-radio-i-q-centre-offset = Deslocamento do centro I/Q
settings-radio-how-far-radio-s-i =
    Quanto a saída I/Q do rádio é centralizada acima do próprio dial, para um equipamento cujo F.I. de recepção foi deslocado de zero — em um Elecraft, MENU:RX SHFT ajustado para 8.0 em vez de NOR, o que tira o dial do pico de CC do misturador (e impede que uma forte estação SSB/AM próxima seja detectada como AM).

    Não é um conversor: o rádio continua mostrando e transmitindo na frequência real, e nada daqui é enviado a ele. Isto apenas diz onde estão as amostras na placa de som, e o fluxo é deslocado de volta para o dial à medida que chega.

    Deixe em 0, a menos que tenha ativado essa opção do menu. Se os sinais caírem a duas vezes o deslocamento de distância, o sinal está invertido.
settings-radio-iq-correction = Correção I/Q
settings-radio-cancel-mirror-image-every-signal =
    Cancela a imagem espelhada de cada sinal e o pico de CC no meio da cascata.

    A saída I/Q de um rádio são dois caminhos analógicos que nunca são exatamente iguais em ganho nem exatamente 90° afastados, e o que isso deixa é uma cópia de cada sinal refletida em torno da frequência sintonizada, geralmente 30-40 dB abaixo — forte o bastante para parecer uma estação que não existe e para ser decodificada como uma. Rádios com trimmers de balanço no painel frontal estão ajustando exatamente isto; aqui é medido a partir do ruído recebido e não precisa de ajuste.

    Deixe ligado. Desligue se estiver ouvindo AM sintonizado exatamente na portadora — a portadora é CC, então vai junto com o pico — ou para verificar se um sinal é real, observando se ele sobrevive.
settings-radio-cancel-mirror-images = Cancelar imagens espelhadas
settings-radio-dc-notch = Notch de CC
settings-radio-widen-hole-taken-out-middle =
    Alarga o buraco retirado do meio da faixa, para um rádio cujo pico central é mais largo que o deslocamento logo abaixo dele.

    0 mantém o bloqueador comum, que tem algumas dezenas de hertz de largura e remove o deslocamento sem tocar em mais nada. Aumentando, a base da faixa vai junto: um passa-altas de primeira ordem, 3 dB abaixo na frequência definida aqui e mais abaixo dela.

    Ele é centralizado onde o I/Q do rádio é centralizado, que é o dial, a menos que o deslocamento acima o tenha movido — então o que estiver sintonizado ali também sai. Um tom de CW em 600 Hz cai dentro de uma configuração de 600 Hz.
settings-radio-panadapter-bw = Largura do panadaptador
settings-radio-cat-family = Família CAT
settings-radio-rigctld-address = Endereço do rigctld
settings-radio-host-port-running-hamlib-rigctld =
    host:porta de um rigctld do Hamlib em execução — 127.0.0.1:4532 é o padrão dele, nesta máquina.

    Inicie um com, por exemplo, `rigctld -m 2028 -r /dev/ttyUSB0 -s 38400`, onde -m é o número de modelo do Hamlib para o seu rádio (`rigctl -l` os lista).

    Esta é a opção geral: alcança frequência, modo, PTT, potência, S-metro e ROE, e nada além disso. Quando uma das famílias nativas acima serve ao seu rádio, ela faz mais — manipular a tecla pelo buffer de texto do próprio equipamento, o filtro de recepção e as escalas de medidor por modelo.
settings-radio-flrig-address = Endereço do flrig
settings-radio-host-port-running-flrig-127 =
    host:porta de um flrig em execução — 127.0.0.1:12345 é o padrão dele, nesta máquina. O flrig serve XML-RPC sempre que está rodando; a porta fica em Config → Setup → Server.

    Como a opção do Hamlib, controla um daemon em vez do rádio, mas pelo driver próprio do flrig para cada modelo — em vários equipamentos, o tratamento de potência e filtro dele é o mais fiel dos dois. Alcança frequência, modo, PTT, potência de transmissão (em watts inteiros), a largura de banda de recepção, o S-metro, a ROE e a potência de saída. A tecla CW passa pela porta cwio do próprio flrig, que precisa ser configurada no flrig.
settings-radio-serial-port = Porta serial
settings-radio-select = — selecionar —
settings-radio-baud = Baud
settings-radio-data-bits = Bits de dados
settings-radio-parity = Paridade
settings-radio-stop-bits = Bits de parada
settings-radio-force-rts = Forçar RTS
settings-radio-force-dtr = Forçar DTR
settings-radio-ptt-method = Método de PTT
settings-radio-how-transmit-keyed = Como a transmissão é acionada.
settings-radio-how-transmit-keyed-network-link = Como a transmissão é acionada. Um link de rede não tem linhas de controle, então DTR e RTS não acionam nada aqui — use CAT, que pede ao daemon para acionar o rádio, ou VOX.
settings-radio-mode-control = Controle de modo
settings-radio-digimode-mode = Modo do modo digital
settings-radio-what-put-radio-mode-sdroxide =
    Em que colocar o rádio para um modo que o sdroxide modula pela placa de som. "DIGI" seleciona a posição DATA/PKT do equipamento, que pega o áudio de transmissão da entrada USB ou ACC com o processamento de voz do caminho do microfone fora dele; "USB" deixa na banda lateral comum, para um rádio já ajustado para modular SSB pela entrada de dados, ou um sem posição DATA nenhuma.

    Abrange todos os modos digitais — FT8, FT4, PSK, RTTY, SSTV e os demais — e o CW enviado como "Placa de som", e tem precedência sobre o Controle de modo nesses casos. O SSTV é o único cuja banda lateral segue a faixa, então ali significa a posição DATA nessa banda lateral: USB-D acima de 40 m, LSB-D em 40 m e abaixo.  O SSTV-FM não faz parte disso — uma portadora FM não tem banda lateral para escolher.
settings-radio-cw-keying = Acionamento de CW
settings-radio-how-cw-panel-s-keyer =
    Como o manipulador do painel CW transmite. "Manipulador do rádio" coloca o rádio em CW e lhe entrega o texto a enviar com o manipulador próprio dele. Usa a velocidade de manipulação do equipamento (definida pelo WPM do painel), e na Yaesu envia pela memória de manipulador 1, sobrescrevendo o que estava guardado nela.

    Exige o break-in ligado no rádio, que o sdroxide liga sozinho a cada mensagem nos Icom, Yaesu e Kenwood — um equipamento com break-in desligado aceita a mensagem e não transmite nada. Fica ligado depois, e um break-in completo nunca é reduzido para semi.

    "Placa de som" envia o sidetone manipulado como áudio (MCW), um tom em dial + tom — e, como um equipamento em CW ignoraria totalmente a placa de som, selecionar CW então segue a configuração de modo digital (USB, DATA ou Controlado pelo rádio) em vez de trocar o equipamento para CW. Escolha "Controlado pelo rádio" ali para equipamentos cuja posição de dados não pode ser comandada pelo CAT (o U-D de um Xiegu), e deixe o equipamento nela você mesmo, como para FT8.
settings-radio-poll-rate = Taxa de consulta
settings-radio-how-often-radio-asked-what =
    Com que frequência o rádio é perguntado sobre o que está fazendo — o dial, o modo e os medidores. Esta é a metade do link de controle que vai do rádio de volta ao sdroxide: gire o botão VFO do equipamento ou troque o modo no painel frontal e a leitura, a faixa e o panadaptador acompanham em uma consulta.

    É também todo o tráfego de controle que esta ponta gera, e em um Icom moderno esse tráfego não é grátis: a porta CI-V e a placa de som ficam atrás de um mesmo hub USB dentro do rádio, então os quadros pedidos aqui são tempo de barramento que o áudio não recebe — ouvido como falhas que parecem defeito de DSP e não são. O padrão de 2 Hz fica meio segundo atrás do botão e é silencioso o bastante para não atrapalhar o áudio. Aumente quando a porta de controle for um dispositivo próprio (um adaptador USB-serial separado, um rigctld em rede); diminua se o áudio do rádio falhar.

    Um Icom com o CI-V Transceive ligado no próprio menu não precisa de nada disso para o dial: ele informa o botão no instante em que se move, e o sdroxide suspende a consulta do dial assim que vê isso acontecer — e a retoma se o rádio mudar sem avisar.

    A taxa é a do dial. O modo vai junto somente a cada quarta consulta, já que é uma configuração que muda algumas vezes em uma noite, não algo que segue um botão.
settings-radio-send-command = Comando de envio
settings-radio-which-transceiver-generation-keys-rig =
    Qual geração de transceptor aciona o equipamento, para o método de PTT "CAT". As duas discordam sobre o que o parâmetro TX significa, e nada no fio as distingue, então escolha a que combina com o seu rádio.

    "Estilo TS-2000 (TX;)" — TS-480, TS-570, TS-870, TS-2000: o envio comum, na banda principal.

    "Estilo TS-590 (TX1;)" — TS-590S/SG, TS-890, TS-990: DATA SEND, que aciona com a entrada de áudio ACC2/USB ativa. Nesses equipamentos, o envio simples seleciona o microfone e silencia o áudio que o sdroxide transmite.

    Errado, um TS-590 transmite silêncio — mas um TS-2000 transmite na sub-faixa, que é outra faixa inteira.
settings-radio-radio = Rádio
settings-radio-k3-k3s-kx3-kx2-k4 = K3 · K3S · KX3 · KX2 · K4
settings-radio-one-profile-covers-family-k3 =
    Um perfil cobre a família: o conjunto de comandos do K3, que o KX3, o KX2 e o K4 respondem. Não há nada a escolher aqui — quantos watts o controle de Excitação abrange (12 em um KX2 ou KX3 puro, 110 com um KPA3 ou um KXPA100) é lido da própria consulta de módulos de opção do equipamento quando a porta abre.

    Atenção às taxas de baud acima: o K3, o K3S, o KX3 e o KX2 não vão além de 38400, e um equipamento ajustado abaixo da taxa escolhida aqui não responde nada.
settings-radio-transmit-input = Entrada de transmissão
settings-radio-where-radio-takes-transmit-audio =
    De onde o rádio pega o áudio de transmissão — o comando TI do equipamento, que é o menu 32 "TX IN" no painel frontal. Afirmado quando a porta abre.

    "Áudio USB" é o que faz esta interface funcionar: o FDM-DUO transmite o que o sdroxide coloca na placa de som USB dele. Um rádio deixado em "Microfone" envia a sala no lugar, e nada na tela avisa disso.

    "Automático" deixa o equipamento escolher — o microfone para um toque de PTT no microfone, a porta USB para um acionamento de CAT ou RTS.

    "Deixar como ajustado no rádio" não envia nenhum TI, para um operador que define isto no painel frontal.
settings-radio-which-two-sockets-back-receiver =
    Qual dos dois conectores da traseira o receptor escuta — o comando AN do equipamento, que é o menu 31 "ANTENNAS" no painel frontal e o indicador "ANT 1 2" no display.

    "RTX" é uma antena fazendo as duas funções, no conector tipo M que também leva a transmissão. "Somente RX" move a recepção para o segundo conector e deixa a transmissão em RTX — uma antena receptora, uma alça ou uma beverage, com a direcional ainda no transmissor.

    Vale imediatamente, e é lido de volta do rádio quando a porta abre: esta é a configuração do próprio equipamento, não uma cópia guardada aqui.
settings-radio-one-profile-covers-both-there =
    Um perfil cobre os dois. Não há nada a escolher aqui — o rádio se identifica quando a porta abre.

    Esta é só a metade CAT: comanda o dial, o modo, o PTT, o S-metro, a ROE e a potência de transmissão pela porta USB CAT do equipamento, e pega o áudio pela porta USB Audio dele. A terceira porta USB do FDM-DUO leva I/Q de banda larga, que esta interface não lê — selecione a interface ELAD acima para isso.

    Atenção à taxa de baud: o menu 70 "CAT BAUD" no rádio precisa coincidir, e ele sai de fábrica em 38400.

    O CW é acionado pela tecla ou manipulador do próprio rádio. O FDM-DUO não tem comando que aceite texto, então o painel CW não consegue acioná-lo pelo CAT.
settings-radio-one-profile-covers-range-qrp =
    Um perfil cobre a linha: o conjunto de comandos próprio da QRP Labs, que é um subconjunto do Kenwood TS-480 com bastante coisa acrescentada. Não há nada a escolher aqui — o rádio informa o nome e a versão de firmware quando a porta abre, e a versão é o que decide se a leitura de proteção de ROE é pedida.

    Não é o perfil Kenwood: em um QMX, o comando PC é o METER de potência, e não o controle de potência, então um rádio controlado como Kenwood teria a leitura do medidor escrita como se fosse uma configuração, e MD8 — um modo Kenwood — é SWR Tune aqui.

    A potência de transmissão é definida no rádio. Não há comando CAT para isso, então o controle de Excitação só alcança o nível do áudio que entra na placa de som.

    O filtro de recepção também é do rádio: ele informa a largura que o modo implica (3,2 kHz em Digi, 300 Hz em CW) e não oferece nada para alterá-la.
settings-radio-baud-rate-ignored-port-usb = A taxa de baud é ignorada — a porta é USB
settings-radio-qmx-serves-own-virtual-com =
    Um QMX serve as próprias portas COM virtuais por USB, então a taxa definida acima não tem efeito em nenhuma das pontas. Ele oferece até três delas; se o rádio não responder nada, a causa mais provável é a errada.

    Uma coisa importa de fato sobre a porta: um retorno de carro enviado a ela coloca o rádio em modo terminal pelo resto da sessão. O sdroxide nunca envia um — mas um programa de terminal deixado aberto na mesma porta enviará, e o CAT para no momento em que isso acontece.
settings-radio-i-q-mode-switched-radio = O modo I/Q é ligado no rádio
settings-radio-radio-s-sound-card-carries =
    A placa de som do rádio leva ou o áudio demodulado ou o I/Q bruto que o ADC dele enxerga, e o sdroxide afirma o que o Formato de som acima pede quando a porta abre (o comando Q9 do rádio, a entrada de menu "IQ mode").

    O deslocamento de centro e a taxa de amostragem acima foram definidos para o que um QMX precisa: o receptor dele é um superheteródino com F.I. de 12 kHz, então o sintetizador fica 12 kHz abaixo do dial e tudo na placa fica 12 kHz acima do meio da faixa. A placa roda a 48 kHz, o que torna o panadaptador com 48 kHz de largura.

    Em CW o rádio desloca esse F.I. mais uns ~700 Hz, o que um valor único não consegue acompanhar — acrescente à mão se você usa o panadaptador com o rádio em CW. Em Digi, onde um rádio usado como front end I/Q normalmente fica, não há nada a acrescentar.

    A QRP Labs observa que o modo I/Q não serve ao WSJT-X e a outros programas que esperam áudio demodulado; aqui a demodulação acontece deste lado, então serve.
settings-radio-rs-hfiq-5-w-hf = RS-HFIQ (transceptor HF de 5 W)
settings-radio-hobbypcb-s-rs-hfiq-there =
    O RS-HFIQ da HobbyPCB. Não há nada a escolher aqui: o perfil é todo o conjunto de comandos do rádio, e a versão de firmware é registrada no log quando a porta abre.

    O link de controle carrega duas coisas e não mais que isso — onde colocar o oscilador, e se deve transmitir. Não há comando de modo, potência, filtro, squelch ou medidor, porque o rádio não tem nenhum deles. Todo o resto é do sdroxide: o que sai da placa de som é banda base complexa centralizada no dial, então modo, filtro e modulação acontecem deste lado.

    O comando de frequência cobre 3–30 MHz. Peça algo fora disso e o rádio recusa e fica onde está; o log informa.

    O CW é acionado como áudio pela cadeia de transmissão — este rádio não tem manipulador que aceite texto. (Ele tem um gerador de CW interno, que o sdroxide nunca usa, por recomendação do próprio firmware.)
settings-radio-sound-format-baud-ptt-are = Formato de som, taxa de baud e PTT são do rádio, não configurações
settings-radio-all-three-have-been-filled =
    Os três foram preenchidos, e nenhum deles é preferência:

    • O formato de som é I/Q (estéreo) — a placa leva banda base complexa e nada mais.
    • 57600 baud, 8N1 — a taxa única do firmware, sem menu para alterá-la. Qualquer outra gera um link mudo, então ela é fixada quando a porta abre.
    • PTT por CAT — o comando *X é a única chave de transmissão que a interface tem.

    O deslocamento de centro foi definido como zero: o sintetizador roda a quatro vezes o dial dentro de um detector em quadratura, então o meio da faixa é o dial. Defina a taxa de amostragem acima para aquela em que a sua placa de som realmente roda; é isso que torna o panadaptador da largura que tem.
settings-radio-dl2man-pe1nnz-s-pocket-qrp =
    O transceptor QRP de bolso de DL2MAN/PE1NNZ e o firmware aberto uSDX de onde ele surgiu. Ele emula um Kenwood TS-480, mas o subconjunto é magro: o dial, o modo, o PTT e a limpeza de RIT/XIT são tudo o que ele responde. Sem S-metro, sem ROE, sem controle de potência, sem VFO B, sem split e sem manipulador CAT, então o controle de Excitação e os medidores não fazem nada pelo CAT — o nível de áudio é o único controle de transmissão que existe.

    O CW é acionado como áudio (MCW) pela banda lateral, ou com a tecla ou manipulador no rádio; não há comando que aceite texto.
settings-radio-audio = Áudio
settings-radio-tr-usdx-has-no-sound =
    Um (tr)uSDX não tem placa de som própria e tem duas formas de ser ouvido, e qual é a certa depende do seu shack, não do rádio.

    "Um cabo" leva o áudio dentro do próprio link serial CAT — o fluxo de 8 bits do próprio rádio, pelo mesmo cabo USB do controle, sem nada mais para conectar. O porém é o firmware: ele não consegue aceitar um comando CAT enquanto o fluxo está rodando (um comando enviado para dentro do fluxo não o pausa, o mata), então este modo nunca consulta. Sintonia e mudanças de modo são enviadas à medida que você as faz, cada uma pausando o fluxo por um instante e reiniciando-o, então espere uma pequena falha em cada mudança; nada é enviado enquanto você apenas ouve. O dial e o modo do próprio rádio também não são seguidos — não há leitura para segui-los —, então comande-o daqui.

    "Placa de som USB" pega o áudio de uma placa de som externa ligada ao conector de alto-falante/microfone de 3,5 mm do rádio, e se comporta como qualquer outro rádio CAT: a consulta do dial roda, então o botão e o modo do próprio rádio são seguidos. Escolha a placa em Áudio do rádio abaixo.

    Os dois modos abrem a porta com DTR em nível alto: o DTR do adaptador serial está ligado ao reset do rádio, então abrir a porta o reinicia, e o primeiro segundo ou dois fica silencioso enquanto ele sobe. O DTR nunca é usado para acionar.
settings-radio-radio-model = Modelo do rádio
settings-radio-which-icom-two-things-ci =
    Qual Icom, pelas duas coisas que o CI-V não faz da mesma forma em todos.

    O endereço do transceptor abaixo é preenchido a partir dele: cada modelo traz um diferente, e um quadro enviado para o endereço errado é simplesmente ignorado — um rádio que não responde nada, sem nenhum erro em lugar algum que diga por quê.

    Isso também decide se o sdroxide consegue selecionar o modo DATA. No CI-V, USB e USB-DATA são o mesmo byte de modo, e um comando separado os distingue; sem ele, um envio de modo digital sai pela entrada do microfone, com o processamento de voz e o filtro de SSB do rádio no caminho.

    "Outro" deixa o endereço por sua conta e não envia nenhum comando de modo DATA.
settings-radio-radio-id-hex = ID do rádio (hex)
settings-radio-show-radio-s-spectrum-scope = Mostrar o scope de espectro do rádio
settings-radio-scope-span = Largura do scope
settings-radio-how-wide-sweep-radio-keeps = Quão larga é a varredura. O rádio mantém a largura escolhida por último na própria tela — muitas vezes alguns kHz. Definir uma largura aqui também coloca o scope em modo de centro, para que ele siga o dial. Isso altera também a tela do próprio rádio; "Como ajustado no rádio" a deixa intacta.
settings-radio-press-apply-reconnect-switch-without = Pressione "Aplicar / reconectar" para trocar sem reiniciar.
settings-radio-radio-audio-sound-card = Áudio do rádio (placa de som)
settings-radio-waiting-sound-cards-machine-radio = Aguardando as placas de som da máquina onde o rádio está conectado.
settings-radio-radio-rx = Do rádio (RX)
settings-radio-radio-tx = Para o rádio (TX)
settings-radio-apply-reconnect = Aplicar / reconectar
settings-radio-reopen-cat-rig-these-sound = Reabrir o equipamento CAT com estas placas de som — sem reiniciar
settings-radio-reconnects-radio-without-restarting = Reconecta o rádio sem reiniciar.
settings-radio-receive-radio-pc = Receber (rádio → PC)
settings-radio-sound-card-radio-s-own = A placa de som por onde entra o áudio próprio do rádio — a saída de fone dele, ligada à entrada de linha ou de microfone do computador. Tudo o que está no panadaptador e nos decodificadores chega aqui.
settings-radio-transmit-pc-radio-mic = Transmitir (PC → microfone do rádio)
settings-radio-sound-card-carries-audio-radio = A placa de som que leva o áudio que o rádio precisa transmitir — a saída do computador no conector de microfone do rádio. O rádio se aciona sozinho, por VOX, no instante em que o áudio chega aqui, então este dispositivo fica silencioso a menos que o rádio esteja transmitindo.
settings-radio-radio-has-no-control-cable = Este rádio não tem cabo de controle — ele se aciona por VOX, então não há PTT a comandar. A transmissão é áudio para o microfone do rádio.
settings-radio-reopen-radio-these-sound-cards = Reabrir o rádio com estas placas de som — sem reiniciar
settings-radio-devices = Dispositivos
settings-radio-discover = Descobrir
settings-radio-no-devices-press-discover = nenhum dispositivo — pressione Descobrir
settings-radio-manual-ip = IP manual
settings-radio-optional-e-g-192-168 = opcional, ex.: 192.168.1.50
settings-radio-sample-rate = Taxa de amostragem
settings-radio-receiver-ddc = Receptor (DDC)
settings-radio-protocol-2-board-carries-several = Uma placa Protocol 2 leva vários receptores sintonizáveis de forma independente (DDCs) em uma mesma conexão — rode este rádio no DDC1 e outro rádio, no mesmo endereço, no DDC2. O transmissor pertence ao rádio do DDC1. Placas Protocol 1 têm só o DDC1.
settings-radio-lna-gain = Ganho do LNA
settings-radio-front-end-gain-hermes-lite = Ganho do front end de uma Hermes-Lite 2. Tem efeito imediatamente — sem reconectar — e é lembrado como o nível em que o rádio começa. Alto demais satura o ADC e a faixa inteira parece distorcida; baixo demais e o receptor fica surdo.
settings-radio-overload-protection = Proteção contra sobrecarga
settings-radio-back-lna-gain-off-itself =
    Reduz o ganho do LNA por conta própria enquanto a placa informa que o ADC está estourando, e o eleva de novo quando isso para.

    A Hermes-Lite 2 amostra os 0–38 MHz inteiros em um único conversor de 12 bits, sem misturador nem pré-seletor à frente, então uma estação de radiodifusão a uma faixa de distância pode levá-la a estourar enquanto a faixa que você olha não mostra nada de errado — o piso de ruído sobe, tudo intermodula e os decodificadores param. A placa sabe disso, e isto é o que age sobre a informação.

    Desligado por padrão: ele move um controle que você definiu. Nada acontece durante a transmissão.
settings-radio-wind-gain-back-when-adc = Reduzir o ganho quando o ADC estourar
settings-radio-applies-apply-reconnect-main-window = Vale em Aplicar / reconectar. A barra de Ganho da janela principal mostra o ganho que a placa está de fato executando, então acompanha o laço e você pode ver o que ele faz; o controle deslizante acima é o nível em que o rádio começa, e o laço não o reescreve.
settings-radio-step = {"  "}Passo
settings-radio-how-far-gain-moves-each = Quanto o ganho se move a cada vez, em dB. Um é o passo do próprio registro de ganho da placa.
settings-radio-attack-decay = {"  "}Ataque / decaimento
settings-radio-how-often-gain-may-come =
    Com que frequência o ganho pode descer enquanto o conversor está estourando, e com que frequência pode subir de novo depois que parou.

    Os dois são deliberadamente cem vezes distantes. Recuar imediatamente: cada milissegundo de estouro é um receptor cheio de intermodulação. Voltar devagar: o que causou o problema — um vizinho acionando, uma emissora entrando ao anoitecer — geralmente não foi embora, e um laço que se recuperasse tão rápido quanto recuou passaria a noite oscilando em torno do limiar. 100 ms e 10 s por decibel é o que o Auto S-Att do PowerSDR e o HermesIntf do N1GP usam.
settings-radio-range = {"  "}Faixa
settings-radio-lowest-highest-gain-loop-may = O menor e o maior ganho que o laço pode usar, em dB. O teto impede que ele decida quão sensível deve ser o seu receptor; o piso é onde você define que, abaixo de certo ponto, a sobrecarga é problema de outro e a solução é um filtro, não mais vinte decibéis.
settings-radio-filter-board = Placa de filtro
settings-radio-accessory-board-hermes-lite-2 = Placa de acessórios no conector J16 da Hermes-Lite 2 (ou nas saídas de coletor aberto de qualquer outra placa openHPSDR — inclusive Protocol 2). "N2ADR" escolhe um relé por faixa; "Código de banda Alex / Hermes" coloca o número da faixa nas saídas 1–4, que é o que a placa Alex de um ANAN, um Zeus SDR, um HiQSDR e o Quisk esperam. "Personalizado" abre uma tabela abaixo em que você declara as sete saídas por conta própria, faixa a faixa, na recepção e na transmissão — para uma chave de antena, o decodificador de faixa de um amplificador ou qualquer outra coisa que nenhuma das duas predefinições cubra; qualquer uma das predefinições pode ser despejada nela como ponto de partida. Deixe isto em "Nenhum" a menos que algo esteja realmente instalado: esses sete pinos são saídas de coletor aberto de uso geral, e os operadores também os ligam ao PTT de amplificador, a relés de antena e à comutação de transverters. Acioná-los a partir dos dados de faixa começaria a operar o que estiver conectado. Vale em Aplicar / reconectar.
settings-radio-io-board-rx-input = Entrada RX da placa IO
settings-radio-where-n2adr-hl2ioboard-takes-receive = De onde uma HL2IOBoard da N2ADR pega o sinal de recepção. A placa em si é encontrada automaticamente e não precisa de configuração; esta existe apenas para operadores que ligaram os próprios conectores SMA dela. Deixe em "Entrada própria do rádio" caso contrário — selecionar o J9 da placa IO sem nada conectado a ele deixa o receptor surdo. Vale em Aplicar / reconectar. Com qualquer uma das opções de J9, as entradas também aparecem no controle ANT e são lembradas por faixa.
settings-radio-power-amplifier = Amplificador de potência
settings-radio-use-hermes-lite-2-s = Usar o PA embutido da Hermes-Lite 2
settings-radio-default-what-you-want-unless = Ligado por padrão, e é o que você quer, a menos que um amplificador externo seja acionado pela saída de baixa potência RF1 da placa. Com ele desligado, o rádio ainda aciona — o relé T/R comuta e qualquer placa de acessórios acompanha —, mas o conector de antena não produz potência nenhuma, e o relé fica deliberadamente em recepção. Ignorado em placas que não são Hermes-Lite.
settings-radio-mirror-board-s-spectrum-about = Espelha o espectro da placa em torno da frequência sintonizada, na transmissão e na recepção. Ligado por padrão: uma Hermes-Lite 2 precisa disso. Desligue apenas se os sinais aparecerem do lado errado do dial e nada decodificar — o indício é uma cascata cheia de traços convincentes enquanto a SSB cai na banda lateral errada e o FT8 não devolve decodificação nenhuma.
settings-radio-frequency-correction = Correção de frequência
settings-radio-crystal-tcxo-error-ppm-applied = Erro do cristal/TCXO em ppm, aplicado a RX e TX. Vale imediatamente.
settings-radio-ppm = {" "}ppm
settings-radio-transmit-buffer = Buffer de transmissão
settings-radio-how-far-ahead-real-time = Quanto à frente do tempo real o áudio de transmissão é enviado à placa pela rede, antes de o sdroxide desacelerar para acompanhar. A própria placa não guarda buffer nenhum — isto apenas amplia a margem do próprio sdroxide. Aumente em um link WiFi ou VPN, onde o padrão baixo (adequado para uma conexão cabeada direta) não dá folga suficiente contra o jitter e o áudio transmitido ou o PTT travam; um valor maior custa latência na transmissão. Tem efeito em APLICAR, que reconecta à placa.
settings-radio-puresignal = PureSignal
settings-radio-adaptive-predistortion-linearise-transmitter-sample = Pré-distorção adaptativa: linearizar o transmissor a partir de uma amostra do que ele realmente emitiu, para umas vinte e poucas decibéis a menos de intermodulação na mesma potência. O receptor é o caminho de realimentação — a placa continua recebendo durante uma transmissão —, então um acoplador direcional e um atenuador precisam levar uma amostra da saída do amplificador a uma entrada que a chave T/R não retire na transmissão. Em uma Hermes-Lite 2, isso significa o conector PureSignal da placa IO, com a entrada de recepção acima ajustada para combinar; em uma placa sem ele, a porta RX de um ANAN, ou um relé externo que liga o acoplador durante toda a transmissão. É este receptor que é lido — uma placa com um segundo ADC não pode usar o RX2 como tomada de realimentação, por mais que o firmware dela o roteie. Sem nada acoplado, o laço nunca trava e o transmissor fica exatamente como estaria. Vale em Aplicar / reconectar, e só no rádio que é dono do transmissor (DDC1).
settings-radio-correct-transmitter-receiver-s-own = Corrigir o transmissor a partir da realimentação do próprio receptor
settings-radio-table-steps = {"  "}Passos da tabela
settings-radio-how-finely-amplifier-s-curve = Com que precisão a curva do amplificador é modelada. Mais passos seguem um joelho mais agudo e levam mais tempo para preencher; 32 é um bom começo.
settings-radio-adaptation = {"  "}Adaptação
settings-radio-how-fast-correction-follows-what = Com que rapidez a correção acompanha o que o acoplador informa. Lento é o certo — isto faz a média da curva de um amplificador, que não se move, a partir de um caminho de realimentação que tem ruído.
settings-radio-hold = Manter
settings-radio-stop-adapting-keep-correction-where = Para de adaptar e mantém a correção onde está — para medir, e para um operador satisfeito com o que ela aprendeu.
settings-radio-manual-ip-overrides-discovery-press = Um IP manual tem precedência sobre a descoberta. Pressione "Aplicar / reconectar" para trocar sem reiniciar.
settings-radio-open-collector-outputs-band = Saídas de coletor aberto por faixa
settings-radio-one-control-word-per-band = Uma palavra de controle por faixa, como a documentação do seu hardware a descreve: o bit 0 é a saída 1, o bit 6 é a saída 7. RX é afirmado durante a recepção nessa faixa, e TX enquanto o transmissor está acionado — dê o mesmo valor aos dois para um filtro, e valores diferentes para algo que pertença a um só lado da comutação (a linha de tecla de um amplificador, o desvio de um pré-amplificador de recepção). Faixas deixadas em 00 não afirmam nada. Vale em Aplicar / reconectar.
settings-radio-alex-band-code = CÓDIGO DE BANDA ALEX
settings-radio-replace-table-what-preset-would = Substitui a tabela pelo que esta predefinição enviaria em cada faixa, como ponto de partida para editar. Nada mais na configuração muda.
settings-radio-clear = LIMPAR
settings-radio-every-output-off-every-band = Cada saída desligada em cada faixa, que é o que "Nenhum" faz.
settings-radio-band = Faixa
settings-radio-rx = RX
settings-radio-tx = TX
settings-radio-outputs-asserted = Saídas afirmadas
settings-radio-other = Outras
settings-radio-everywhere-outside-amateur-bands-short = Em todo lugar fora das faixas de amador — escuta em ondas curtas, e qualquer coisa em que o dial de um transverter caia e que nenhuma faixa cubra.
settings-radio-dongle = Dongle
settings-radio-rescan = Buscar novamente
settings-radio-re-list-usb-bus-no = Relista o barramento USB. Nenhum dispositivo é aberto, então é seguro pressionar durante a recepção.
settings-radio-no-dongles-press-rescan = nenhum dongle — pressione Buscar novamente
settings-radio-first-one-found = — o primeiro encontrado —
settings-radio-rtl2832u-s-resampler-reaches-225 = O reamostrador do RTL2832U alcança 225–300 kHz e 900 kHz–3,2 MHz, nada entre eles. Tem efeito em Aplicar.
settings-radio-often-drops-samples = {"  "}(muitas vezes descarta amostras)
settings-radio-agc = AGC
settings-radio-manual-setting-measurement-weak-signal = Manual é a configuração para medição e modos digitais de sinal fraco. O sintonizador e o demodulador têm laços automáticos independentes.
settings-radio-tuner-gain = Ganho do sintonizador
settings-radio-applies-immediately-no-reconnect-tuner = Vale imediatamente — sem reconectar. O sintonizador tem 29 passos discretos, então o valor se fixa no mais próximo que ele consegue produzir. Ignorado enquanto o AGC do sintonizador está rodando.
settings-radio-crystal-error-parts-per-million = Erro do cristal em partes por milhão. Rode com RUST_LOG=sdroxide_rtlsdr=debug e o log imprime o erro de clock medido após cerca de 20 segundos — esse é o número a informar. Vale imediatamente.
settings-radio-hf-reception = Recepção de HF
settings-radio-tuner-itself-starts-24-mhz =
    O próprio sintonizador começa em 24 MHz. Um RTL-SDR Blog V4 faz upconversão abaixo disso por hardware; outros dongles alcançam HF apenas amostrando o ADC diretamente, pela porta HF do V3. Automático entrega tudo abaixo de 24 MHz ao ADC e tudo acima dele ao sintonizador.

    A amostragem direta cobre todas as faixas de HF, inclusive 17 m e 15 m — elas chegam na segunda zona de Nyquist do ADC, na orientação correta. Porém, nada filtra a entrada do ADC, então o que estiver em 28,8 MHz menos o dial vem junto: 10,7 MHz sob 17 m, 7,726 MHz sob 15 m.

    Trocar de modo interrompe o fluxo por um instante.
settings-radio-removes-dongle-s-own-dc =
    Remove o pico de CC próprio do dongle do centro da faixa, e a imagem espelhada que cada sinal deixa refletida em torno dele, medindo o desequilíbrio nas próprias amostras — sem calibração, e vale imediatamente. O sintonizador não tem modo de deslocamento de oscilador, então esta é a única forma de limpar o centro.

    Uma portadora de AM sintonizada exatamente no dial também está em CC, então vai junto com o pico: sintonize um quilohertz ao lado dela, ou desligue isto.
settings-radio-remove-centre-spike-mirror-image = Remover o pico central e a imagem espelhada
settings-radio-bias-tee = Bias tee
settings-radio-feed-4-5-v-dc = Alimentar ~4,5 V CC pelo coaxial
settings-radio-bias-tee-never-connect-transceiver = O bias tee está LIGADO. Nunca conecte um transceptor, uma antena aterrada ou um pré-amplificador alimentado de outra fonte enquanto isto estiver ativo — a CC vai direto pela linha de alimentação.
settings-radio-receive-only-dongle-sample-rate = Somente recepção. O dongle e a taxa de amostragem têm efeito em Aplicar; o restante vale à medida que você altera.
settings-radio-server-address = Endereço do servidor
settings-radio-where-rtl-tcp-listening-address =
    Onde o rtl_tcp está escutando: um endereço, ou um endereço e porta. A porta padrão é 1234, que é o padrão do próprio rtl_tcp.

    Na outra ponta, inicie-o com `rtl_tcp -a 0.0.0.0` — vinculado a 127.0.0.1, que é o que ele faz sem -a, ele só aceita conexões desta mesma máquina.

    Tem efeito em Aplicar.
settings-radio-host-host-port-e-g = host ou host:porta, ex.: raspberrypi.local:1234
settings-radio-requested-server-same-resampler-limits =
    Solicitado ao servidor, com os mesmos limites de reamostrador de um dongle local — é o mesmo silício na outra ponta.

    O número ao lado de cada taxa é o que ela custa no link: as amostras são enviadas sem compressão, e uma taxa que a rede não aguenta faz o rtl_tcp derrubar a conexão em vez de degradar. Tem efeito em Aplicar.
settings-radio-wired-link = {"  "}(link cabeado)
settings-radio-runs-server-s-dongle-manual = Roda no dongle do servidor. Manual é a configuração para medição e modos digitais de sinal fraco.
settings-radio-applies-immediately-no-reconnect-sent = Vale imediatamente — sem reconectar. Enviado em décimos de dB e ajustado pelo servidor a um passo que o sintonizador dele tenha; o protocolo não tem respostas, então o valor em que ele se fixou não pode ser lido de volta, e este controle continua mostrando o que foi pedido. Ignorado enquanto o AGC do sintonizador está rodando.
settings-radio-crystal-error-server-s-dongle =
    Erro do cristal do dongle do *servidor*, em partes por milhão — uma propriedade daquele hardware, então é definido aqui e não nos dongles desta máquina. Vale imediatamente.

    O erro de clock medido que a interface USB imprime não está disponível aqui: pela rede, o que essa medição enxerga é o buffer, não o cristal, e ela erra por milhares de ppm. Calibre o dongle pelo USB uma vez e leve o número para cá, ou sintonize uma emissora de frequência conhecida e ajuste até que ela fique no dial.
settings-radio-tuner-starts-24-mhz-below =
    O sintonizador começa em 24 MHz; abaixo disso a outra ponta precisa de ajuda. Um Blog V4 faz upconversão por conta própria e o Automático não mexe nele — que é a única coisa que ele pode fazer, já que o protocolo informa o chip do sintonizador e nada além disso, e um V4 é indistinguível de um R828D comum pelo fio.

    Em um V3 ou em qualquer outro dongle, o Automático coloca o servidor em amostragem direta abaixo do limite próprio de 24 MHz do sintonizador — o que cobre todas as faixas de HF, com 17 m e 15 m chegando na segunda zona de Nyquist do ADC e com 28,8 MHz menos o dial dobrado por cima. Escolha Amostragem direta explicitamente para um R828D comum que não ouve nada em HF. A troca interrompe o fluxo por um instante.
settings-radio-removes-dongle-s-dc-spike =
    Remove aqui o pico de CC do dongle e a imagem espelhada, a partir das amostras à medida que chegam — são artefatos do hardware, então viajam pela rede junto com todo o resto. Vale imediatamente.

    Uma portadora de AM sintonizada exatamente no dial também fica em CC, então vai junto com o pico: sintonize um quilohertz ao lado dela, ou desligue isto.
settings-radio-powers-preamp-far-end-s = Alimenta um pré-amplificador pelo dongle da outra ponta. Servidores mais antigos não implementam o comando e o ignoram em silêncio — o protocolo não tem como dizer não —, então um bias tee que não liga não é necessariamente culpa desta ponta.
settings-radio-feed-4-5-v-dc-2 = Alimentar ~4,5 V CC pelo coaxial remoto
settings-radio-bias-tee-hardware-somewhere-else = O bias tee está LIGADO, em hardware que fica em outro lugar. O que estiver na outra ponta dessa linha de alimentação — um transceptor, uma antena aterrada, um pré-amplificador já alimentado — não está na sua frente para conferir.
settings-radio-sdrplay-server-rsp-tcp = Servidor SDRplay (rsp_tcp)
settings-radio-sdrplay-server-greets-exactly-like = Um servidor SDRplay se apresenta exatamente como um dongle, então estes campos são sempre mostrados, e não quando um é detectado. Com um servidor rtl_tcp comum eles são simplesmente ignorados — o protocolo não tem respostas e descarta comandos que não conhece. A aba Dispositivo identifica o servidor como rsp_tcp quando ele se identifica.
settings-radio-input = Entrada A
settings-radio-input-b = Entrada B
settings-radio-hi-z = Hi-Z
settings-radio-lna-state = Estado do LNA
settings-radio-step-index-not-db-figure = Um índice de passo, não um valor em dB: o quanto cada passo vale depende do modelo do RSP e da faixa. 0 é o máximo de ganho.
settings-radio-if-gain-reduction = Redução de ganho de F.I.
settings-radio-reduction-so-more-less-signal = Uma redução, então mais significa menos sinal. Só é obedecida com o AGC do RSP desligado.
settings-radio-enable = Ativar
settings-radio-dbfs = {" "}dBfs
settings-radio-level-rsp-s-agc-aims = O nível que o AGC do RSP procura manter.
settings-radio-notches = Notches
settings-radio-am = AM
settings-radio-fm = FM
settings-radio-dab = DAB
settings-radio-rf = RF
settings-radio-reference-out = Saída de referência
settings-radio-t-24-mhz-clock-out = Saída de clock de 24 MHz
settings-radio-rsp2-rspduo-only = Somente RSP2 e RSPduo.
settings-radio-receive-only-address-sample-rate = Somente recepção. O endereço e a taxa de amostragem têm efeito em Aplicar; o restante vale à medida que você altera. Uma conexão caída é tentada de novo sozinha, então um servidor reiniciado volta sem que nada aqui precise ser tocado.
settings-radio-where-spyserver-listening-address-address =
    Onde o spyserver está escutando: um endereço, ou um endereço e porta. A porta padrão é 5555, que é o padrão do próprio spyserver.

    Na outra ponta, confira se o arquivo de configuração dele vincula um endereço que outras máquinas alcancem — vinculado a 127.0.0.1, ele só aceita conexões desta mesma máquina.

    Tem efeito em Aplicar.
settings-radio-host-host-port-e-g-2 = host ou host:porta, ex.: raspberrypi.local:5555
settings-radio-i-q-bandwidth = Largura de banda I/Q
settings-radio-how-wide-slice-band-arrives =
    Quão larga é a fatia da faixa que chega como I/Q — e, portanto, quanto do link isto usa. Esta é a janela *demodulada*: ela segue o dial, e tudo que for mais largo que ela é a FFT do servidor na faixa acima do panadaptador.

    Automático procura cerca de 96 kHz, que leva todos os modos aqui, inclusive FM larga, e ainda cabe em um uplink celular. Tem efeito em Aplicar.
settings-radio-which-stage-server-s-own =
    Qual estágio da escada de taxas própria do servidor pedir. Cada receptor tem uma escada diferente — é a taxa máxima dele dividida pela metade, estágio a estágio —, então isto é guardado como o estágio e não como um valor em hertz, e a mesma configuração ainda faz sentido apontada para outro servidor.

    Automático procura cerca de 1 Msps. Pressione Testar conexão para ver a que os estágios correspondem neste servidor. Tem efeito em Aplicar.
settings-radio-automatic = Automático
settings-radio-sample-format = Formato de amostra
settings-radio-bits-per-component-wire-so =
    Bits por componente no fio, e portanto o que isto custa no link: 16 bits é o dobro de 8 bits, e ponto flutuante de 32 bits é quatro vezes isso sem mais informação do que o ADC do receptor tinha.

    8 bits é o que faz um receptor remoto funcionar sobre um uplink doméstico, e é o certo para quase tudo. Um servidor pode sobrepor isto — alguns são configurados para exigir um formato — e informa isso no log quando o faz. Tem efeito em Aplicar.
settings-radio-gain = Ganho
settings-radio-server-s-gain-stage-index =
    O estágio de ganho do servidor, como índice — não como um número de decibéis. O que cada índice significa pertence ao receptor da outra ponta e muda com a faixa, e o protocolo nunca diz, então nada aqui pode convertê-lo em dB sem inventar um valor.

    A faixa real é a do servidor e só é conhecida depois de conectado; um índice além dela é limitado ao máximo. Vale imediatamente — sem reconectar.
settings-radio-index = índice{" "}
settings-radio-digital-gain = Ganho digital
settings-radio-how-far-server-scales-samples =
    Quanto o servidor amplia as amostras antes de quantizá-las para o fio. Automático observa o que de fato chega e mantém o pico na metade da escala total, que é a única coisa que pode estar certa tanto em uma faixa vazia quanto em uma lotada — as duas estão a trinta decibéis de distância e nenhum valor fixo serve às duas.

    Importa somente em 8 bits, onde um sinal bem abaixo da escala total perde os bits inferiores para o quantizador e um acima dela é cortado reto. Formatos mais largos têm folga de sobra e recebem o valor que todo outro cliente calcula, a partir do índice de ganho e do estágio de decimação. Vale imediatamente.
settings-radio-full-band-strip = Faixa de banda completa
settings-radio-server-s-own-fft-whole =
    A FFT própria do servidor da faixa inteira, desenhada na faixa acima do panadaptador. Nesta interface é a única visão de banda que existe — o próprio panadaptador tem só a largura do I/Q que está sendo recebido.

    Desligar isto deixa um receptor sem como ver nada que ele já não esteja sintonizado. Vale imediatamente, com uma pequena falha no áudio enquanto o servidor muda o que está enviando.
settings-radio-ask-server-low-rate-fft =
    Pedir ao servidor uma FFT de baixa taxa da faixa inteira, além do I/Q, e desenhá-la na faixa acima do panadaptador.

    Custa quase nada — alguns quilobytes por quadro, uma dúzia de vezes por segundo — e mostra o receptor inteiro em vez da fatia que está sendo demodulada. Vale desligar só em um link em que cada byte conta. Vale imediatamente, com uma pequena falha no áudio.
settings-radio-show = Mostrar
settings-radio-whole-band = banda inteira
settings-radio-how-much-receiver-strip-covers = Quanto do receptor a faixa cobre. A banda inteira é a visão mais larga que existe; estreitá-la coloca o mesmo número de raias em menos espectro, que é um detalhe mais fino sobre um trecho menor.
settings-radio-strip-db-window = Janela de dB da faixa
settings-radio-server-quantises-fft-into-window =
    O servidor quantiza a FFT dele nesta janela antes de enviá-la, um byte por raia — então estes valores decidem com que precisão a faixa é medida, não só como ela é desenhada. O piso e o teto com que a faixa é *exibida* são definidos pelo ajuste automático de nível do próprio motor e são outra coisa.

    O padrão de 150 dB é a faixa inteira do protocolo e não precisa de atenção. Estreitá-la em torno do piso de ruído compra resolução em um receptor cujos sinais estão todos numa pequena parte da escala. Vale imediatamente.
settings-radio-top = topo{" "}
settings-radio-range-2 = faixa{" "}
settings-radio-i-q-correction = Correção I/Q
settings-radio-remove-dc-spike-mirror-image = Remove o pico de CC e a imagem espelhada no DSP, deste lado. Se o receptor da outra ponta precisa disto depende do que ele é — um Airspy HF+ não precisa, um RTL-SDR precisa — e o protocolo não diz com qual está falando. Vale imediatamente.
settings-radio-enabled = Ativado
settings-radio-test-connection = Testar conexão
settings-radio-connect-read-what-server-says = Conecta, lê o que o servidor diz sobre o receptor dele e desconecta de novo — sem iniciar um fluxo, então é seguro pressionar em um servidor que outra pessoa está usando.
settings-radio-receive-only-panadapter-narrow-i = Somente recepção. O panadaptador é a janela I/Q estreita, que segue o dial; a visão de banda é a FFT do servidor na faixa acima dele. Pressione "Aplicar / reconectar" para trocar sem reiniciar.
settings-radio-receive-only-wideband-i-q = Somente recepção. I/Q de banda larga, igual a qualquer SDR local. Pressione "Aplicar / reconectar" para trocar sem reiniciar.
settings-radio-receiver-address = Endereço do receptor
settings-radio-where-receiver-serves-web-page =
    Onde o receptor serve a página web dele: um endereço, ou um endereço e porta. A porta padrão é 8073, que é a do próprio KiwiSDR.

    Atenção: um receptor acessado pelo proxy do projeto (algo.proxy.kiwisdr.com — quase metade dos públicos) responde na porta 80, então informe-a explicitamente. Escolher um receptor em "SDRs públicos" preenche isto corretamente.

    Tem efeito em Aplicar.
settings-radio-host-host-port-e-g-3 = host ou host:porta, ex.: kiwi.example.org:8073
settings-radio-password = Senha
settings-radio-receiver-s-user-password-where =
    A senha de *usuário* do receptor, quando o operador dele definiu uma. Em branco em quase todo receptor público. Não é a senha de administrador.

    Tem efeito em Aplicar.
settings-radio-announce = Anunciar como
settings-radio-name-connection-shows-up-under =
    O nome sob o qual esta conexão aparece, que o dono do receptor e todos os outros que o escutam podem ver.

    Em branco, é o seu indicativo de estação, que é a convenção da própria rede — alguns operadores bloqueiam clientes que não se identificam. Tem efeito em Aplicar.
settings-radio-blank-station-callsign = em branco = indicativo da estação
settings-radio-band-view = Visão de banda
settings-radio-ask-receiver-own-waterfall-well =
    Pedir ao receptor a cascata dele, além do I/Q, e desenhá-la na faixa acima do panadaptador.

    Vale ter: o I/Q tem só cerca de 12 kHz de largura, então sem isto não há nada para sintonizar *por*. É uma segunda conexão que leva cerca de 20 kB/s em velocidade total, contra os 44 kB/s do I/Q.

    Tem efeito em Aplicar.
settings-radio-show-receiver-s-0-30 = mostrar a cascata de 0-30 MHz do receptor
settings-radio-band-view-speed = Velocidade da visão de banda
settings-radio-how-often-receiver-sends-waterfall =
    Com que frequência o receptor envia uma linha da cascata, de 1 (mais lento) a 4. É o único ajuste aqui que muda o que o link custa durante a execução, então reduza-o em uma conexão medida.

    Vale imediatamente.
settings-radio-band-view-span = Largura da visão de banda
settings-radio-how-much-band-receiver-s =
    Quanta banda a cascata do receptor cobre. A faixa completa de 0-30 MHz fica à esquerda do controle deslizante, e cai pela metade a cada passo para a direita.

    A cascata tem sempre 1024 raias de largura, não importa quão larga seja a janela, então isto é realmente um controle de resolução: a banda inteira fica a 29 kHz por raia, que é um *mapa* da banda em vez de uma imagem de qualquer coisa, enquanto algumas centenas de quilohertz mostram estações individuais em uma faixa de radiodifusão. Com zoom, a janela acompanha o seu dial.

    A banda inteira é o padrão e é o que torna a faixa algo para sintonizar; o panadaptador abaixo dela são os cerca de 12 kHz de I/Q. Vale imediatamente.
settings-radio-receiver-agc = AGC do receptor
settings-radio-receiver-s-agc-which-far =
    O AGC *do receptor*, que fica do outro lado do link e antes do I/Q — então ele age antes de qualquer coisa que o sdroxide faça.

    Ligado por padrão, diferentemente de todo SDR local aqui, porque a alternativa medida era pior: em um receptor ao vivo, o ganho manual não era monotônico em toda a sua faixa e a sua extremidade superior cortava o I/Q na escala total, enquanto o AGC mantinha cerca de -24 dBFS com 7 dB de folga.

    Isso faz com que a amplitude da amostra não seja um nível de sinal, e por isso o S-metro é lido do próprio valor do receptor. Vale imediatamente.
settings-radio-let-receiver-ride-own-gain = deixar o receptor controlar o próprio ganho
settings-radio-manual-gain = Ganho manual
settings-radio-fixed-gain-when-receiver-s = Ganho fixo quando o AGC do receptor está desligado, em uma escala própria de 0-90. Não são decibéis de nada declarado — o protocolo o chama de manGain e não diz mais nada, e medimos que ele não é monotônico, então trate-o como um controle para achar um bom ponto, não como calibração. Vale imediatamente.
settings-radio-read-receiver-s-status-page = Lê a página de status do receptor: o que ele é, o que cobre, quantos canais estão livres e se o operador permite conexões de aplicativos que não sejam um navegador. Não ocupa nenhum dos canais dele.
settings-radio-host-port-e-g-127 = host:porta, ex.: 127.0.0.1:50001
settings-radio-iq-sample-rate = Taxa de amostragem IQ
settings-radio-receiver = Receptor
settings-radio-rig-two-receivers-sunsdr2dx-can = Um equipamento com dois receptores (SunSDR2DX) pode atender duas abas de rádio em uma só conexão — rode este rádio no RX1 e outro no RX2. O transmissor pertence ao rádio do RX1.
settings-radio-stream-delay = Atraso do fluxo
settings-radio-how-long-rig-s-iq = Quanto tempo o IQ do equipamento leva para chegar em um novo centro depois que o sdroxide o move. É o pipeline de DSP do próprio equipamento, não a rede — um servidor nesta máquina também tem um —, e, enquanto o panadaptador é arrastado com zoom totalmente afastado, errar por esse tanto desloca as linhas mais recentes da cascata para o lado do histórico pelo erro vezes a velocidade do arraste. Alto demais as desloca exatamente tanto quanto baixo demais, no sentido contrário. Meça com `cargo run --release -p sdroxide-tci --example retune_latency`; o padrão é um SunSDR2DX no ExpertSDR3 em 192 kHz.
settings-radio-wideband-iq-receive-audio-transmit = I/Q de banda larga na recepção, áudio na transmissão. Pressione "Aplicar / reconectar" para trocar sem reiniciar.
settings-radio-radio-address = Endereço do rádio
settings-radio-address-shown-radio-under-set = O endereço mostrado no rádio em SET > Network. O Network Control precisa estar ativado ali, e o rádio precisa ter um usuário e senha de rede definidos.
settings-radio-host-ip-e-g-192 = host ou IP, ex.: 192.168.1.50
settings-radio-control-port = Porta de controle
settings-radio-t-50001-unless-has-been-changed = 50001, a menos que tenha sido alterada no rádio.
settings-radio-network-user = Usuário de rede
settings-radio-stored-clear-radio-json-protocol = Armazenado em texto puro no radio.json. O protocolo o ofusca de forma reversível no fio, então nada aqui o tornaria um segredo.
settings-radio-receive = Receber de
settings-radio-af-radio-demodulates-sdroxide-shows = AF: o rádio demodula e o sdroxide mostra a faixa de áudio. F.I. de 12 kHz: o rádio envia em vez disso a frequência intermediária do DRM dele e o sdroxide demodula, o que traz os próprios filtros, redução de ruído e decodificadores para cerca de ±12 kHz. De qualquer forma, a cascata larga é o scope do próprio rádio — nenhum Icom envia I/Q. Uma preferência de F.I. usa temporariamente AF para modos mais largos, como WFM, e depois volta a F.I. para os modos compatíveis. Uma preferência de AF permanece AF. HD Radio não está disponível nesta conexão.
settings-radio-audio-sample-rate = Taxa de amostragem de áudio
settings-radio-t-12-khz-if-needs-48000 = Um F.I. de 12 kHz exige o fluxo de 48000 Hz — nessa taxa é usado o áudio demodulado do rádio.
settings-radio-if-spectrum = Espectro de F.I.
settings-radio-which-way-round-radio-s = Em que sentido roda o F.I. de 12 kHz do rádio. "Automático" usa o que o modelo é conhecido por fazer — espelhado em um IC-7760, normal em todos os outros Icom. Ajuste à mão se a SSB sair na banda lateral errada: o indício é ter de selecionar USB onde a faixa roda LSB, enquanto a indicação de modo do próprio rádio concorda com o sdroxide o tempo todo.
settings-radio-normal = Normal
settings-radio-mirrored = Espelhado
settings-radio-displayed-bandwidth = Largura de banda exibida
settings-radio-width-audio-band-panadapter-cat = Largura do panadaptador da faixa de áudio, como para um equipamento CAT. Usado nos modos digitais e quando o scope do rádio está desligado; caso contrário, o panadaptador é o scope, e a Largura do scope define a largura dele.
settings-radio-how-cw-panel-s-keyer-2 = Como o manipulador do painel CW transmite. "Manipulador do rádio" coloca o rádio em CW e lhe entrega o texto a enviar com o manipulador próprio dele (CI-V 0x17). "Placa de som" envia o sidetone manipulado pela LAN como áudio (MCW), um tom em dial + tom — e, como um equipamento em CW ignoraria a entrada do modulador, selecionar CW então mantém o rádio em USB comum, o mesmo modo em que os modos digitais trafegam.
settings-radio-how-much-audio-radio-holds = Quanto áudio o rádio guarda antes de modular. Mais sobrevive a uma rede pior, ao custo de latência de transmissão.
settings-radio-streams-radio-s-own-475 = Transmite a varredura própria de 475 raias do rádio. Em AF é o panadaptador — o áudio que o rádio envia é o que passou pelo filtro dele, não uma imagem da banda — e na F.I. de 12 kHz é a cascata da banda inteira acima de um. De qualquer forma, é a imagem do rádio, não o DSP do sdroxide: não há I/Q a partir do qual calcular uma.
settings-radio-how-wide-sweep-only-wide = Quão larga é a varredura. Esta é a única visão larga que um Icom tem, e o rádio mantém a largura escolhida por último na própria tela — muitas vezes alguns kHz, por isso a faixa pode aparecer não mais larga que o panadaptador abaixo dela. Definir uma largura aqui também coloca o scope em modo de centro, para que siga o dial. Isso altera também a tela do próprio rádio; "Como ajustado no rádio" a deixa intacta.
settings-radio-switch-modulation-input-lan = Trocar a entrada de modulação para LAN
settings-radio-transmit-audio-only-heard-when = O áudio de transmissão só é ouvido quando a entrada MOD do rádio está em LAN. O sdroxide consegue escrever isso em um modelo cuja numeração de menu ele conhece; em qualquer outro, informa e deixa o menu como está. É um empréstimo: o que o rádio tinha ali é lido primeiro e devolvido quando a sessão termina, então um equipamento usado sozinho depois ainda ouve o próprio microfone.
settings-radio-copy-diagnostic-report = Copiar relatório de diagnóstico
settings-radio-copies-radio-s-last-session = Copia a última sessão deste rádio — o handshake dele e o rastro CI-V — para a área de transferência, para um relatório de bug. É do rádio, não da estação: com dois Icoms na LAN, cada aba responde pelo próprio endereço.
settings-radio-control-audio-radio-s-own = Controle, áudio e o scope próprio do rádio por uma única conexão de rede — sem cabo serial e sem placa de som. Pressione "Aplicar / reconectar" para trocar sem reiniciar.
settings-radio-testing = Testando…
settings-radio-was-only-check-press-apply = Foi só uma verificação — pressione Aplicar / reconectar abaixo para começar a usar este rádio.
settings-radio-radios = Rádios
settings-radio-asks-network-iio-devices-also = Pergunta à rede pelos dispositivos IIO, e também tenta 192.168.2.1 diretamente — um Pluto na ponta de um cabo USB muitas vezes não é alcançável por multicast, embora o endereço funcione.
settings-radio-no-radios-press-discover = nenhum rádio — pressione Descobrir
settings-radio-address = Endereço
settings-radio-overrides-selection-above-usb-cable = Tem precedência sobre a seleção acima. O cabo USB apresenta o Pluto como um adaptador de rede, não como uma porta serial, então isto é um endereço IP mesmo quando o rádio está na sua mesa.
settings-radio-pluto-revision-c-pluto-unlocked = Um Pluto+ ou um Pluto de revisão C desbloqueado para 2R2T pode atender duas abas de rádio em uma só caixa — este rádio no RX1 e outro no RX2, cada um na sua antena. As duas cadeias compartilham o mesmo oscilador, então sintonizar qualquer um move os dois; o que o RX2 traz é uma segunda antena no mesmo espectro (diversidade), não uma segunda faixa. O transmissor pertence ao rádio do RX1. Um Pluto 1R1T de fábrica recusa o RX2 quando conecta.
settings-radio-width-spectrum-sdroxide-receives-ad9361 = Largura do espectro que o sdroxide recebe. O AD9361 chega a 61,44 Msps; o link de rede USB não, e é a isso que esta lista está ajustada. As taxas mais baixas exigem uma configuração de filtro carregada no AD9361, o que o sdroxide não faz — um Pluto de fábrica roda nelas a cerca de 2,084 Msps e informa isso ao conectar. Tem efeito em Aplicar.
settings-radio-more-than-usb-2-will = {"  "}(mais do que o USB 2 suporta)
settings-radio-stock-pluto-runs-2-084 = {"  "}(um Pluto de fábrica roda a 2,084)
settings-radio-analog-filter = Filtro analógico
settings-radio-ad9361-s-baseband-filter-leave = O filtro de banda base do AD9361. Deixe em 0 para automático, que o abre até nove décimos da taxa de amostragem — largo de propósito, porque o receptor estaciona o oscilador um quarto de largura de faixa fora do dial para manter os sinais longe do pico de CC, e um filtro estreito cortaria exatamente a parte para a qual os moveu. Tem efeito em Aplicar.
settings-radio-ad9361-has-four-modes-not = O AD9361 tem quatro modos, não um interruptor liga/desliga. Ataque lento serve a SSB e CW; ataque rápido serve sinais em rajadas; manual é a configuração para medição e modos digitais de sinal fraco. Vale imediatamente.
settings-radio-rx-gain = Ganho RX
settings-radio-applies-immediately-no-reconnect-ignored = Vale imediatamente — sem reconectar. Ignorado a menos que o AGC esteja em manual, o que é o comportamento do próprio AD9361, não do sdroxide.
settings-radio-tx-gain = Ganho TX
settings-radio-negative-because-ad9361-expresses-transmit = Negativo porque o AD9361 expressa o nível de transmissão como atenuação: 0 dB é a saída total. Vale imediatamente. O transmissor é ajustado para o mínimo antes de este valor ser aplicado na conexão, para que nada que o programa anterior deixou possa estar ativo.
settings-radio-reference-error-parts-per-million = Erro de referência em partes por milhão. Rode com RUST_LOG=sdroxide_pluto=debug e o log imprime o erro de clock medido após cerca de 20 segundos — esse é o número a informar. Aplicado pelo sdroxide, não gravado no ajuste fino persistente do próprio rádio. Vale imediatamente.
settings-radio-full-duplex = Duplex total
settings-radio-keep-receiving-while-transmitting = Continuar recebendo durante a transmissão
settings-radio-leave-off-pluto-reached-over =
    Deixe isto desligado em um Pluto acessado pelo cabo USB. Esse link não aguenta um megassample por segundo nos dois sentidos ao mesmo tempo, e uma transmissão que faz faltar dados ao buffer de transmissão sai picotada — então, por padrão, a recepção para durante a transmissão.

    Ative para uma placa em Ethernet real (um LibreSDR, um Pluto em um adaptador gigabit), onde há espaço para os dois: você então ouve o receptor durante a própria transmissão, que é como uma estação QO-100 escuta o próprio downlink.

    Uma transmissão precisa do dobro da taxa de amostragem em link — 2,5 Msps são 10 MB/s em cada sentido — então reduza a taxa se o log começar a dizer que o link não a sustenta. O panadaptador ainda mostra o sinal transmitido durante a transmissão; é o áudio que continua chegando.

    Tem efeito em Aplicar.
settings-radio-duplex = Duplex
settings-radio-whether-ad9361-runs-both-directions =
    Se o AD9361 roda as duas direções ao mesmo tempo (FDD, que é como um Pluto inicia e em que o sdroxide sempre o deixou) ou uma de cada vez (TDD).

    Deixe em FDD, a menos que queira os pinos de PTT abaixo — o TDD é o que eles acionam, e ele descarta o Duplex total acima. Tem efeito em Aplicar.
settings-radio-ptt-pins = Pinos de PTT
settings-radio-pluto-s-four-gpo-test =
    Os quatro pontos de teste GPO do Pluto podem acionar sozinhos um amplificador de potência externo, um LNA ou uma chave de transmissão/recepção. Escolha um par e um pino fica em nível alto o tempo todo em que o rádio recebe, e o outro o tempo todo em que transmite — sem software do computador no meio e sem linha serial de PTT para fiar.

    Isso coloca o rádio em TDD, qualquer que seja a linha Duplex, porque os pinos seguem as linhas de habilitação do AD9361, e em FDD ambas ficam afirmadas durante toda a sessão. O Duplex total é desligado junto, então deixe os dois nos padrões se trabalha com satélites e precisa ouvir o próprio downlink.

    A Analog Devices observa em sua própria nota que um LNA externo fica em GPO0/GPO1, então use GPO2/GPO3 se a sua placa for fiada assim. Os pinos ficam em cerca de 1,3 V com alguns miliamperes: acione um transistor ou um optoisolador com eles, nunca uma bobina de relé.

    Tem efeito em Aplicar.
settings-radio-buffer-size = Tamanho do buffer
settings-radio-how-much-device-holds-before =
    Quanto o dispositivo guarda antes de cada transferência, em amostras complexas. O padrão de 32768 são cerca de 16 ms a 2 Msps: longo o bastante para que o ida-e-volta de cada transferência não seja o gargalo, curto o bastante para que um reajuste não fique visivelmente atrasado.

    Reduza pela metade se o log relatar que o socket de recepção está sendo substituído. Um link marginal — um Pluto acessado pelo cabo USB em alta taxa de amostragem é o caso típico — trava no meio de uma transferência, e uma transferência menor tem menos chance de ser pega no meio do voo e se recupera mais rápido depois. Aumente para trocar latência de reajuste por menos ida-e-volta.

    Tem efeito em Aplicar.
settings-radio-samples = {" "}amostras
settings-radio-rx-tx-port = Porta RX / TX
settings-radio-ad9361-s-rf-port-select = O rf_port_select do AD9361. Um Pluto de fábrica liga um de cada, então deixe estes vazios, a menos que tenha uma placa que não faça isso. Tem efeito em Aplicar.
settings-radio-opens-radio-reads-what-says = Abre o rádio, lê o que ele diz sobre si mesmo e informa a faixa de sintonia que esta placa específica tem. Não inicia um fluxo.
settings-radio-copies-last-session-s-protocol = Copia o rastro do protocolo da última sessão para a área de transferência, para um relatório de bug.
settings-radio-wideband-iq-receive-transmit-half = I/Q de banda larga na recepção e na transmissão. Meio duplex — a recepção para durante a transmissão, porque o link de rede USB não aguenta os dois ao mesmo tempo. Não precisa de SoapySDR nem de libiio.
settings-radio-not-yet-verified-against-real = Ainda não verificado em hardware real. Se apresentar mau funcionamento, envie o relatório de diagnóstico — ele contém cada comando trocado com o rádio e os primeiros bytes do fluxo de amostras.
settings-radio-flexradio-announces-itself-local-network = Um FlexRadio se anuncia na rede local cerca de uma vez por segundo. Um rádio acessado por roteador ou VPN nunca transmite para você — informe o endereço dele abaixo.
settings-radio-another-gui-client-has-radio = Outro cliente gráfico tem este rádio e o multiFLEX está desativado. Desconecte esse cliente, ou ative o multiFLEX no rádio.
settings-radio-overrides-selection-above-use-radio = Tem precedência sobre a seleção acima. Use isto para um rádio em outra sub-rede, atrás de uma VPN ou em uma porta fora do padrão.
settings-radio-width-spectrum-sdroxide-receives-192 = Largura do espectro que o sdroxide recebe. 192 kHz é o máximo do rádio para um fluxo DAX IQ, e portanto a maior largura que esta interface pode mostrar.
settings-radio-t-192-khz-widest-dax-iq = 192 kHz é o fluxo DAX IQ mais largo que um FLEX envia, e o rádio prende a largura do panadaptador a ele — então essa é a faixa inteira, e nenhuma configuração aqui a amplia.
settings-radio-dax-iq-channel = Canal DAX IQ
settings-radio-radio-has-four-change-only = O rádio tem quatro. Altere isto só se outra coisa na rede já estiver usando o canal 1 — o rádio recusa um canal duplicado.
settings-radio-station-name = Nome da estação
settings-radio-shown-against-session-radio-s = Mostrado junto a esta sessão na lista de clientes do rádio. O rádio também reconhece um cliente por ele, então renomear faz o rádio tratar o sdroxide como um novo.
settings-radio-gui-client-id = ID do cliente gráfico
settings-radio-identity-radio-restores-session-s = A identidade para a qual o rádio restaura os slices e panadaptadores desta sessão. Vazio, ela é derivada do nome da estação — estável, mas igual em todo sdroxide que manteve o nome padrão, e um rádio resolve uma duplicata derrubando o cliente anterior. O sdroxide percebe isso no fio e recorre a uma identidade de uma só sessão, o que custa a restauração; informe aqui um UUID próprio para mantê-la.
settings-radio-optional-e-g-uuid-your = opcional, ex.: um UUID próprio
settings-radio-mirror-radio-s-i-q =
    Espelha o I/Q do rádio em torno do centro do panadaptador. Desligado por padrão, que é como um FLEX-6600 foi verificado.

    Experimente se o áudio de recepção estiver ininteligível tanto em USB *quanto* em LSB e nada decodificar, enquanto a cascata parece totalmente convincente — é assim que um fluxo espelhado se apresenta, e é a única falha sem outro sintoma (issue #368). Se não for isso, ligar isto torna o problema óbvio em vez de sutil. Vale em Aplicar / reconectar.
settings-radio-network-mtu = MTU de rede
settings-radio-largest-datagram-radio-may-send = O maior datagrama que o rádio pode enviar. 1450 é o que o próprio SmartSDR pede. Reduza em um caminho com MTU menor — uma VPN ou um túnel — onde os fragmentos são descartados e nenhum espectro chega.
settings-radio-b = {" "}B
settings-radio-checks-radio-answers-without-registering = Verifica se o rádio responde, sem se registrar como cliente gráfico — então não perturba uma sessão do SmartSDR que já esteja rodando.
settings-radio-wideband-iq-receive-over-dax = I/Q de banda larga na recepção pelo DAX, áudio de transmissão que o rádio modula. Pressione "Aplicar / reconectar" para trocar sem reiniciar.
settings-radio-not-yet-verified-against-real-2 = Ainda não verificado em hardware real. Se apresentar mau funcionamento, envie o relatório de diagnóstico — ele contém cada linha de protocolo trocada com o rádio.
settings-radio-devices-soapysdr-can-see = Dispositivos que o SoapySDR enxerga
settings-radio-waiting-machine-radio-attached-where = Aguardando a máquina à qual o rádio está conectado, onde os módulos estão instalados. O dispositivo aberto é o `device_args` do config.toml dessa máquina, ou o --device dela.
settings-radio-ask-every-installed-soapysdr-module = Pede a todo módulo SoapySDR instalado que faça uma varredura. Nada é aberto, então é seguro durante a recepção — mas pode levar um momento.
settings-radio-not-enumerated-yet-press-rescan = Ainda não enumerado — pressione Buscar novamente.
settings-radio-no-soapysdr-devices-found-check = Nenhum dispositivo SoapySDR encontrado. Verifique se o módulo do seu rádio está instalado e se você pode acessar o dispositivo.
settings-radio-not-radio = não é um rádio
settings-radio-sound-card-listed-above-if = Uma placa de som aparece acima como se fosse um SDR. Ela aceita qualquer frequência e a ignora, então abri-la mostra a entrada da placa de som em vez da faixa. O sdroxide não as escolhe automaticamente — mas uma linha device_args que nomeie uma delas ainda é obedecida.
settings-radio-which-one-opens-set-device = O que abre é definido por --device ou device_args no config.toml; sem nenhum dos dois, vence o primeiro rádio da lista.
settings-radio-no-device = nenhum dispositivo
settings-radio-rig-has-no-software-adjustable = Este equipamento não tem ganhos ajustáveis por software.
settings-radio-rx-gains = Ganhos RX
settings-radio-tx-gains = Ganhos TX
settings-radio-antennas = Antenas
settings-radio-remembered-next-start = Lembrado para o próximo início.
settings-radio-these-controls-appear-once-soapysdr = Estes controles aparecem assim que um dispositivo SoapySDR é aberto — são do próprio dispositivo, e é preciso perguntar a ele.
settings-radio-stream = Fluxo
settings-radio-follow-app-setting = Seguir a configuração do aplicativo
settings-radio-msps = ou, em Msps
settings-radio-baseband-filter = Filtro de banda base
settings-radio-let-driver-choose = Deixar o driver escolher
settings-radio-changing-either-reopens-radio = Alterar qualquer um dos dois reabre o rádio.
settings-radio-driver-publishes-no-settings-own = Este driver não publica configurações próprias.
settings-radio-applied-once-remembered-next-start = Aplicado de imediato e lembrado para o próximo início.
settings-radio-no-rx-888-found-press = Nenhum RX-888 encontrado — pressione Buscar novamente
settings-radio-adc-clock = Clock do ADC
settings-radio-t-129-6-msps-needs-superspeed = 129,6 Msps exige um link SuperSpeed e um computador rápido; 64,8 é o padrão seguro. Alterá-lo reabre o receptor automaticamente, o que leva um momento mas não exige reinício.
settings-radio-panadapter-width = Largura do panadaptador
settings-radio-hf-only = {"  "}· somente HF
settings-radio-usable-both-front-ends = Utilizável nos dois front ends.
settings-radio-below-vhf-crossover-ordinary-width = Abaixo da transição VHF esta é uma largura comum. Acima dela, o F.I. de 8 MHz do sintonizador não pode ser centralizado em uma janela tão larga, então o espectro ao vivo fica deslocado para um lado do panadaptador, com nada ao lado.
settings-radio-dial-above-crossover-now-so = O dial já está acima da transição, então esta largura mostra o F.I. de 8 MHz do sintonizador fora do centro, com espectro morto ao lado. Reduza a largura para preencher o panadaptador.
settings-radio-vga-gain = Ganho VGA
settings-radio-ad8370-variable-gain-amplifier-ahead = Amplificador de ganho variável AD8370 antes do ADC.
settings-radio-attenuator = Atenuador
settings-radio-pe4304-step-attenuator-0-5 = Atenuador por passos PE4304, em passos de 0,5 dB.
settings-radio-adc-range = Faixa do ADC
settings-radio-wide-2-25-vp-p = Larga (2,25 Vp-p)
settings-radio-selects-adc-s-wider-input = Seleciona a faixa de entrada mais larga do ADC: mais folga para sinais fortes de radiodifusão, menos contagens para os fracos. Desligado seleciona a faixa mais sensível de 1,5 Vp-p.
settings-radio-dither = Dither
settings-radio-adds-small-dither-signal-ahead = Adiciona um pequeno sinal de dither antes do ADC: custa um pouco de piso de ruído, mas compra faixa dinâmica livre de espúrios.
settings-radio-randomiser = Randomizador
settings-radio-adc-scrambles-output-so-digital = O ADC embaralha a saída para que o barramento digital pare de irradiar para o front end; o driver a desembaralha. Deixe ligado a menos que esteja depurando. Vale ao reconectar.
settings-radio-dc-hf-antenna-port = CC na porta de antena HF
settings-radio-powers-active-antenna-preamp-down = Alimenta uma antena ativa ou pré-amplificador pelo coaxial.
settings-radio-vhf-tuner-gain = Ganho do sintonizador VHF
settings-radio-r828d-rf-gain-used-above = Ganho de RF do R828D, usado acima da transição automática HF/VHF. 29 passos discretos; o mais próximo é usado.
settings-radio-auto = Automático
settings-radio-let-tuner-run-own-lna = Deixar o sintonizador rodar os próprios laços de LNA e misturador.
settings-radio-bias-tee-vhf = Bias tee (VHF)
settings-radio-dc-vhf-antenna-port = CC na porta de antena VHF
settings-radio-clock-trim = Ajuste do clock
settings-radio-corrects-reference-oscillator-applied-when = Corrige o oscilador de referência. Aplicado quando você solta o valor — reabrir a cada pixel de um arraste reiniciaria o receptor centenas de vezes.
settings-radio-receive-only-below-adc-s = Somente recepção. Abaixo do limite de Nyquist do ADC, a antena é amostrada diretamente e a sintonia é instantânea, porque não há conversor de descida de hardware — o fluxo completo do ADC é convertido para banda base no computador. Acima dele, o receptor passa automaticamente para o sintonizador R828D e para a SMA de VHF, então as duas antenas precisam estar conectadas. VHF exige um clock do ADC de 32,4 Msps ou mais para o F.I. do sintonizador caber, e com clocks abaixo de 48 Msps existe um intervalo entre as duas faixas que nada consegue alcançar. Todas as configurações aqui valem de imediato — não há botão Aplicar.
settings-radio-device = Dispositivo
settings-radio-no-devices-press-rescan = nenhum dispositivo — pressione Buscar novamente
settings-radio-receiver-s-input-pad-one = O atenuador de entrada do receptor — um passo, para dentro ou para fora. O mesmo controle do controle deslizante de Ganho da janela principal.
settings-radio-pre-selection-filters = Filtros de pré-seleção
settings-radio-low-pass-bank-front-adc = O banco de passa-baixas antes do ADC. Desviá-lo dá a visão mais larga e o pior comportamento perto de sinais fortes fora da faixa — um transmissor de radiodifusão a alguns quilômetros vai colocar imagens por toda a faixa. Deixe ligado a menos que esteja deliberadamente escutando fora da faixa filtrada.
settings-radio-filters-circuit = Filtros no circuito
settings-radio-rig-control-fdm-duo-only = Controle do equipamento — somente FDM-DUO
settings-radio-transceiver-s-cat-usb-port = A porta USB CAT do transceptor, que é um cabo separado do de recepção. Deixe a porta sem definir em um FDM-S1 ou FDM-S2, que não têm nenhuma — e em um FDM-DUO você preferiria dirigir só pelo cabo de recepção, onde ele ainda pode ser sintonizado e acionado, mas nenhum medidor pode ser lido.
settings-radio-port-set-radio-s-display = Com a porta definida, o visor do rádio e este dial são o mesmo número nos dois sentidos: a janela de recepção do FDM-DUO é construída em torno do seu VFO, então sintonizar aqui move o painel frontal, e girar o botão do painel frontal move a leitura aqui. O panadaptador se recentraliza no dial conforme anda. Sem porta — ou com uma à qual o rádio não responda —, não há VFO a comandar, e a sintonia fica dentro da janela que o rádio já está enviando.
settings-radio-none-usb-control-only = — nenhuma (somente controle USB) —
settings-radio-must-match-menu-70-cat = Deve coincidir com o menu 70 "CAT BAUD" do rádio, que sai de fábrica em 38400. O rádio tem estas quatro taxas e nenhuma outra, e uma porta aberta em qualquer outra fica muda nos dois sentidos — o rádio ignora o dial e recusa acionar, sem nada que diga por quê.
settings-radio-how-transmit-keyed-cat-sends = Como a transmissão é acionada. "CAT" envia o próprio comando TX do rádio e não exige nada configurado no equipamento; RTS exige o menu 54 "PTT" em PTT+RTS.
settings-radio-where-radio-takes-transmit-audio-2 =
    De onde o rádio pega o áudio de transmissão — o comando TI do equipamento, que é o menu 32 "TX IN" no painel frontal. Afirmado quando a porta abre.

    "Áudio USB" é o que faz a transmissão funcionar aqui: o FDM-DUO envia o que o sdroxide coloca na placa de som USB dele. Um rádio deixado em "Microfone" envia a sala no lugar, e nada na tela avisa disso.
settings-radio-which-two-sockets-back-receiver-2 =
    Qual dos dois conectores da traseira o receptor escuta — o comando AN do equipamento, que é o menu 31 "ANTENNAS" no painel frontal e o indicador "ANT 1 2" no visor.

    "RTX" é uma antena fazendo as duas funções, no conector tipo M que também leva a transmissão. "Somente RX" move a recepção para o segundo conector e deixa a transmissão em RTX — uma antena receptora, uma alça ou uma beverage, com a direcional ainda no transmissor.

    Isto move todo este receptor: o panadaptador, os demoduladores e o áudio próprio do equipamento vêm todos do conector selecionado aqui.

    Vale imediatamente, e é lido de volta do rádio quando a porta de controle abre — esta é a configuração do próprio equipamento, não uma cópia guardada aqui.
settings-radio-how-often-radio-asked-what-2 = Com que frequência o rádio é perguntado sobre o que está fazendo — o dial, o modo e os medidores. Gire o botão VFO do próprio equipamento e a leitura acompanha em uma consulta. É também todo o tráfego de controle que esta ponta gera, então reduza-o se o áudio do rádio falhar.
settings-radio-transmit-audio = Áudio de transmissão
settings-radio-system-default = — padrão do sistema —
settings-radio-fdm-duo-s-own-usb = A porta USB Audio própria do FDM-DUO. Deixada no padrão do sistema, quase nunca é o rádio — são os alto-falantes da máquina.
settings-radio-press-rescan-list-sound-cards = pressione Buscar novamente para listar as placas de som
settings-radio-copies-last-session-s-trace = Copia o rastro da última sessão para a área de transferência, para um relatório de bug: cada comando trocado com o dispositivo, e os primeiros bytes do fluxo de amostras.
settings-radio-only-receiving-fdm-duo-has = Até agora, só a recepção em um FDM-DUO foi executada em hardware real. Se algo apresentar mau funcionamento, anexe o relatório de diagnóstico a um relatório de bug.
settings-radio-cw-keyed-radio-s-own = O CW é acionado pela tecla ou manipulador próprio do rádio: o FDM-DUO não tem comando que aceite texto, então o painel CW não consegue acioná-lo.
settings-radio-press-apply-reopen-radio = Pressione Aplicar para reabrir o rádio.
settings-radio-no-receivers-press-rescan = nenhum receptor — pressione Buscar novamente
settings-radio-which-rates-receiver-has-depends = Quais taxas um receptor tem depende do modelo e do firmware. Tem efeito em Aplicar.
settings-radio-every-rate-any-hf-model = todas as taxas que qualquer modelo HF+ oferece — conecte um para ver a sua própria
settings-radio-receiver-s-own-gain-control = O controle de ganho próprio do receptor. Deixe-o ligado para a escuta geral; desligue-o para ajustar o atenuador à mão, para medição.
settings-radio-agc-threshold = Limiar do AGC
settings-radio-high = Alto
settings-radio-high-trades-little-sensitivity-headroom = Alto troca um pouco de sensibilidade por folga contra vizinhos fortes — a configuração certa em uma faixa lotada à noite.
settings-radio-front-end-attenuation-gain-0 = Atenuação do front end, como ganho — 0 dB é nenhuma. Só é obedecida com o AGC desligado.
settings-radio-preamp = Pré-amplificador
settings-radio-hf-low-noise-amplifier-buys = O amplificador de baixo ruído de HF. Compra sensibilidade ao custo de intermodulação, então vem desligado por padrão — o que costuma ser o certo com uma antena real.
settings-radio-lna = LNA ligado
settings-radio-frequency-calibration = Calibração de frequência
settings-radio-parts-per-billion-receiver-s = Partes por bilhão — a unidade própria deste receptor, mil vezes mais fina que o ppm que um RTL-SDR usa. Nada disto é gravado na memória flash do receptor: isto substitui o valor armazenado só para a sessão.
settings-radio-use-receiver-s-stored-value = Usar o valor armazenado no receptor
settings-radio-ppb = {" "}ppb
settings-radio-feed-dc-up-coax = Alimentar CC pelo coaxial
settings-radio-not-every-hf-has-one = Nem todo HF+ tem um; em um receptor sem ele, isto não faz nada.
settings-radio-host-dsp = DSP do computador
settings-radio-image-balancer-zero-if-offset = O balanceador de imagem, o deslocamento de zero-IF e o oscilador de ajuste fino. Desligue-os apenas para ver a saída bruta do hardware — com eles desligados, a imagem espelhada aparece nas taxas de zero-IF e o dial é preciso só até o quilohertz mais próximo.
settings-radio-correct-image-fine-tune = Corrigir a imagem e o ajuste fino
settings-radio-copies-last-session-s-trace-2 = Copia o rastro da última sessão para a área de transferência, para um relatório de bug: cada comando trocado com o receptor, e os primeiros bytes do fluxo de amostras.
settings-radio-receive-only-no-soapysdr-no = Somente recepção. Não precisa de SoapySDR nem de libairspyhf. Abaixo do piso do sintetizador, o oscilador do computador faz a sintonia, que é como este receptor alcança VLF. O receptor e a taxa de amostragem têm efeito em Aplicar; o restante vale à medida que você altera.
settings-radio-not-yet-verified-against-real-3 = Ainda não verificado em hardware real. Se apresentar mau funcionamento, envie o relatório de diagnóstico — ele contém cada comando trocado com o receptor e os primeiros bytes do fluxo de amostras decodificados como pares I/Q.
settings-radio-no-airspy-r2-mini-found = Nenhum Airspy R2 ou Mini encontrado — pressione Buscar novamente
settings-radio-read-receiver = ler deste receptor
settings-radio-r2-mini-offer-different-rates = um R2 e um Mini oferecem taxas diferentes e não podem ser distinguidos até que um esteja aberto
settings-radio-rate-you-get-receiver-s = Esta é a taxa que você obtém. O ADC do receptor roda o dobro dela — digitaliza um sinal real, e o sdroxide faz a banda base complexa a partir dele no computador.
settings-radio-gain-curve = Curva de ganho
settings-radio-step-along-curve-above-not = Um passo ao longo da curva acima, não um valor em dB — o LNA, o misturador e o VGA do sintonizador se movem juntos, e quanto cada passo vale depende da curva e da faixa. 0 é o extremo silencioso.
settings-radio-tuner-agc = AGC do sintonizador
settings-radio-lna-2 = LNA
settings-radio-hands-lna-tuner-s-own = Entrega o LNA ao laço próprio do sintonizador, substituindo a curva.
settings-radio-mixer = Misturador
settings-radio-same-mixer-stage = O mesmo para o estágio do misturador.
settings-radio-loop-running-gain-slider-no = Com um laço rodando, o controle deslizante de ganho não define mais o estágio que ele possui — o laço o sobrescreve.
settings-radio-dc-antenna-port = CC na porta de antena
settings-radio-t-12-bit-packing = Empacotamento de 12 bits
settings-radio-third-less-usb-traffic-leave = Um terço a menos de tráfego USB. Deixe ligado: este é um dispositivo USB 2.0 e a taxa máxima é 30 MB/s empacotada contra 40 sem empacotar. Vale ao reconectar.
settings-radio-dc-removal = Remoção de CC
settings-radio-remove-adc-s-offset = Remove o offset do ADC
settings-radio-turn-off-see-raw-hardware = Desligue para ver a saída bruta do hardware. Vale saber para onde vai o pico: o offset cai na borda da faixa, não no centro, porque o sinal é transladado por um quarto da taxa de amostragem no caminho.
settings-radio-every-command-exchanged-receiver-sample = Cada comando trocado com o receptor, a aritmética da taxa de amostragem e as primeiras amostras decodificadas como I/Q.
settings-radio-receive-only-24-1800-mhz = Somente recepção, 24–1800 MHz. Não precisa de SoapySDR nem de libairspy. O receptor, a taxa de amostragem e o empacotamento têm efeito em Aplicar; o restante vale à medida que você altera.
settings-radio-not-yet-verified-against-real-4 = Ainda não verificado em hardware real. Se apresentar mau funcionamento, envie o relatório de diagnóstico — ele contém cada comando trocado com o receptor, a aritmética de taxas e as primeiras amostras decodificadas como pares I/Q.
settings-radio-no-hydrasdr-rfone-found-press = Nenhum HydraSDR RFOne encontrado — pressione Buscar novamente
settings-radio-these-are-rates-receiver-turned = estas são as taxas que este receptor acabou tendo
settings-radio-receiver-only-reports-three-these = o receptor só informa três destas; as demais estão na tabela alternativa do firmware, e uma versão mais antiga pode não tê-las
settings-radio-rf-input = Entrada de RF
settings-radio-antenna-sma-only-socket-bias = A SMA de antena — o único conector com o bias tee por trás.
settings-radio-cable-socket-no-bias-tee = Um conector de cabo. Sem bias tee aqui; o hardware o coloca somente em ANT.
settings-radio-powers-active-antenna-preamp-down-2 = Alimenta uma antena ativa ou pré-amplificador pelo coaxial. Somente o conector ANT tem isso — as duas portas de cabo são entradas comuns.
settings-radio-third-less-usb-traffic-leave-2 = Um terço a menos de tráfego USB. Deixe ligado: este é um dispositivo USB 2.0 e a taxa máxima é 36 MB/s empacotada contra 48 sem empacotar. Vale ao reconectar.
settings-radio-one-these-boards-1d50-60a1 = Uma destas placas está em 1d50:60a1, o ID USB que os protótipos da HydraSDR compartilham com o Airspy R2 e o Mini. O sdroxide confere o firmware depois de abrir e informa se a interface errada foi escolhida, nos dois sentidos.
settings-radio-receive-only-24-1800-mhz-2 = Somente recepção, 24–1800 MHz. Não precisa de SoapySDR nem de libhydrasdr. O receptor, a taxa de amostragem e o empacotamento têm efeito em Aplicar; o restante vale à medida que você altera.
settings-radio-re-list-fobos-sdrs-no = Relistar os SDRs Fobos. Nenhum dispositivo é aberto, então é seguro pressionar durante a recepção.
settings-radio-no-fobos-sdr-found-press = Nenhum SDR Fobos encontrado — pressione Buscar novamente
settings-radio-input-2 = Entrada
settings-radio-through-tuner-lna-vga-mixer = Pelo sintonizador: LNA, VGA e a faixa de frequência do próprio misturador.
settings-radio-direct-sampling-hf1-antenna-input = Amostragem direta na entrada de antena HF1 — sem sintonizador, sem ganho de LNA/VGA, sintonizada inteiramente em software.
settings-radio-direct-sampling-hf2-antenna-input = Amostragem direta na entrada de antena HF2, igual à HF1.
settings-radio-both-real-adc-channels-once = Os dois canais reais do ADC ao mesmo tempo, combinados pelo filtro de diversidade abaixo — uma fonte de ruído anulada, ou dois caminhos com desvanecimento combinados.
settings-radio-direct-sampling-tuner-so-lna = Amostragem direta — o sintonizador (e portanto o ganho de LNA/VGA) não está no caminho. A taxa abaixo é um alvo para o conversor de descida em software, não uma configuração de hardware, então a taxa obtida pode diferir dela.
settings-radio-receiver-s-own-reported-rates = as taxas que este receptor informa
settings-radio-measured-one-real-unit-connect = medido em uma unidade real — conecte este receptor e Buscar novamente para ver a lista dele
settings-radio-clock-source = Fonte de clock
settings-radio-external-reference = Referência externa
settings-radio-diversity-taps = Derivações de diversidade
settings-radio-taps = {" "}derivações
settings-radio-one-tap-gain-phase-null = Uma derivação é um ganho e uma fase — um nulo em uma frequência que piora de cada lado, que é tudo o que um defasador analógico consegue fazer. Cada derivação adicional compra um período de amostra da diferença de caminho entre HF1 e HF2 que o filtro consegue equalizar, e é isso que transforma esse entalhe em uma faixa silenciosa de ponta a ponta.
settings-radio-receive-only-rf-port-tuner = Somente recepção. Porta RF (sintonizador), HF1/HF2 (amostragem direta, decodificada em software) e HF1+HF2 (os dois ao mesmo tempo, combinados pelo filtro de diversidade) funcionam todos. O receptor, a entrada e a taxa de amostragem têm efeito em Aplicar; ganho, fonte de clock e derivações valem à medida que você os altera — o modo, a taxa de adaptação e a retenção do filtro de diversidade ficam na caixa DIV da própria janela principal, depois que este rádio é selecionado.
settings-radio-no-hackrf-found-press-rescan = Nenhum HackRF encontrado — pressione Buscar novamente
settings-radio-front-end-amplifier-8-db = Amplificador de front end, em passos de 8 dB. Este é o estágio que muda a sensibilidade — e o estágio que sobrecarrega primeiro com uma antena real.
settings-radio-baseband-amplifier-after-mixer-2 = Amplificador de banda base depois do misturador, em passos de 2 dB. Aumente-o para um sinal fraco antes de recorrer ao LNA.
settings-radio-rf-amp = Amp RF
settings-radio-one-switch-shared-transmit-setting = Uma chave, compartilhada com a configuração de transmissão abaixo — o rádio aplica a que pertence à direção em que está entrando. Desligado costuma ser o certo com uma antena real.
settings-radio-hackrf-pro-chooses-itself-three = Um HackRF Pro escolhe isto sozinho — três quartos da taxa de amostragem, filtrada na FPGA — e ignora qualquer coisa que o computador peça.
settings-radio-leave-automatic-choosing-one-too = Deixe em Automático. Escolher uma muito estreita não suaviza só as bordas da faixa — ela retira o deslocamento do LO que mantém o pico de CC fora do seu sinal.
settings-radio-about-3-v-50-ma = Cerca de 3 V a 50 mA pelo coaxial, para uma antena ativa ou um pré-amplificador. Somente HackRF One ou Pro; o Jawbreaker e o rad1o não têm esse circuito.
settings-radio-remove-dc-mirror-image = Remover CC e a imagem espelhada
settings-radio-zero-if-radio-so-own = Este é um rádio de zero-IF, então o vazamento do próprio LO fica no meio da faixa, e o erro de quadratura do misturador coloca uma imagem espelhada sobre ele. Desligar isto mostra a saída bruta do hardware, que é a forma rápida de distinguir um problema de driver de um problema de DSP.
settings-radio-corrects-reference-oscillator = Corrige o oscilador de referência.
settings-radio-transmit = Transmitir
settings-radio-transmitter = Transmissor
settings-radio-off-default-while-off-backend = Desligado por padrão. Enquanto isto está desligado, o backend não publica nenhum canal de transmissão, então nada pode acionar o rádio. Vale ao reconectar.
settings-radio-tx-vga-gain = Ganho VGA de TX
settings-radio-transmit-driver-amplifier-drive-applied = O amplificador de driver de transmissão. A Excitação é aplicada digitalmente antes deste estágio, então deixe a excitação alta e defina o nível de saída aqui — reduzir a excitação faz o DAC rodar a uma fração da escala total e aumenta a intermodulação.
settings-radio-same-switch-receive-setting-above = A mesma chave da configuração de recepção acima, aplicada quando o rádio muda de direção.
settings-radio-transmit-armed-into-dummy-load = A transmissão está armada. Em carga fantasma até você tê-la medido: os harmônicos de um HackRF são fortes o bastante para exigir um filtro passa-baixas externo para a faixa em que você está, e ele é meio duplex, então a recepção para durante cada transmissão.
settings-radio-every-command-exchanged-radio-session = Cada comando trocado com o rádio nesta sessão, em ordem, inclusive a sequência em torno de cada acionamento.
settings-radio-dc-7-25-ghz-half = CC – 7,25 GHz, meio duplex. Não precisa de SoapySDR, libusb nem libhackrf. O rádio e a taxa de amostragem têm efeito em Aplicar; o restante vale à medida que você altera. As bordas de faixa são as do firmware: um HackRF é especificado para 1 MHz – 6 GHz (100 kHz em um Pro) e fica muito atenuado fora disso, mas sintoniza ali.
settings-radio-ask-sdrplay-api-service-device = Pede ao serviço SDRplay API a lista de dispositivos dele. Nada é aberto, então é seguro pressionar durante a recepção.
settings-radio-no-rsps-press-rescan-needs = nenhum RSP — pressione Buscar novamente (precisa do serviço SDRplay API)
settings-radio-both-tuners-running-api-fixes = Com os dois sintonizadores rodando, a API fixa o ADC em 6 MHz e devolve 2 Msps a partir de um F.I. baixo — então 2 Msps é a largura mais ampla, e as mais estreitas são essa taxa decimada. Tem efeito em Aplicar.
settings-radio-rates-below-2-msps-run = Taxas abaixo de 2 Msps fazem o ADC rodar a 2 Msps e decimam no serviço. Tem efeito em Aplicar.
settings-radio-reduced-adc-resolution = {"  "}(resolução reduzida do ADC)
settings-radio-if-bandwidth = Largura de banda de F.I.
settings-radio-tuner-s-analog-filter-auto = O filtro analógico do sintonizador. Automático escolhe o mais largo que cabe na taxa de amostragem. Tem efeito em Aplicar.
settings-radio-rsp-s-own-if-gain = O laço de ganho de F.I. próprio do RSP, rodado pelo serviço de API. Desligado devolve a você o controle deslizante de ganho de F.I. — a configuração para medição e modos digitais de sinal fraco.
settings-radio-agc-set-point = Ponto de ajuste do AGC
settings-radio-signal-level-loop-holds-adc = Nível de sinal em que o laço mantém o ADC. Mais baixo deixa mais folga para sinais fora do canal. Quão baixo ele pode ir depende da taxa de amostragem — acima de 8,064 Msps o conversor tem menos folga para dar, e acima de 9,216 Msps, menos ainda.
settings-radio-rsp-s-native-gain-unit = A unidade de ganho nativa do RSP: 20 dB é o ganho máximo (0 com a faixa estendida abaixo), 59 dB o mínimo. Vale imediatamente. Ignorado enquanto o AGC está rodando — o laço é dono deste valor então, e o S-metro mostra em que ele se fixou.
settings-radio-extended-if-range = Faixa de F.I. estendida
settings-radio-lets-if-gain-reduction-go = Deixa a redução de ganho de F.I. descer abaixo de 20 dB, até 0 — os últimos 20 dB de ganho que o receptor tem. Desligado é o padrão da própria API e o certo para a escuta comum, porque o fundo da faixa é onde um RSP mais facilmente sobrecarrega. Vale ligar para sinais fracos com o LNA já em 0. Também permite que o ponto de ajuste do AGC suba até 0 dBFS. Vale imediatamente.
settings-radio-allow-if-gain-reduction-below = Permitir redução de ganho de F.I. abaixo de 20 dB
settings-radio-front-end-attenuation-steps-0 = Atenuação do front end em passos: 0 é o ganho máximo, cada passo acima acrescenta mais atenuação. Algumas faixas têm menos passos — o driver limita e mantém a sua escolha para quando você sintonizar de volta, e a barra aqui termina no que a faixa em que o rádio está aceitará. Uma escolha mais alta feita em outra faixa é mantida e continua mostrada ao lado. Vale imediatamente.
settings-radio-t-0-max-gain = 0 = ganho máximo
settings-radio-reference-error-parts-per-million-2 = Erro de referência em partes por milhão, aplicado pelo próprio dispositivo. Vale imediatamente.
settings-radio-radio-s-tuner = O sintonizador deste rádio
settings-radio-tuner = Sintonizador
settings-radio-which-rspduo-s-two-tuners = Qual dos dois sintonizadores do RSPduo este rádio escuta. O outro pertence ao rádio configurado para ele — dê a esse o mesmo receptor e o outro sintonizador. Tem efeito em Aplicar.
settings-radio-which-rspduo-s-two-tuners-2 = Qual dos dois sintonizadores do RSPduo carrega a antena que você está escutando. O outro carrega a segunda antena. Tem efeito em Aplicar.
settings-radio-which-rspduo-s-two-tuners-3 = Qual dos dois sintonizadores do RSPduo rodar (um de cada vez). Tem efeito em Aplicar.
settings-radio-applies-immediately = Vale imediatamente.
settings-radio-fm-broadcast-notch = Notch de radiodifusão FM
settings-radio-hardware-notch-over-88-108 = Notch de hardware sobre a faixa de radiodifusão de 88–108 MHz, para quando um transmissor local sobrecarrega todo o resto. Vale imediatamente.
settings-radio-dab-notch = Notch DAB
settings-radio-hardware-notch-over-165-230 = Notch de hardware sobre a faixa DAB de 165–230 MHz. Vale imediatamente.
settings-radio-hdr-mode = Modo HDR
settings-radio-enable-below-2-mhz = Ativar abaixo de 2 MHz
settings-radio-rspdx-s-high-dynamic-range =
    O caminho de alta faixa dinâmica do RSPdx para LF/MF. Vale imediatamente.

    Ainda não funciona: no único RSPdx em que foi testado, ligá-lo silencia o receptor — em todo centro, filtro, porta, ganho e taxa testados. O que o modo precisa não está na superfície documentada da API, e descobri-lo é o trabalho de alguém. Deixe isto desligado, a menos que seja você quem está investigando.

    O que se sabe: não é um modo que segue o dial. O caminho tem um filtro analógico fixo, construído apenas nos centros listados no filtro abaixo, então sintonizar em qualquer outro lugar não funcionaria mesmo que o resto funcionasse.
settings-radio-hdr-filter = Filtro HDR
settings-radio-feed-4-7-v-dc = Alimentar ~4,7 V CC pelo coaxial
settings-radio-second-tuner = Segundo sintonizador
settings-radio-rspduo-two-whole-tuners-one = Um RSPduo são dois sintonizadores inteiros em uma placa, com clock de uma mesma referência. Rodando os dois, eles ouvem seus trechos no mesmo instante a partir do mesmo clock — o que torna coerentes duas antenas ligadas a eles, com uma fase relativa definida pelas linhas de alimentação e não pelo acaso, e permite combinar o par. Ou deixe-os separados: os sintonizadores sintonizam de forma independente, então o outro pode ser um segundo rádio na própria faixa.
settings-radio-run-both-tuners = Rodar os dois sintonizadores
settings-radio-puts-rspduo-api-s-dual = Coloca o RSPduo no modo de dois sintonizadores da API, em que o ADC fica fixo em 6 MHz e a maior largura é 2 Msps. Tem efeito em Aplicar, porque o modo é escolhido quando o dispositivo é aberto.
settings-radio-used = Usado para
settings-radio-what-other-tuner-takes-effect = Para que serve o outro sintonizador. Tem efeito em Aplicar — como rodar os dois, isto é escolhido quando a placa é aberta.
settings-radio-add-second-radio-settings-radio = Adicione um segundo rádio (Configurações → Rádio → +), dê a ele este mesmo receptor e o outro sintonizador do RSPduo, e configure-o para rodar os dois sintonizadores também — o rádio que abrir a placa a coloca no modo de dois sintonizadores, então ambos precisam esperar por isso. Os dois então sintonizam de forma independente: HF em uma aba e VHF na outra, a partir de uma só placa. Eles compartilham um clock de ADC, então ambos rodam na taxa de amostragem que o rádio que abriu a placa pediu, e nenhum dos dois pode transmitir.
settings-radio-what-do = O que fazer com ele
settings-radio-lna-state-2 = Seu estado de LNA
settings-radio-second-tuner-s-own-front = A atenuação de front end própria do segundo sintonizador. Ajuste para que as duas antenas mostrem aproximadamente o mesmo piso de ruído: este é o ajuste em que tudo o mais se apoia, porque a combinação pondera os dois ramos pelo ruído deles, e um segundo front end levado à sobrecarga entrega ao filtro uma cópia distorcida da interferência — que não pode ser subtraída de uma não distorcida. Vale imediatamente.
settings-radio-if-gain-reduction-2 = A redução de ganho de F.I. dele
settings-radio-second-tuner-s-if-gain = O ganho de F.I. do segundo sintonizador, na unidade nativa do RSP: 20 dB é o ganho máximo. Ignorado enquanto o AGC está rodando — e um ganho estável é o que o filtro quer, então desligar o AGC vale a pena para um nulo que você pretende manter.
settings-radio-filter-length = Comprimento do filtro
settings-radio-one-tap-gain-phase-null-2 = Uma derivação é um ganho e uma fase — um nulo em uma frequência que piora de cada lado, que é tudo o que um defasador analógico consegue fazer. Cada derivação adicional compra um período de amostra da diferença de caminho entre as duas antenas que o filtro consegue equalizar, e é isso que transforma esse entalhe em uma faixa silenciosa de ponta a ponta.
settings-radio-adaptation-2 = Adaptação
settings-radio-slow-steady-left-converging-inside = Lento e estável à esquerda, convergindo em menos de um segundo e visivelmente procurando à direita. Comece rápido para achar o nulo, depois mantenha-o.
settings-radio-stop-filter-moving-reach-moment = Pare o filtro de se mover. Use isto no momento em que um nulo aparecer: um filtro deixado adaptando se reapontará para o que ficar mais forte, o que, em uma faixa silenciosa, é a estação que você está ouvindo.
settings-radio-restart = Reiniciar
settings-radio-zero-filter-find-null-again = Zera o filtro e encontra o nulo de novo.
settings-radio-nothing-here-can-tell-wanted = Nada aqui consegue distinguir um sinal desejado de um indesejado — o filtro só sabe o que as duas antenas têm em comum. Em Cancelar, a segunda antena quer ouvir a fonte de ruído e o mínimo possível da faixa, ou vai cancelar a estação também, diligentemente. Em Combinar, as duas querem ouvir a mesma estação. A profundidade do nulo é registrada no log a cada poucos segundos.
settings-radio-not-yet-verified-against-rspduo = Ainda não verificado em um RSPduo: a operação com dois sintonizadores aqui foi escrita a partir da API da SDRplay, não medida no hardware.
settings-radio-no-receiver-listed-check-against = Nenhum receptor está listado para conferir isto — pressione Buscar novamente, ou leia o log para ver o que de fato abriu.
settings-radio-receive-only-1-khz-2 = Somente recepção, 1 kHz–2 GHz. Precisa do serviço SDRplay API do fabricante (sdrplay.com/api) — os RSPs posteriores ao RSP1 original não têm protocolo aberto. Dispositivo, taxa de amostragem, largura de banda e sintonizador do RSPduo têm efeito em Aplicar; o restante vale à medida que você altera.
settings-radio-board = Placa
settings-radio-first-one-found-2 = O primeiro encontrado
settings-radio-receive-chain = Cadeia de recepção
settings-radio-both-chains-tune-together-they = As duas cadeias sintonizam juntas — compartilham um sintetizador —, mas são front ends separados em conectores separados. Escolha aquela em que sua antena está, que é a configuração a usar quando uma cadeia recebeu a modificação de casamento de HF e a outra é de fábrica.
settings-radio-receive-gain = Ganho de recepção
settings-radio-one-combined-figure-which-limesuite = Um valor combinado, que o LimeSuite distribui entre o LNA, o TIA e o PGA por conta própria. Aceita decibéis inteiros, então qualquer coisa mais fina é truncada.
settings-radio-receive-port = Porta de recepção
settings-radio-automatic-follows-frequency-lnal-low = O automático segue a frequência: LNAL em baixa, LNAH em alta — a menos que um LimeRFE esteja conectado abaixo, que é um cabo em um conector, e então é LNAW em todas as frequências. Informe o conector ao qual o seu está ligado, se não for esse. A cadeia a que o conector pertence é o seletor acima.
settings-radio-t-0-follows-sample-rate-worth = 0 segue a taxa de amostragem. Vale deixar assim: um filtro mais estreito que um quarto da largura de faixa retira em silêncio o deslocamento do LO de zero-IF, o que devolve o vazamento do LO por cima do sinal que você está ouvindo.
settings-radio-following-rate = seguindo a taxa
settings-radio-corrections = Correções
settings-radio-host-iq-dc-correction = Correção I/Q / CC no computador
settings-radio-adaptive-image-dc-removal-side = Remoção adaptativa de imagem e de CC deste lado, além da calibração própria do chip. Desligá-la é a forma de um clique para distinguir um problema de driver de um problema de DSP.
settings-radio-calibrate-automatically = Calibrar automaticamente
settings-radio-runs-chip-s-own-dc = Roda a calibração própria de offset de CC e de imagem do chip quando o rádio é aberto, e de novo depois que o dial se estabiliza em uma nova faixa ou um conector diferente. Esses números são medidos em uma frequência e erram em outras, que é o que uma portadora no meio da faixa normalmente é. Custa cerca de um segundo cada vez.
settings-radio-calibrate-now = Calibrar agora
settings-radio-stalls-receiver-better-part-second = Trava o receptor por boa parte de um segundo.
settings-radio-second-aerial = Segunda antena
settings-radio-socket = Seu conector
settings-radio-same-first = Igual à primeira
settings-radio-gain-2 = Seu ganho
settings-radio-set-low-coupled-sample-your = Ajuste BAIXO. A amostra acoplada do seu próprio transmissor é um sinal forte, e uma cadeia de realimentação levada à compressão mede mal a curva do amplificador — ela ensina à correção a própria distorção. Comece no fundo e use o atenuador do acoplador.
settings-radio-set-so-both-aerials-show = Ajuste para que as duas antenas mostrem aproximadamente o mesmo piso de ruído. Este é o ajuste em que tudo se apoia: a combinação pondera os dois ramos pelo ruído deles, e uma segunda cadeia levada à compressão entrega ao filtro uma cópia distorcida da interferência — que não pode ser subtraída de uma não distorcida.
settings-radio-table-steps-2 = Passos da tabela
settings-radio-how-finely-correction-follows-amplifier = Com que precisão a correção segue a curva do amplificador. Mais passos acompanham um joelho mais agudo, mas cada um precisa ser aprendido a partir das amostras que caíram nele — e o topo de um histograma de amplitude de voz é fino. Trinta e dois basta para a curva suave que um amplificador de HF de fato tem. Alterá-lo reinicia a correção.
settings-radio-how-hard-each-block-feedback = Quanto cada bloco de realimentação move a correção. A curva de um amplificador não muda, então não há pressa — o meio tira dela o ruído de várias transmissões.
settings-radio-keep-correction-curve-learned-clean = Mantenha a correção como está. Uma curva aprendida em uma transmissão limpa vale a pena manter, e o amplificador não terá mudado até a próxima.
settings-radio-forget-correction-learn-again = Esquecer a correção e aprendê-la de novo.
settings-radio-directional-coupler-amplifier-s-output = Um acoplador direcional na saída do amplificador entra nesta cadeia, e o transmissor compara o que voltou com o que pretendia enviar — então envia o inverso da diferença, de modo que o que sai do amplificador fica reto. Uns vinte e poucos decibéis a menos de intermodulação nos QSOs dos outros, com o amplificador mantendo a potência. A correção fica em unidade até que a realimentação se alinhe com a transmissão, então um acoplador não conectado não custa nada; e ela nunca pode pedir ao conversor mais que a escala total, então um caminho de realimentação lendo bobagem não sobrecarrega nada. Como está indo, é registrado no log enquanto você transmite.
settings-radio-transmit-not-armed-so-there = A transmissão não está armada, então não há nada aqui para corrigir.
settings-radio-off-interface-publishes-no-transmit = Com isto desligado, a interface não publica nenhum canal de transmissão, então nada pode acionar o rádio.
settings-radio-transmit-gain = Ganho de transmissão
settings-radio-transmit-port = Porta de transmissão
settings-radio-board-s-two-transmit-sockets = Os dois conectores de transmissão da placa são duas redes de casamento diferentes, não duas tomadas na mesma: BAND1 (TX_1) cobre de 30 MHz a 1,9 GHz e BAND2 (TX_2) é a porta de 13 cm. Acionar uma faixa de amador abaixo de 23 cm pela BAND2 põe a transmissão em uma rede de casamento que deixa passar quase nada — todas as configurações leem correto, a excitação é a que você ajustou, e o medidor de potência fica em zero. Deixe isto em Automático e ele segue o dial.
settings-radio-limesdr-transmits-about-100-khz = Uma LimeSDR transmite de cerca de 100 kHz a 3,8 GHz sem filtragem própria. Use um passa-baixas, um canal LimeRFE apropriado ou uma carga fantasma.
settings-radio-limerfe-front-end = Front end LimeRFE
settings-radio-connected = Conectado por
settings-radio-limerfe-s-own-micro-usb = A porta micro-USB do próprio LimeRFE, não a do rádio. 9600 baud, fixo pelo firmware dele.
settings-radio-receive-connector = Conector de recepção
settings-radio-transmit-connector = Conector de transmissão
settings-radio-follow-dial = Seguir o dial
settings-radio-switch-filters-match-operating-frequency = Comuta os filtros para casar com a frequência de operação, antes de qualquer RF aparecer. Sintonizar dentro de uma só faixa não põe nada no link de controle.
settings-radio-relays = Relés
settings-radio-receive-attenuator = Atenuador de recepção
settings-radio-notch-filter = Filtro notch
settings-radio-fan = Ventilador
settings-radio-worth-having-any-sustained-transmitting = Vale ligar para qualquer transmissão contínua.
settings-radio-automatic-board-receives-switched-transmit = No Automático a placa recebe, e é comutada para transmitir na tecla para baixo e de volta na tecla para cima — em qualquer cabeamento. Os canais de amador dela têm um filtro com uma chave de transmissão/recepção de cada lado, então uma placa solicitada para os dois ao mesmo tempo põe essa chave no transmissor e para de ouvir qualquer coisa. O canal de 30 MHz só é alcançável pelo J5, que é um conector para as duas direções; acima dele, transmitir pelo J4 mantém o caminho de recepção fora do conector que o amplificador está acionando.
settings-radio-limesdr-support-has-not-been = O suporte à LimeSDR não foi verificado em hardware. Este é o rastro da última sessão, para um relatório de problema.
settings-radio-gains-filters-corrections-antenna-sockets = Ganhos, filtros, correções, os conectores de antena e todo controle do LimeRFE — conectores, faixa, relés, atenuador, notch e ventilador — valem imediatamente. A placa, a cadeia de recepção, a taxa de amostragem, o armamento da transmissão e a conexão do LimeRFE têm efeito em Aplicar.

## Janela de Configurações, aba Rádio: mensagens com valores

settings-radio-fill-from = PREENCHER DE { $name }
settings-radio-decimation-auto = Automático (mais próximo de { $khz } kHz)
settings-radio-decimation-stage-server = Estágio { $stage } — a taxa do servidor ÷ { $divisor }
settings-radio-decimation-stage = Estágio { $stage } — taxa ÷ { $divisor }
settings-radio-kiwi-receive-only = Somente recepção — esta é a antena de outra pessoa. O panadaptador é os cerca de 12 kHz que o receptor envia como I/Q, que segue o dial; a visão de banda acima é a cascata própria dele, e sintonizar por ela sintoniza o receptor. Cerca de { $rate } kB/s enquanto conectado. Pressione "Aplicar / reconectar" para trocar sem reiniciar.
settings-radio-connected-detail = Conectado: { $detail }
settings-radio-failed = Falhou: { $error }
settings-radio-pluto-buffer = ≈ { $ms } ms, { $kib } KiB por transferência
settings-radio-driver-settings = Configurações de { $driver }
settings-radio-rx888-coverage = Cobertura de 0–{ $mhz } MHz, { $mbytes } MB/s pelo USB
settings-radio-rx888-digitised = de { $mhz } MHz digitalizados
settings-radio-elad-16-bit = { $khz } kHz (amostras de 16 bits)
settings-radio-elad-pad = Atenuador de { $db } dB ativado
settings-radio-hackrf-rx-preamp = Pré-amplificador de { $db } dB na recepção
settings-radio-hackrf-tx-preamp = Pré-amplificador de { $db } dB na transmissão
settings-radio-device-not-found = { $device } (não encontrado)
settings-radio-lime-chain = Cadeia { $n } (RX{ $n }_*)
settings-radio-lime-chain-tx = Cadeia { $n } (RX{ $n }_* / TX{ $n }_*)
settings-radio-lime-second-chain = A outra cadeia de recepção da placa, nos conectores { $sockets }. Ela compartilha o sintetizador, então ouve a mesma faixa no mesmo instante que a primeira — o que permite que leve uma segunda antena, ou uma amostra do seu próprio transmissor.
settings-radio-tx-blocked = A transmissão está bloqueada: { $reason }
settings-radio-rfe-rx-fallback = Recebendo em { $port }, estas caem para o caminho de banda larga sem filtro: { $bands }.
settings-radio-rfe-tx-fallback = Transmitindo em { $port }, estas caem para o caminho de banda larga — sem amplificador de faixa e sem filtragem: { $bands }.

## Barra superior e faixa de controles (src/app/top_bar.rs)

topbar-how-far-one-press-moves = Quanto um toque move o dial. Toque para dar o próximo passo: 10 Hz, 100 Hz, 500 Hz, 1, 2,5, 5, 9, 10 e 25 kHz. Desligue a linha em Configurações › UI.
topbar-volume-gain-agc-squelch-noise = Volume, ganho, AGC, squelch, os controles de ruído e o visor RDS na radiodifusão FM
topbar-receiver = RECEPTOR
topbar-vfo-b-split-rit-xit = VFO A/B, split e os deslocamentos RIT/XIT
topbar-tuning = SINTONIA
topbar-diversity-filter-which-way-combines = O filtro de diversidade: de que forma combina as duas antenas, com que rapidez se adapta e como mantê-lo onde está
topbar-diversity = DIVERSIDADE
topbar-second-receiver-s-frequency-mode = A frequência, o modo, o filtro e o nível do segundo receptor
topbar-sub-receiver = SUB-RECEPTOR
topbar-transceiver-s-own-aerial-socket = O conector de antena próprio do transceptor, e a chave de energia dele
topbar-radio = RÁDIO
topbar-tune-voice-keyer-drive-mic = Tune, o manipulador de voz, e os níveis de excitação e de microfone
topbar-transmit = TRANSMISSÃO
topbar-tune = {" "}TUNE{" "}
topbar-panadapter-two-layers-their-speeds = O panadaptador — suas duas camadas, as velocidades e o detalhe delas, o peak hold — além do contraste da cascata, o tamanho da FFT e os skimmers
topbar-display = EXIBIÇÃO
topbar-logbook-spots-awards-memories-scanner = Diário de contatos, spots, prêmios, memórias, o scanner, as configurações e o manual
topbar-system = SISTEMA
topbar-linked = Conectado
topbar-not-linked = Não conectado
topbar-bar-trace = barra / traço
topbar-needle-bar-trace = agulha / barra / traço
topbar-swap-vfos = Trocar VFOs
topbar-copy-b = Copiar A para B
topbar-clip = Área de transferência: esquerdo enche / direito esvazia
topbar-clip-full = Área de transferência: esquerdo cola { $freq } ({ $mode }) / direito esvazia
topbar-second-receiver-right-ear-tunes = Segundo receptor, no ouvido direito. Ele sintoniza independentemente de A/B — seus controles aparecem no módulo SUB, e sua banda de passagem na cascata.
topbar-band-plan = , do plano de faixas
topbar-repeater-shift-following-band-plan = Deslocamento de repetidora: seguindo o plano de faixas, que não tem deslocamento para esta frequência — simplex
topbar-repeater-shift-transmit-above-below = Deslocamento de repetidora: transmitir acima ou abaixo do dial, para trabalhar uma repetidora. Simplex agora
topbar-repeater-shift = DESLOCAMENTO DE REPETIDORA
topbar-auto = AUTO
topbar-take-shift-band-plan-dial = Obter o deslocamento a partir do plano de faixas conforme o dial se move. Ele só age dentro de uma sub-faixa de saída de repetidora — em qualquer outro lugar, deixa o rádio em simplex, para que os canais de chamada fiquem em simplex.
topbar-offset = Deslocamento
topbar-repeater-tone-ctcss-dcs-goes = Tom de repetidora: o CTCSS/DCS que vai por baixo da voz, a rajada de 1750 Hz, e o squelch de tom de recepção
topbar-transmit-tone = TOM DE TRANSMISSÃO
topbar-set-here-sent-nfm-sub = definido aqui, enviado em NFM — a sinalização subaudível é de um canal FM
topbar-normal = NORMAL
topbar-invert = INVERTER
topbar-bit-order-dcs-encoded-here = A ordem de bits com que o DCS é codificado aqui é transcrita do padrão e nunca foi conferida com uma repetidora — se não abrir, tente CTCSS
topbar-t-1750-hz-burst = RAJADA DE 1750 HZ
topbar-every-over = A CADA TRANSMISSÃO
topbar-open-burst-every-time-transmitter = Abre com a rajada toda vez que o transmissor é acionado
topbar-send = ENVIAR
topbar-send-burst-now-keying-transmitter = Envia a rajada agora — acionando o transmissor pelo tempo dela se ele ainda não estiver acionado
topbar-nfm-only-burst-fm-repeater = Somente NFM: a rajada é o abridor de porta de uma repetidora FM
topbar-receive-tone-squelch = SQUELCH DE TOM DE RECEPÇÃO
topbar-require-receive-what-station-transmits = Exigir na recepção o que esta estação transmite
topbar-hide-docked-band-selector-click = Ocultar o seletor de faixa acoplado — clique de novo para trazê-lo de volta
topbar-show-docked-band-selector = Mostrar o seletor de faixa acoplado
topbar-dock = ACOPLAR
topbar-keep-band-mode-selector-open = Manter o seletor de faixa e de modo aberto ao lado da cascata, em vez de fechar este pop-up a cada vez
topbar-band-mode = FAIXA E MODO
topbar-hide-band-chip-brings-back = Ocultar — o chip de faixa o traz de volta
topbar-undock = DESACOPLAR
topbar-return-selector-top-bar-popup = Devolver o seletor ao pop-up da barra superior
topbar-dec-off = DEC desligado
topbar-vol = Vol
topbar-counted-steps-not-decibels-what =
    {""}

    Contado em passos, não em decibéis: o que um passo vale é assunto do próprio receptor, e em um SDRplay depende também da faixa. Certo continua sendo mais ganho.
topbar-gain = Ganho


topbar-plutosdr-s-own-agc-setting =
    {""}

    O AGC próprio do PlutoSDR é que define este ganho, então mover o controle deslizante não faz nada. Defina Configurações → Rádio → AGC como Manual para controlá-lo aqui.
topbar-agc-hang-time-click-cycle = Tempo de retenção do AGC — clique para alternar: Desligado / Lento / Médio / Rápido
topbar-radio-s-own-agc-click = O AGC próprio do rádio — clique para alternar: Lento / Médio / Rápido. Não pode ser desligado daqui.
topbar-man = Man
topbar-manual-audio-gain-used-while = Ganho de áudio manual, usado enquanto o AGC está desligado. Começa do nível que o AGC mantinha quando foi desligado.
topbar-radio-s-own-squelch-sent = O squelch próprio do rádio, enviado pelo link de controle. Esta é a porta pela qual o áudio realmente passa — o squelch por software só fecharia ainda mais sobre o que o equipamento já deixou passar.
topbar-max-gain = ganho máximo
topbar-manual-gain = ganho manual
topbar-noise-reduction = redução de ruído
topbar-impulse-noise-blanker = Limitador de ruído impulsivo
topbar-auto-notch-cancel-constant-tone = Notch automático: cancela os elementos de tom constante (heterodinos)
topbar-noise-reduction-off-click-switch = Redução de ruído desligada — clique para ligá-la, ou escolha um motor nas linhas abaixo
topbar-binaural-audio-not-while-sub = Áudio binaural — não enquanto o sub-receptor estiver com o ouvido direito
topbar-binaural-audio-passband-spread-across = Áudio binaural: a banda de passagem é espalhada entre os dois ouvidos, então sinais em tons diferentes vêm de direções diferentes, e o que você sintoniza flutua de um lado para o outro. Clique para voltar ao mono
topbar-binaural-cw-spread-passband-across = CW binaural: espalha a banda de passagem entre os dois ouvidos, para que sinais em tons diferentes venham de direções diferentes — uma pilha de chamadas vira vários lugares em vez de uma nota lotada, e sintonizar uma estação a faz flutuar. Melhor com fones
topbar-binaural-audio-spread-passband-across = Áudio binaural: espalha a banda de passagem entre os dois ouvidos. Na voz, o ruído se espalha por toda a imagem enquanto a estação fica no meio dela, o que é mais fácil de ouvir por uma hora — ao custo de a própria voz ficar espalhada. Melhor com fones
topbar-mute = SILENCIAR
topbar-record-audio-raw-i-q = Gravar o áudio, o I/Q bruto, ou os dois
topbar-wfm-stereo-forced-off-click = Estéreo WFM forçado desligado — clique para estéreo automático
topbar-wfm-stereo-pilot-locked-click = Estéreo WFM: piloto travado. Clique para forçar mono
topbar-wfm-stereo-automatic-no-pilot = Estéreo WFM: automático, sem piloto nesta estação
topbar-rds-data-arriving-click-see = Chegam dados RDS — clique para vê-los
topbar-rds-no-data-station-click = RDS — sem dados nesta estação. Clique assim mesmo para os diagnósticos do decodificador
topbar-drm-decoding-click-details = DRM decodificando — clique para os detalhes
topbar-drm-click-decoder-s-state = DRM — clique para o estado do decodificador
topbar-hd-radio-decoding-click-details = HD Radio decodificando — clique para os detalhes
topbar-hd-radio-click-decoder-s = HD Radio — clique para o estado do decodificador
topbar-presets = PREDEFINIÇÕES
topbar-preset-right-click = Clique com o botão direito para alterar esta predefinição
topbar-preset-edit = EDITAR PREDEFINIÇÃO
topbar-preset-label = Rótulo
topbar-preset-label-hint = O que o botão mostra. Até 8 caracteres.
topbar-preset-lo = Inf
topbar-preset-hi = Sup
topbar-preset-use-current = Usar atual
topbar-preset-use-current-hint = Usar as bordas do filtro atual do receptor
topbar-preset-save = Salvar
topbar-preset-reset = Restaurar
topbar-preset-reset-hint = Restaurar a predefinição original deste modo
topbar-preset-cancel = Cancelar
topbar-width-per-sideband = LARGURA POR BANDA LATERAL
topbar-width = LARGURA
topbar-passband-width-hertz-type-drag = Largura da banda de passagem em hertz — digite-a ou arraste. Clique no campo para informar um valor exato.
topbar-edges = BORDAS
topbar-low-edge-hz-carrier = Borda inferior, em Hz a partir da portadora
topbar-high-edge-hz-carrier = Borda superior, em Hz a partir da portadora
topbar-noise-reduction-voice-off-click = Redução de ruído (voz), desligada — clique para escolher um motor
topbar-record-audio = GRAVAR ÁUDIO
topbar-mp3 = ● MP3
topbar-what-you-hear-mp3-receiver = O que você ouve, em MP3: o receptor em um canal e a sua própria transmissão no outro. Esta é a gravação de um QSO.
topbar-mono = MONO
topbar-mixed-one-channel-click-two = Misturado em um canal — clique para dois
topbar-two-channels-rx-left-tx = Dois canais: RX à esquerda, TX à direita — clique para um canal misturado
topbar-quick-clip = CLIP RÁPIDO
topbar-stop-after = PARAR APÓS
topbar-no-stop = sem parada
topbar-auto-record = GRAVAÇÃO AUTOMÁTICA
topbar-set-squelch-auto-record-needs = Defina um squelch — a gravação automática precisa de um para reconhecer o silêncio
topbar-auto-record-needs-sdr-front = A gravação automática exige um front end SDR com squelch por software
topbar-stop-following-squelch = Parar de seguir o squelch
topbar-waiting-signal = aguardando um sinal
topbar-record-spectrum = GRAVAR ESPECTRO
topbar-i-q-wav = ● I/Q WAV
topbar-i-q-wav-2 = I/Q WAV
topbar-raw-spectrum-receiver-delivering-stereo = O espectro bruto que o receptor entrega, como WAV estéreo de ponto flutuante de 32 bits (RF64 acima de 4 GB), que SDR#, SDRuno, HDSDR e SDRangel abrem — e que o próprio sdroxide reproduz com --file. Grande: 8 bytes por amostra.
topbar-radio-hands-over-demodulated-audio = Este rádio entrega áudio demodulado, então não há I/Q para gravar.
topbar-what-capture-costs-sample-rate = Quanto uma captura custa nesta taxa de amostragem
topbar-engine = MOTOR
topbar-no-noise-reduction-decoders-never = Sem redução de ruído — os decodificadores nunca a viram de qualquer forma
topbar-strength = INTENSIDADE
topbar-tone = TOM
topbar-ctcss-dcs-no-sub-audible = CTCSS / DCS — nenhum tom subaudível neste sinal
topbar-tone-squelch = SQUELCH DE TOM
topbar-carrier-squelch-open-any-signal = Squelch de portadora: abre com qualquer sinal
topbar-require-tone-currently-being-received = Exigir o tom que está sendo recebido agora
topbar-ctcss = CTCSS
topbar-any-dcs = QUALQUER DCS
topbar-open-any-dcs-coded-signal = Abre com qualquer sinal codificado em DCS. Qual dos 104 códigos ele carrega não pode ser lido de forma confiável aqui, então não há nada mais fino para escolher
topbar-which-socket-back-radio-receiving =
    Qual conector na traseira o rádio está recebendo — o próprio comando ANT dele, a mesma configuração do botão ANT do painel frontal. Clique para avançar ao próximo.

    A escolha é lembrada por faixa, e restaurada na próxima vez que o dial entrar nessa faixa.
topbar-radio-s-separate-receiving-antenna =
    A antena de recepção separada do rádio, ligada ao caminho de recepção ou desligada dele — a configuração RX ANT própria dele. A antena no conector principal continua na transmissão de qualquer forma.

    O próprio rádio lembra isto por faixa, então o sdroxide o lê de volta após cada troca de faixa, em vez de restaurar o que viu por último: clicar aqui é a única coisa que o move.
topbar-radio-s-own-power-switch =
    A chave de energia própria do rádio, pelo link de controle, e a única liga/desliga real do programa — não a chave LINK do sdroxide, que fecha a ponta do sdroxide e deixa o rádio funcionando.

    Para que LIGAR alcance alguma coisa, a ponta de controle do rádio precisa ficar acordada enquanto ele está desligado: Network Control pela LAN, ou uma porta CI-V ainda alimentada pela rede em um equipamento desligado pelo painel frontal.
topbar-switch-radio = Ligar o rádio
topbar-switch-radio-off-audio-meters = Desligar o rádio. O áudio e os medidores param com ele.
topbar-what-second-aerial-click-swap =
    Para que serve a segunda antena — clique para trocar.

    CANCELAR a subtrai da primeira, no ganho, fase e atraso que fazem uma fonte de ruído local se alinhar nas duas: a forma em DSP de um defasador cancelador de ruído. COMBINAR soma as duas na fase que reforça, ponderadas pelo quão bem cada uma ouve — recepção por diversidade, que preenche os desvanecimentos.
topbar-hold = MANTER
topbar-stop-filter-moving-reach-moment = Pare o filtro de se mover. Use isto no momento em que um nulo aparecer: um filtro deixado adaptando se reapontará para o que ficar mais forte, o que, em uma faixa silenciosa, é a estação que você está ouvindo.
topbar-restart = REINICIAR
topbar-zero-filter-find-null-again = Zera o filtro e encontra o nulo de novo.
topbar-adapt = Adaptar
topbar-how-fast-filter-chases-slow = Com que rapidez o filtro persegue: lento e estável à esquerda, convergindo em menos de um segundo e visivelmente procurando à direita. Comece rápido para achar o nulo, depois MANTENHA-o.
topbar-where-sub-receiver-listens-shift = Onde o sub-receptor escuta. Shift-clique na cascata, ou arraste dentro da banda de passagem do sub, para movê-lo.
topbar-dial = ←DIAL
topbar-move-sub-receiver-main-dial = Mover o sub-receptor para o dial principal
topbar-dial-2 = DIAL←
topbar-move-main-dial-sub-receiver = Mover o dial principal para o sub-receptor
topbar-filter = Filtro
topbar-sub-receiver-passband-edges-hz = Bordas da banda de passagem do sub-receptor, em Hz
topbar-sub-receiver-level-plays-right = Nível do sub-receptor (toca no ouvido direito)
topbar-ptt = {" "}PTT{" "}
topbar-voice-keyer-record-transmit-stored = Manipulador de voz: gravar e transmitir mensagens armazenadas
topbar-drive = Excitação
topbar-tune-2 = Tune
topbar-tx-audio-processing-microphone-click = O Áudio TX está processando o microfone — clique para abri-lo
topbar-click-open-tx-audio-gate = Clique para abrir o Áudio TX: gate, EQ, compressor e limitador para o microfone
topbar-mic = Mic
topbar-cessb = CESSB
topbar-microphone-gain = Ganho do microfone
topbar-deviation-how-far-mode-s =
    Desvio: quanto a rajada deste modo oscila um rádio que a modula por conta própria. Um transmissor FM converte nível de áudio em oscilação de frequência e não tem ALC para segurá-la, então a escala total em uma entrada de dados configurada para voz desvia demais — o que soa perfeitamente normal para quem ouve e não decodifica para ninguém.

    Mantido por modo, então um desvio definido para pacote de 1200 bauds nunca vai parar no FT8.
topbar-transmit-audio-how-hard-mode =
    Áudio de transmissão: com que força este modo aciona o modulador de um rádio que modula o que enviamos a ele — um equipamento CAT pela placa de som, um FLEX, um Icom pela porta de rede. Reduza até que o ALC do equipamento mal se mova e defina a potência no rádio; ALC trabalhando em um modo digital de envelope constante é o que espalha o sinal.

    Excitação não é este controle: nesses rádios, a Excitação alcança o registrador de potência do equipamento e nunca toca no áudio dele.

    Mantido por modo — FT8, RTTY, PSK e MCW mantêm cada um o seu.
topbar-tx-audio = Áudio TX
topbar-cw-psk-rtty-skimmers-decode = Skimmers de CW / PSK / RTTY — decodificam sinais por toda a faixa e os marcam na cascata
topbar-skimmers = SKIMMERS
topbar-run-skimmer = Executar este skimmer
topbar-minimum-snr-decoded-signal-needs = SNR mínima que um sinal decodificado precisa ter para ser marcado
topbar-needs-wideband-iq-source = precisa de uma fonte I/Q de banda larga
topbar-cw-decoder = DECODIFICADOR CW
topbar-how-many-signals-model-reads = Quantos sinais o modelo lê ao mesmo tempo. Os demais mantêm a marcação, mas não carregam texto.
topbar-spots = SPOTS
topbar-save-skimmer-s-spot-list = Salvar a lista de spots do skimmer em um arquivo
topbar-show-hide-full-band-waterfall = Mostrar/ocultar a faixa de cascata da banda inteira acima do panadaptador — tudo o que este receptor consegue ver de uma vez
    Receptor de banda estreita: SCAN → PANO varre uma faixa e monta aqui uma visão geral.
topbar-spectrum-waterfall-either-layer-both = Espectro e cascata — uma das camadas, as duas, ou nenhuma. Acesa quando as duas estão visíveis.
topbar-spectrum = ESPECTRO
topbar-show-spectrum = MOSTRAR ESPECTRO
topbar-draw-spectrum-line-across-top = Desenha a linha do espectro no topo do panadaptador. Desligada, a cascata ocupa toda a altura.
topbar-peak-hold = PEAK HOLD
topbar-trace-highest-level-each-column = Traça o nível mais alto que cada coluna atingiu sobre a linha ao vivo, decaindo de volta — o que a banda fez enquanto você olhava para outro lado
topbar-draw-spectrum-receding-surface-instead = Desenha o espectro como uma superfície que recua, em vez de uma linha chapada: o espectro mais recente na frente, os anteriores fluindo para longe de você. Os últimos segundos da banda como uma paisagem — um portadora que aparece e some é uma crista, e não uma linha que treme. A grade e o peak hold pertencem à linha chapada e não são desenhados nela.
topbar-how-quickly-spectrum-line-follows = Com que rapidez a linha do espectro acompanha a banda. Mais lento faz a média de mais quadros uns com os outros: uma linha mais estável, e uma portadora fraca que fica parada tempo suficiente para ser lida. A cascata não é afetada por isso — essas linhas recebem todos os quadros de qualquer forma.
topbar-waterfall = CASCATA
topbar-show-waterfall = MOSTRAR CASCATA
topbar-draw-scrolling-waterfall-below-spectrum = Desenha a cascata em rolagem abaixo do espectro. Desligada, a linha do espectro ocupa toda a altura.
topbar-how-fast-waterfall-scrolls-lines = Com que rapidez a cascata rola, em linhas por segundo: Lento 5, Médio 28, Rápido 56, Mais rápido 112, Mais rápido ainda 224. O motor cronometra estes sozinho, então os dois mais rápidos são detalhe real, e não a mesma linha desenhada duas vezes — na medida em que o receptor consegue alimentá-los: uma linha nunca pode mostrar mais de uma transformada, e um front end estreito gera apenas algumas dezenas por segundo. Eles custam histórico, já que a cascata guarda um número fixo de linhas — 73 segundos em Médio, 9 em Mais rápido ainda.
topbar-decode-labels = RÓTULOS DE DECODIFICAÇÃO
topbar-mark-every-decoded-station-waterfall = Marca cada estação decodificada na cascata com o indicativo, na frequência em que foi ouvida — FT8, FT4 e os outros modos por slots. Uma boa abertura põe trinta delas pela faixa duas vezes por minuto, sobre os traços que você está lendo; desligue-as e a cascata é só a banda. A lista de decodificações ao lado ainda tem todos os indicativos.
topbar-switch-3d-set = Ligue o 3D para definir isto
topbar-how-3d-spectrum-drawn-only = Como o espectro 3D é desenhado. Só a forma difere — as duas renderizações mostram os mesmos espectros, e ambas ocultam o que está atrás delas, então um sinal forte à frente fica de pé sobre a faixa que cobre.
topbar-lines = LINHAS
topbar-one-trace-per-remembered-spectrum = Um traço por espectro lembrado, na cor da própria linha chapada, cada um ocultando os que estão atrás. A forma sem os níveis, e a leitura a escolher quando a forma é o que se procura — custa um pouco mais que a sólida, em vez de menos, porque desenha a mesma superfície e depois contorna cada crista por cima dela.
topbar-solid = SÓLIDO
topbar-filled-surface-coloured-waterfall-s = Uma superfície preenchida, colorida pela paleta da cascata, para que o nível esteja na cor e também na altura, e as duas metades do panadaptador concordem sobre como é um sinal forte. Mude a paleta em Configurações › Exibição.
topbar-how-fast-3d-spectrum-flows = Com que rapidez o espectro 3D flui para longe de você, em linhas por segundo: Lento 6, Médio 12, Rápido 24, Mais rápido 48. A superfície tem uma profundidade fixa de linhas, então isto também define quanto tempo ela guarda. Mais lento é uma memória maior e uma superfície que rasteja; mais rápido é uma mais curta que se move. A cascata mantém a própria taxa de rolagem, abaixo.
topbar-how-many-columns-panadapter-waterfall = Quantas colunas o panadaptador e a sua cascata são desenhados. Automático lê o renderizador desta máquina e o tamanho do panadaptador e escolhe o máximo que ele pode suportar — uma tela 4K quer 4096. Cada coluna é um byte em cada quadro, então um cliente conectado a uma estação remota paga pelo detalhe no próprio link: 4096 colunas a 60 fps são cerca de um quarto de megabyte por segundo, o dobro da largura padrão.
topbar-read-machine-screen-take-most = Lê esta máquina e esta tela, pega o máximo que elas aguentam e as acompanha se mudarem
topbar-width-force-right-now-whatever = A largura em vigor agora, seja qual for a que a linha acima pede
topbar-keep-floor-ceiling-set-best = Mantém o piso/teto definidos para o melhor contraste da cascata — reajustados suavemente após uma troca de faixa, um deslocamento ou zoom, e quando os níveis derivam. Ligue para ajustar de imediato; desligue para manter os níveis onde você os definiu.
topbar-keep-tuned-frequency-middle-panadapter = Mantém a frequência sintonizada no meio do panadaptador: a janela desliza sob o dial, em vez de a imagem saltar uma faixa inteira quando você sintoniza além da borda. Ligue para centralizar de imediato; desligue para deslocar e dar zoom onde quiser.
topbar-spectrum-floor-ceiling-fft-size = Piso / teto do espectro e tamanho da FFT
topbar-level-drawn-bottom-spectrum-darkest = O nível desenhado na base do espectro e como a cor mais escura da cascata. Aumente até que o piso de ruído apenas escureça. Com o AJUSTAR ligado, ele desloca o nível do AJUSTAR, que continua ligado.
topbar-ceil = teto{" "}
topbar-waterfall-mean-not-peak = A cascata como potência média (Welch) em vez do pico: ruído calmo, os mesmos sinais. Um ponto CW mais curto que uma linha ou uma portadora mais estreita que uma coluna aparece mais fraco.
topbar-welch-time-constant = Por quanto tempo o WELCH faz a média, para a cascata e a linha do espectro: mais longo é mais calmo e mostra uma mudança mais tarde.
topbar-waterfall-reassign = REASSIGN: a cascata como espectrograma reatribuído (Auger e Flandrin 1995): a potência de cada célula é movida para a frequência em que está centrada, de modo que uma portadora ou um tom é uma linha nítida em vez de uma faixa borrada. A alternativa ao WELCH; o ruído fica igualmente calmo.
topbar-reassign-dim = Quanto mais escuro o REASSIGN desenha o que não moveu para uma linha: o ruído e o que fica abaixo do limiar. 0 desenha como o WELCH; mais alto deixa as linhas sobre fundo escuro.
topbar-level-drawn-top-bring-down = O nível desenhado no topo. Reduza até que o sinal mais forte que lhe interessa atinja a cor plena. Com o AJUSTAR ligado, ele desloca o nível do AJUSTAR, que continua ligado.
topbar-fit-shift = Quanto o nível do AJUSTAR está deslocado neste controle. O AJUSTAR mantém o deslocamento em todas as bandas; ligá-lo de novo volta a zero.
topbar-fft-size = TAMANHO DA FFT
topbar-flip = INVERTER
topbar-scroll-waterfall-upwards-newest-row = Rola a cascata para cima — linha mais recente embaixo (V)
topbar-logbook-all-qsos-digital-manual = Diário de contatos — todos os QSOs (digitais + manuais)
topbar-live-spots-dx-cluster-pota = Spots ao vivo — cluster DX, POTA, SOTA, PSK Reporter
topbar-award-tracking-dxcc-was-waz = Acompanhamento de prêmios — DXCC / WAS / WAZ / locators
topbar-band-conditions-published-forecast-beside = Condições de faixa — a previsão publicada ao lado do que de fato foi ouvido em cada faixa
topbar-satellite-qo-100-beacon-calibration = Satélite — a calibração pela baliza do QO-100 está rodando
topbar-satellite-doppler-tracking-qo-100 = Satélite — rastreamento Doppler, e a calibração pela baliza do QO-100
topbar-ism-band-devices-decoding-now = Dispositivos de faixa ISM — decodificando agora, esteja esta janela aberta ou não. Desligue-os com DECODIFICANDO dentro da janela.
topbar-ism-band-devices-weather-sensors = Dispositivos de faixa ISM — sensores meteorológicos, medidores e automação residencial ouvidos ao seu redor
topbar-public-sdrs-internet-browse-kiwisdr = SDRs públicos na internet — navegue pelos diretórios KiwiSDR e SpyServer e abra um como rádio
topbar-winlink-radio-email = E-mail por rádio Winlink
topbar-memory-channels = Canais de memória
topbar-scanner-stopped-signal = Scanner — parado em um sinal
topbar-scanner-running = Scanner — em execução
topbar-scan-memory-channels-frequency-range = Varre canais de memória ou uma faixa de frequências
topbar-hfdl-ground-network-decode-log = Rede terrestre HFDL — o log de decodificação e o mapa de aeronaves, abaixo da cascata. Ligue o decodificador com ESCUTAR dentro do painel.
topbar-hfdl-ground-network-decoding-now = Rede terrestre HFDL — decodificando agora. Abra o painel.
topbar-hfdl-ground-network-aircraft-shortwave = Rede terrestre HFDL — o enlace de dados de ondas curtas das aeronaves, um canal de escuta por vez
topbar-grid-tracker-worked-maidenhead-squares = Rastreador de locators — quadrados Maidenhead trabalhados num mapa, com o que é ouvido
topbar-amplifier-set-up-hardrock-under = Amplificador — configure um Hardrock em Configurações → Amplificador
topbar-settings-device-gains-antennas-audio = Configurações — ganhos dos dispositivos, antenas, dispositivos de áudio
topbar-user-manual-f1 = Manual do usuário (F1)
topbar-range-was-typed-settings-radio = Essa faixa foi digitada em Configurações ▸ Rádio ▸ Faixa de RX, não informada pelo rádio. Amplie-a, ou esvazie a caixa para usar o que o dispositivo informa sobre si mesmo.
topbar-what-device-reports-about-itself = Isto é o que o dispositivo informa sobre si mesmo. Se o rádio cobre esta faixa, informe a faixa real dele em Configurações ▸ Rádio ▸ Faixa de RX.
topbar-band = FAIXA
topbar-recent = RECENTES
topbar-recent-empty =
    Preenche após 3 s em um local
topbar-mode = MODO
topbar-digital = DIGITAL

## Barra superior: os chips de janela da caixa Sistema.

topbar-chip-log = LOG
topbar-chip-spots = SPOTS
topbar-chip-awards = PRÊMIOS
topbar-chip-bands = FAIXAS
topbar-chip-sat = SAT
topbar-chip-ism = ISM
topbar-chip-public-sdr = SDR PÚBLICO
topbar-chip-mail = E-MAIL
topbar-chip-mem = MEM
topbar-chip-scan = SCAN
topbar-chip-hfdl = HFDL
topbar-chip-grid = GRID
topbar-chip-amp = AMP
topbar-chip-settings = ⚙ CONFIG
topbar-chip-help = ? AJUDA

## Barra superior: o chip de padrões de modo.

topbar-changed-agc = AGC
topbar-changed-squelch = squelch
topbar-changed-auto-notch = notch automático
topbar-changed-stereo = estéreo
topbar-changed-binaural = binaural
# $mode é um nome de modo (USB, CW…); $changed lista o que difere dos padrões dele.
topbar-mode-defaults =
    Volta aos padrões de { $mode }. Alterados em relação a eles: { $changed }.

    Clique para devolvê-los. Os valores próprios de { $mode } voltam, e o que você definiu aqui é esquecido.

## Janela de Configurações, aba Rádio: mensagens mais longas com valores

settings-radio-no-such-baud = o rádio não tem configuração de { $baud } baud — { $default } será usado no lugar
settings-radio-icom-scope-hint =
    Transmite a varredura do scope próprio do rádio pelo link CI-V e a desenha como panadaptador — a única imagem da banda que um equipamento de áudio demodulado pode dar, já que o áudio que ele envia é o que já passou pelo filtro dele.

    Exige o rádio configurado para isso: CI-V USB Baud Rate em 115200 e CI-V USB Port em "Unlink from {"["}REMOTE]" (ambos em SET > Connectors > CI-V), e a taxa de baud acima ajustada para coincidir — abaixo de { $min_baud } as varreduras não cabem no link e o scope fica desligado.

    As varreduras dividem o barramento USB interno do rádio com a placa de som dele. Se o áudio recebido começar a falhar com o scope ligado, esta caixa é a primeira coisa a tentar desligar.
settings-radio-rx-tx-words = { $rx } / TX { $tx }
settings-radio-soapy-native = O hardware acima é suportado diretamente pelo sdroxide: { $names }. Escolher essa interface acima dá a você as próprias configurações dela — estágios de ganho, filtros e notches que o SoapySDR não consegue expressar — e não precisa de nenhum módulo SoapySDR.
settings-radio-rx888-width-hint = Quanto do espectro digitalizado o panadaptador mostra de uma vez. Toda a cadeia de DSP roda nesta largura, então uma largura maior custa proporcionalmente mais CPU — 1/2 é a banda inteira na cascata, e uma quantidade séria de aritmética. Acima da transição VHF ({ $crossover } MHz) o panadaptador não está olhando para a antena, mas para o F.I. do sintonizador, que tem 8 MHz de largura e fica em { $if_centre } MHz — então qualquer coisa mais larga que { $widest } não pode ser centralizada nele, e a largura extra é espectro morto ao lado do sinal, e não mais dele.
settings-radio-sdrplay-serial-missing = O serial { $serial } não está entre os receptores que o serviço SDRplay informa. Escolha um dos receptores listados, ou reconecte este e pressione Buscar novamente.
settings-radio-hdr-filter-hint =
    O filtro analógico na frente do caminho HDR — um controle diferente da própria largura de banda do receptor. Cada configuração é construída em um conjunto fixo de centros e não pode fazer nada em outros lugares; esta é construída em { $centres }. Vale imediatamente.

    O caminho em si ainda não funciona — veja a chave acima — então isto escolhe um filtro para um modo que está atualmente silencioso.
settings-radio-rspduo-only = Somente um RSPduo tem um segundo sintonizador, e o receptor selecionado aqui é um { $model }, então isto não fará nada.
settings-radio-lime-tx-gain-low = O ganho de transmissão está em { $db } dB, no fundo da sua faixa — alguns microwatts saindo da placa, que aparecerão como nada em um medidor de potência, seja o que for que venha depois dela. Este é o padrão para que um transmissor armado não o surpreenda; aumente antes de acionar.

## Janela de Configurações, Geral: ROE

settings-general-swr-tune-limit = (sintonia: { $limit }:1)

## Barra superior e faixa de controles: mensagens com valores

# $reading é o próprio texto de ajuda do medidor; $faces lista as faces pelas quais um clique passa.
topbar-smeter-hover =
    { $reading }

    Clique para alternar a face do medidor: { $faces }
topbar-step-down = Para baixo { $step }
topbar-step-up = Para cima { $step }
# $auto é topbar-band-plan ou vazio.
topbar-repeater-shift-hover = Deslocamento de repetidora { $shift } — recebendo em { $rx }, transmitindo em { $tx } MHz{ $auto }
topbar-tone-and-burst = Transmitindo { $tone } por baixo da voz, e uma rajada de 1750 Hz de { $ms } ms no início de cada transmissão
topbar-tone-under-voice = Transmitindo { $tone } por baixo da voz
topbar-burst-every-over = Uma rajada de 1750 Hz de { $ms } ms no início de cada transmissão
topbar-match-tx = CASAR TX ({ $tone })
topbar-decimation-on = Decimação no front end: o receptor recebe o centro de 1/{ $n } dos { $msps } Msps que este rádio transmite — { $span } — e o restante é descartado antes que qualquer parte chegue ao receptor.
topbar-decimation-off = Decimação no front end, desligada: o receptor vê os { $span } inteiros que este rádio transmite.
topbar-decimation-hint =
    { $what }

    Uma largura menor significa resolução mais fina na cascata, um piso de ruído mais baixo (3 dB a cada metade) e menos CPU — ao custo da faixa de cada lado dela. O dial ainda sintoniza em qualquer lugar; o rádio move o seu LO para acompanhar.

    Clique para alternar: desligado / 2 / 4 … { $max }.
topbar-rx-gain-hint = Ganho RX do front end ({ $stage }). Demais corta o ADC do receptor e espalha sinais espúrios pela faixa; de menos, e ele fica surdo.
topbar-rx-gain-stages =
    {""}

    Este equipamento tem { $n } estágios de ganho RX — os demais estão em Configurações → Dispositivo.
topbar-passband-now =
    {""}

    A banda de passagem está agora em { $dbfs } dBFS.
# $level é topbar-passband-now ou vazio.
topbar-squelch-hint =
    Silencia o áudio abaixo desta potência na banda de passagem de recepção, em dBFS. À esquerda está aberto.{ $level } Defina-o acima do ruído e abaixo do sinal.

    Um fluxo que chega com o AGC próprio do rádio já aplicado — o F.I. de 12 kHz de um Icom, por exemplo — fica muito mais alto nesta escala do que a banda base bruta de um SDR, e é por isso que a barra chega à escala total.
topbar-nr-cycle-hint =
    Redução de ruído: { $strength } — { $engine }

    Clique para alternar a intensidade: Baixa / Média / Alta / Desligada. O motor está nas linhas abaixo.
topbar-recording-both = Gravando { $audio } e { $iq }
topbar-recording-audio = Gravando áudio em { $audio }
topbar-recording-iq = Gravando I/Q em { $iq }
topbar-auto-record-armed = Gravação automática armada: um arquivo por transmissão, fechado após { $secs } s de silêncio
topbar-rds-hover = RDS: { $ps }. Clique para os dados da estação
topbar-drm-hover = DRM: { $service }. Clique para os detalhes da transmissão
topbar-drm-no-codec = DRM: { $summary } — travado, mas o seu áudio { $codec } não pode ser decodificado aqui. Clique para ver o que falta
topbar-drm-state = DRM: { $summary } — clique para o estado do decodificador
topbar-hd-hover = HD Radio: { $station }. Clique para os detalhes da transmissão
topbar-hd-state = HD Radio: { $summary } — clique para o estado do decodificador
topbar-binaural-preset = { $preset } em cada ouvido — { $lo } … { $hi } Hz, as duas bandas laterais
topbar-nr-hover =
    Redução de ruído: { $strength } — { $engine }

    Clique para trocar o motor ou a intensidade
topbar-clipping = Cortando { $span } — pressione de novo para recomeçar
topbar-record-clip = Gravar { $span } e parar — um trecho para anexar a um relatório de recepção
topbar-starting-clip = iniciando um trecho de { $span }…
topbar-stop-recording-after = Parar a gravação MP3 após { $minutes } minutos
topbar-stops-in = para em { $time }
topbar-auto-record-hint = Grava cada transmissão em seu próprio arquivo, fechado após { $secs } segundos de silêncio
topbar-recording-silence = gravando · { $secs } s de silêncio (fecha em { $hold })
topbar-tone-open = Recebendo { $tone }, que é o squelch de tom — áudio aberto
topbar-tone-closed = Recebendo { $heard }, mas o squelch de tom quer { $wanted } — o áudio continua fechado
topbar-tone-heard = Recebendo CTCSS/DCS { $tone }
topbar-tone-nothing = Squelch de tom { $tone }: nada correspondente a ele está sendo recebido
topbar-tone-use = USAR { $tone }
topbar-sockets = Conectores: { $sockets }
topbar-voice-keyer-tx = Transmitindo { $slot } — clique para abrir o manipulador de voz
topbar-drive-uncal-gen = Esta frequência, que não está em nenhuma faixa de amador, não é calibrada, então a configuração chega ao transmissor integralmente.
topbar-drive-uncal-band = { $band } não é calibrada, então a configuração chega ao transmissor integralmente.
# $trim tem sinal (+1,5); $diff é o seu tamanho; $more é "less" ou "more".
topbar-drive-cal-gen = Esta frequência, que não está em nenhuma faixa de amador, é calibrada em { $trim } dB, então a mesma configuração põe { $diff } dB { $more ->
        [less] a menos
       *[more] a mais
    } no ar aqui do que em uma faixa não calibrada.
topbar-drive-cal-band = { $band } é calibrada em { $trim } dB, então a mesma configuração põe { $diff } dB { $more ->
        [less] a menos
       *[more] a mais
    } no ar aqui do que em uma faixa não calibrada.
topbar-drive-hover =
    Com que força o transmissor é excitado. Em um rádio que o sdroxide modula por conta própria, ele escala as amostras moduladas; em um equipamento com controle de potência próprio, é a fração da potência nominal que o equipamento recebe.

    { $trim }

    Configurações → Rádio → Excitação de transmissão por faixa.
topbar-cessb-hover =
    SSB de envelope controlado: mais potência média para o mesmo pico, sem espalhamento — três ou quatro decibéis de sonoridade aparente na outra ponta. Quantos decibéis a voz é levada ao processador; 0 é desligado, 6 é uma primeira tentativa sensata. Não mexe na Excitação.

    Agora: { $now }
topbar-waterfall-rate = { $rows } linhas por segundo — { $seconds } segundos de banda
topbar-detail-auto-columns = { $n } colunas, seja qual for a que o Automático escolheria aqui
topbar-n-columns = { $n } colunas
topbar-fft-bins-whole = { $n } raias em todo o fluxo — uma por coluna, ou mais finas
topbar-fft-bins-pooled = { $n } raias agrupadas nas { $cols } colunas do panadaptador: sinais mais nítidos, não mais deles
topbar-sat-locked = Satélite — travado em { $name }
topbar-amp-in-line = { $model } — na linha (PTT)
topbar-amp-bypassed = { $model } — desviado (DESLIGADO)
topbar-amp-not-answering = { $model } — não responde
topbar-filter-edges = { $lo } … { $hi } Hz da portadora
topbar-filter-isb = Filtro de recepção: { $width } Hz em cada ouvido ({ $edges }). As duas bandas laterais do ISB são transmissões separadas, então a largura é a de uma delas.
topbar-filter-width = Filtro de recepção: { $width } Hz de largura ({ $edges }).
topbar-filter-hint =
    { $what }

    Clique para a largura, as duas bordas e as predefinições deste modo, em números — as alças do panadaptador posicionam uma banda de passagem a olho, e é aqui que um valor exato é digitado.
topbar-band-outside-range =
    { $band } está fora do que este rádio recebe ({ $ranges } MHz).

    { $why }
# $time é "day" ou "night".
topbar-band-forecast = { $band }: { $verdict } ({ $time ->
        [day] de dia
       *[night] à noite
    }) — previsão do HAMQSL.com a partir dos índices solares, não uma medição do seu próprio percurso.

## Barra superior: palavras curtas e legendas

topbar-squelch-open = aberto
topbar-off = desligado
topbar-recording = gravando
topbar-sql = sql
topbar-stations = estações
topbar-caption-reaction = reação
topbar-caption-flow = fluxo
topbar-caption-detail = detalhe
topbar-floor = piso

## Abas dos painéis de modos digitais (src/app/panels/mod.rs, pane_label).

pane-aircraft = AERONAVES
pane-chart = GRÁFICO
pane-chat = CHAT
pane-decodes = DECODIFICAÇÕES
pane-files = ARQUIVOS
pane-heard = OUVIDAS
pane-image = IMAGEM
pane-map = MAPA
pane-messages = MENSAGENS
pane-monitor = MONITOR
pane-panel = PAINEL
pane-qso = QSO
pane-reading = LEITURA
pane-receive = RECEBER
pane-saved = SALVAS
pane-send = ENVIAR
pane-spots = SPOTS
pane-stations = ESTAÇÕES
pane-status = ESTADO
pane-terminal = TERMINAL
pane-text = TEXTO
pane-traffic = TRÁFEGO
pane-vessels = EMBARCAÇÕES
pane-wfall = CASCATA

## Painéis de modos digitais, comuns (src/app/panels/mod.rs)

panels-save = {" "}SALVAR{" "}
panels-nothing-decoded-save = Nada decodificado para salvar
panels-clear-rx = {" "}LIMPAR RX{" "}
panels-empty-receive-window-nothing-air = Esvazia a janela de recepção. Nada que está no ar para.
panels-nothing-received-clear = Nada recebido para limpar
panels-save-what-panel-has-decoded = Salva o que este painel decodificou em um arquivo
panels-prop = PROP
panels-shade-map-where-signals-are = Sombreia o mapa conforme onde os sinais de fato estão chegando, em cada faixa. Cada recepção é posicionada no ponto médio do seu percurso — a porção da ionosfera que a curvou — e não na estação distante, então isto é um mapa do céu, e não de onde os radioamadores estão. Construído a partir dos percursos em que esta estação tomou parte, mais — quando a Reverse Beacon Network está ligada em Configurações → Spots — o que os skimmers do mundo estão ouvindo, o que cobre as faixas em que este rádio não está.
panels-night = NOITE
panels-shade-where-sun-down-twilight = Sombreia onde o Sol está abaixo do horizonte, e o crepúsculo entre, para que a linha cinza apareça no mapa. As faixas baixas se alongam e as altas se fecham no lado noturno dela, e o próprio terminador é onde está o DX.
panels-all-bands = TODAS AS FAIXAS
panels-every-band-once-one-hue = Todas as faixas ao mesmo tempo, um tom para cada — condições gerais.
panels-one-band = UMA FAIXA
panels-one-band-cold-hot-blue = Uma faixa, do frio ao quente: azul, verde, amarelo, vermelho.
panels-frequency-you-are-actually-working = A frequência em que você está de fato operando: o dial mais o deslocamento de tom ao lado. É isto que vai para o log — só o dial fica esse tanto abaixo.
panels-send-return = ENVIAR AO RETORNAR
panels-freq = ⇵ FREQ
panels-nothing-agreed-mode-save-your = Nada combinado para este modo — salve a sua abaixo.
panels-forget-saved-frequency = Esquecer esta frequência salva
panels-dial-already-one-your-saved = Este dial já é uma das suas frequências salvas.
panels-dial-already-agreed-list-mode = Este dial já está na lista combinada para este modo.
panels-decode-squelch-raise-stop-decoding = Squelch de decodificação — aumente para parar de decodificar ruído
panels-message-repeats-long-transmit-held = A mensagem se repete enquanto a transmissão estiver pressionada — um meteoro captura a parte dela que estiver passando.
panels-w1abc-w9xyz-fn42 = W1ABC W9XYZ FN42
panels-tx = {"  "}TX LIGADO{"  "}
panels-tx-2 = {"   "}TX{"   "}
panels-call-cq = {" "}CHAMAR CQ{" "}
panels-nocall = SEMCHAMADA

## Janela de Áudio TX (src/app/tx_audio.rs)

txaudio-heard-too-little-speech-talk = Ouviu fala de menos — fale normalmente durante toda a contagem e tente de novo.
txaudio-tx-audio = Áudio TX
txaudio-mode-not-voice-mode-so = Este modo não é um modo de voz, então nada que ele envia passa pela cadeia. As configurações feitas aqui valem em SSB, AM e FM.
txaudio-bypass = DESVIAR
txaudio-switch-every-stage-out-once = Desliga todos os estágios de uma vez, inclusive o redutor de ruído, para que o microfone saia sem processamento. Clique de novo para restaurar exatamente os estágios que estavam ligados — a comparação A/B mais rápida do que a cadeia está fazendo.
txaudio-denoise = REDUZIR RUÍDO
txaudio-rnnoise-microphone-ahead-every-other = RNNoise no microfone, antes de todos os outros estágios, para que o ruído do ambiente seja removido antes que o gate ou o compressor possa elevá-lo. Acrescenta cerca de 10 ms de atraso.
txaudio-digital-voice = VOZ DIGITAL
txaudio-also-run-freedv-rade-digital = Também passa a voz digital FreeDV / RADE pela cadeia. Desligado por padrão: um vocoder é treinado com fala natural e pode decodificar pior uma voz muito processada, então teste no ar antes de confiar nisso. O áudio TCI e os modos digitais nunca passam pela cadeia.
txaudio-test = Testar
txaudio-rec = {" "}⏺ GRAVAR{" "}
txaudio-play = {" "}▶ TOCAR{" "}
txaudio-play-test-phrase-through-chain = Toca a frase de teste pela cadeia como está ajustada agora, nos seus alto-falantes. Ajuste enquanto ela toca e ouça a mudança. O áudio de recepção é substituído enquanto ela toca.
txaudio-recording = gravando…
txaudio-no-phrase-yet = nenhuma frase ainda
txaudio-live = Ao vivo
txaudio-live-2 = {" "}🎧 AO VIVO{" "}
txaudio-hear-your-microphone-through-chain = Ouça o seu microfone pela cadeia enquanto fala, sem acionar o transmissor. Use fones: pelos alto-falantes ele realimenta. Você se ouve com alguns décimos de segundo de atraso, o que torna a fala desconfortável — ajuste os níveis nos medidores e julgue o som com GRAVAR / TOCAR. O áudio de recepção é substituído enquanto isso roda; pressionar PTT para.
txaudio-meters = {" "}MEDIDORES{" "}
txaudio-run-your-microphone-through-chain = Passa o seu microfone pela cadeia só para os medidores: nada nos alto-falantes, então não há atraso para falar em cima nem nada para realimentar. É a forma de ajustar o ganho do microfone e de entrada, o limiar do gate e o quanto o compressor trabalha. O áudio de recepção continua; pressionar PTT para.
txaudio-monitoring-live-use-headphones-speakers = Monitorando ao vivo — use fones, ou os alto-falantes vão realimentar o microfone.
txaudio-voice-modes-only-tci-clients = Somente modos de voz. Clientes TCI e os modos digitais contornam a cadeia; quando ela está em uso, mantenha o EQ e o processador de TX do próprio rádio planos.
txaudio-auto-set-stopped-microphone-was = O ajuste automático parou: o microfone foi tomado.
txaudio-microphone-may-clipping-before-gets = {" "}O microfone pode estar cortando antes de chegar aqui — reduza o ganho dele se soar áspero.
txaudio-level = Nível
txaudio-mic-control-same-one-transmit = O controle Mic — o mesmo da barra de transmissão, compartilhado por todos os perfis. 0 dB no meio, +6 dB no topo.
txaudio-mic = Mic
txaudio-gain-top-mic-microphone-stored = Ganho sobre o Mic para este microfone, guardado com o perfil. Os limiares das predefinições supõem uma voz com pico em torno de -10 dBFS no medidor IN; o Ajuste automático o coloca ali. Permanece dentro dele com DESVIAR, e vale também para a voz digital: é nível, não processamento.
txaudio-input = Entrada
txaudio-auto-set = Ajuste automático
txaudio-auto-set-cancelled = Ajuste automático cancelado.
txaudio-talk-now-you-would-air = Fale agora, como faria no ar…
txaudio-db = -∞ dB
txaudio-chain = Cadeia
txaudio-order-audio-meets-them = Na ordem em que o áudio os encontra
txaudio-switch-stage = Ligar este estágio
txaudio-earlier-chain = Antes na cadeia
txaudio-later-chain = Depois na cadeia
txaudio-output-limiter = O limitador de saída
txaudio-output = Saída
txaudio-trim-limiter-always-last = Ajuste e limitador — sempre por último
txaudio-profiles = Perfis…
txaudio-gate-expander = Gate / expansor
txaudio-pushes-room-noise-fan-rumble = Empurra para baixo o ruído do ambiente, o ronco do ventilador e o chiado nas pausas. Defina o limiar logo acima do seu piso de ruído e abaixo das sílabas mais suaves.
txaudio-gate = GATE
txaudio-hard-gate-10-1-down = Gate duro: 10:1, até -40 dB
txaudio-expander = EXPANSOR
txaudio-gentle-expander-2-1-never = Expansor suave: 2:1, nunca mais que -15 dB — o ambiente nunca fica morto
txaudio-reset = REDEFINIR
txaudio-back-defaults = Volta aos padrões
txaudio-threshold = Limiar
txaudio-below-gate-starts-close = Abaixo disto o gate começa a fechar.
txaudio-return = Retorno
txaudio-hysteresis-once-open-closes-only = Histerese: uma vez aberto, ele só fecha este tanto abaixo do limiar, para que uma voz que oscile ali não faça o gate tremer.
txaudio-hold = Retenção
txaudio-how-long-stays-open-after = Por quanto tempo fica aberto depois que a voz cai, para que o fim das palavras não seja cortado.
txaudio-release = Liberação
txaudio-how-fast-closes-once-hold = Com que rapidez fecha quando a retenção termina. Aumente se o gate ficar bombeando.
txaudio-floor = Piso
txaudio-most-ever-turns-pauses-down = O máximo que ele reduz as pausas.
txaudio-ratio = Razão
txaudio-how-steeply-closes-below-threshold = O quão abruptamente fecha abaixo do limiar.
txaudio-look-ahead = Antecipação
txaudio-opens-much-before-word-starts = Abre este tanto antes de a palavra começar, para que a primeira consoante não seja cortada. Acrescenta o mesmo atraso a tudo o que é transmitido.
txaudio-high-pass-low-shelf-two = Passa-altas, shelf de graves, dois picos, shelf de agudos e passa-baixas. Arraste um ponto na curva, ou defina-o abaixo. Ajuste o passa-baixas para a largura do seu filtro de transmissão.
txaudio-flat = PLANO
txaudio-every-band-back-0-db = Todas as faixas de volta a 0 dB, as duas bordas desligadas
txaudio-freq = Freq
txaudio-gain = Ganho
txaudio-q-slope = Q / Inclinação
txaudio-high-pass = Passa-altas
txaudio-removes-handling-noise-breath-pops = Remove ruído de manuseio, estouros de respiração e zumbido abaixo do corte.
txaudio-low-shelf = Shelf de graves
txaudio-body-mud-cut-little-here = Corpo, ou lama: corte um pouco aqui para uma voz que soe oca.
txaudio-peak-1 = Pico 1
txaudio-bump-dip-voice-band-often = Um realce ou corte na banda da voz — frequentemente um corte em torno de 300–500 Hz.
txaudio-peak-2 = Pico 2
txaudio-presence-lift-around-2-3 = Presença: um realce em torno de 2–3 kHz é o que se destaca na SSB.
txaudio-high-shelf = Shelf de agudos
txaudio-brightness = Brilho.
txaudio-low-pass = Passa-baixas
txaudio-stops-energy-above-transmit-filter = Impede que energia acima do filtro de transmissão seja gasta no modulador.
txaudio-t-24-db-oct = 24 dB/oitava
txaudio-t-12-db-oct = 12 dB/oitava
txaudio-click-steeper-gentler-slope = Clique para uma inclinação mais íngreme ou mais suave
txaudio-de-esser = De-esser
txaudio-turns-down-only-sibilant-band = Reduz somente a banda sibilante quando "s" e "sh" ultrapassam o limiar, para que o resto da voz mantenha o nível. Encontre a frequência varrendo-a enquanto diz "sister silver cellophane" e pare onde o chiado for mais forte.
txaudio-frequency = Frequência
txaudio-centre-sibilant-band = Centro da banda sibilante.
txaudio-width-band-higher-narrower = Largura da banda — quanto mais alto, mais estreita.
txaudio-band-level-where-reduction-starts = Nível da banda em que a redução começa. Baixo demais e ela atua o tempo todo, abafando a voz.
txaudio-amount = Quantidade
txaudio-most-band-ever-turned-down = O máximo que a banda é reduzida. Comece em torno de -6 dB.
txaudio-attack = Ataque
txaudio-how-fast-reacts = Com que rapidez reage.
txaudio-how-fast-lets-go = Com que rapidez solta.
txaudio-compressor = Compressor
txaudio-brings-quiet-syllables-up-holds = Traz as sílabas baixas para cima e segura as altas. Mire em 3–6 dB de redução nos picos; mais de 10 dB é compressão evidente, mais adequada a concursos.
txaudio-level-above-which-starts-compress = Nível acima do qual começa a comprimir.
txaudio-how-hard-compresses-above-threshold = O quanto comprime acima do limiar.
txaudio-how-fast-reacts-peak-10 = Com que rapidez reage a um pico. 10–25 ms mantém as consoantes nítidas.
txaudio-how-fast-recovers = Com que rapidez se recupera.
txaudio-knee = Joelho
txaudio-soft-knee-width-0-hard = Largura do joelho suave; 0 é um joelho duro.
txaudio-makeup = Compensação
txaudio-gain-after-compression-bring-level = Ganho depois da compressão, para trazer o nível de volta.
txaudio-drive = Excitação
txaudio-gain-ahead-compressor-so-more = Ganho antes do compressor, para que mais da voz cruze o limiar — eleva a potência média. Use junto com o limitador de saída.
txaudio-phase = Fase
txaudio-all-pass-phase-rotator-ahead = Rotador de fase passa-tudo antes do detector. Equilibra os picos assimétricos que uma voz tem, para que o mesmo nível de pico carregue mais potência média. 4 é uma configuração comum de radiodifusão; 0 é desligado.
txaudio-last-stage-trim-look-ahead = O último estágio: um ajuste, e um limitador com antecipação que mantém todo pico abaixo do teto sem o espalhamento que o corte brusco causa. Desligado por padrão, então ligar a cadeia não altera um nível de excitação que você já tenha definido.
txaudio-limiter = Limitador
txaudio-on = Ligado
txaudio-ceiling = Teto
txaudio-no-peak-leaves-chain-above = Nenhum pico sai da cadeia acima disto.
txaudio-trim = Ajuste
txaudio-output-gain-ahead-limiter = Ganho de saída, antes do limitador.
txaudio-off = {"  "}desligado
txaudio-meters-move-while-you-transmit = Os medidores se movem enquanto você transmite uma voz, toca a frase de teste, ou roda AO VIVO ou MEDIDORES.
txaudio-profiles-2 = Perfis
txaudio-profile-whole-chain-every-stage = Um perfil é a cadeia inteira: as configurações de cada estágio, quais estão ligados, a ordem deles e o ganho de entrada do seu microfone. Carregar um substitui a cadeia; o controle Mic e a excitação permanecem como estão.
txaudio-name = Nome
txaudio-save = Salvar
txaudio-store-chain-now-under-name = Guarda a cadeia como está agora com este nome, substituindo um perfil de mesmo nome
txaudio-import = Importar…
txaudio-read-profile-json-file = Lê um perfil de um arquivo .json
txaudio-tx-audio-profile = Perfil de áudio TX
txaudio-no-profiles-yet-save-stores = Nenhum perfil ainda. Salvar guarda a cadeia que você configurou agora.
txaudio-load = Carregar
txaudio-export = Exportar…
txaudio-delete = Excluir
txaudio-starting-points = Pontos de partida
txaudio-set-your-level-auto-set = Defina o seu nível primeiro com o Ajuste automático — os limiares das predefinições o supõem — e depois ajuste daqui, em vez de transmitir como carregado: grave uma frase e ouça após cada alteração. Uma predefinição mantém o seu ganho de entrada.
txaudio-profile-loaded = Perfil carregado.

## Sobreposição da visão 3D do sistema solar (src/solar3d/overlay.rs)

solar-prop = PROP
solar-where-signals-are-actually-getting =
    Onde os sinais estão de fato chegando, faixa por faixa. Cada recepção em que esta estação toma parte — WSPR, FT8/FT4, JS8 e o diário de contatos — é posicionada no ponto médio do seu percurso, que é a porção da ionosfera que o curvou, e não na estação distante. Assim, isto é um mapa do céu, e não de onde os radioamadores estão.

    Só pode mostrar percursos em que esta estação foi uma das pontas. Os oceanos se iluminam porque os pontos médios dos percursos longos caem ali; a Antártida fica escura porque ninguém transmite de lá. Desligado por padrão — pinta o planeta inteiro.
solar-orbits = ÓRBITAS
solar-orbital-paths = Trajetórias orbitais
solar-clouds = NUVENS
solar-cloud-cover-noaa-s-global = Cobertura de nuvens do mosaico geoestacionário global da NOAA, desenhada como profundidade de ar em vez de uma imagem colada na superfície: o infravermelho diz quão fria é cada topo de nuvem, e é assim que se sabe a altura dela. Tempestades relampejam dentro das torres que constroem.
solar-planets = PLANETAS
solar-other-seven-planets-eighteen-their = Os outros sete planetas, dezoito das suas luas, e os anéis de Saturno e Urano. As posições planetárias vêm do conjunto de elementos kepleriano do JPL; as luas são órbitas circulares ajustadas ao JPL Horizons.
solar-coronal-mass-ejection-trajectory-cones = Cones de trajetória de ejeção de massa coronal
solar-sun-obs = OBS SOL
solar-solar-observations-sun-s-disk = Observações solares no disco do Sol: regiões ativas de manchas solares, e de onde vieram os flares
solar-labels = RÓTULOS
solar-body-region-labels = Rótulos de corpos e regiões
solar-small-bodies = CORPOS PEQUENOS
solar-names-asteroids-comets-they-are = Nomes nos asteroides e cometas. Eles são desenhados de qualquer forma — isto é apenas se as trinta e cinco designações são desenhadas junto. Qualquer que seja a configuração, o corpo em que a câmera está e o que a caixa de busca encontrou continuam nomeados.
solar-decoded-ft8-ft4-stations-path = Estações FT8/FT4 decodificadas e o percurso até a estação em QSO
solar-sats = SATS
solar-amateur-radio-satellites-their-orbits = Satélites de radioamador, suas órbitas e elevação a partir do seu QTH
solar-aurora = AURORA
solar-auroral-oval-noaa-s-ovation = O oval auroral do modelo OVATION da NOAA, desenhado como cascas de emissão em suas altitudes reais, com a borda voltada ao equador do contorno de 10 % marcada na superfície
solar-awards = PRÊMIOS
solar-award-coverage-every-dxcc-entity = Cobertura de prêmios: cada entidade DXCC pintada conforme o que o seu diário ainda não tem. Laranja queima onde você nunca trabalhou a entidade, âmbar onde trabalhou mas não está confirmada, verde-escuro onde um QSL já voltou. Acompanha o filtro de faixa da janela PRÊMIOS.
solar-view = VISÃO
solar-which-body-camera-orbits-animated = Em torno de qual corpo a câmera orbita, e o passeio animado
solar-layers = CAMADAS
solar-what-drawn-orbits-clouds-cmes = O que é desenhado: órbitas, nuvens, CMEs, rótulos, QSOs, aurora
solar-which-sdo-channel-wraps-sun = Qual canal do SDO envolve o Sol, e quão antiga é a imagem
solar-weather = TEMPO
solar-aurora-propagation-numbers-which-bands = A aurora, os números de propagação e quais faixas estão abertas
solar-space-weather = CLIMA ESPACIAL
solar-nothing-measured-yet = nada medido ainda
solar-scale = ESCALA
solar-body-moon-orbit-exaggeration = Exagero do corpo e da órbita da Lua
solar-time = TEMPO
solar-scrub-whole-scene-forward-back = Percorre a cena inteira para frente e para trás
solar-activity = ATIVIDADE
solar-replay-last-hour-decodes-globe = Reproduz a última hora de decodificações no globo
solar-what-propagation-heat-map-showing = O que o mapa de calor de propagação está mostrando
solar-propagation = PROPAGAÇÃO
solar-auto = ▶ AUTO
solar-fly-spline-through-set-framed = Percorre uma spline por um conjunto de pontos de vista enquadrados. Qualquer entrada do mouse cancela.
solar-moons-ride-circular-orbits-fitted = As luas seguem órbitas circulares ajustadas ao JPL Horizons: a um ou dois graus de onde realmente estão, e até seis para Miranda, cujo plano orbital oscila rápido demais para um círculo acompanhar.
solar-fetch-everything-again-now = Buscar tudo de novo agora
solar-all-sats = TODOS OS SATS
solar-show-every-satellite-element-set = Mostra todo satélite do conjunto de elementos, e não só os populares. Os anéis de órbita ficam nos poucos curados — noventa de uma vez é ilegível.
solar-starting = iniciando…
solar-body = corpo{" "}
solar-moon-orbit = órbita da lua{" "}
solar-stretch-earth-moon-distance-so = Estica a distância Terra→Lua para que o par possa ser visto separado
solar-sun = sol{" "}
solar-sun-radius-exaggeration-leave-1 = Exagero do raio do Sol. Deixe em 1× para manter a geometria dos CMEs legível — um Sol inchado engole a base de cada cone.
solar-show = MOSTRAR
solar-paint-globe-what-getting-through = Pinta o globo pelo que está chegando
solar-paints-globe-what-getting-through = Pinta o globo conforme o que está chegando, em cada faixa. Cada recepção é posicionada no ponto médio do seu percurso — onde a ionosfera fez o trabalho — e não na estação distante. Construído a partir do que esta estação ouve, mais os skimmers do mundo quando a Reverse Beacon Network está ligada.
solar-display = EXIBIÇÃO
solar-all-bands = TODAS AS FAIXAS
solar-every-band-once-one-hue = Todas as faixas ao mesmo tempo, um tom para cada, misturando onde duas se sobrepõem. A visão para “como estão as condições” em vez de “os 20 m estão abertos”.
solar-one-band = UMA FAIXA
solar-one-band-cold-hot-blue = Uma faixa, do frio ao quente: azul, verde, amarelo, vermelho.
solar-band = FAIXA
solar-nothing-heard-yet = nada ouvido ainda
solar-bands = FAIXAS
solar-sources = FONTES
solar-contacts-logbook-path-was-open = Contatos no diário. Um percurso que estava aberto, sem relatório de sinal que valha o nome — um RST não é um SNR, então estes contam para o quão movimentada está uma célula e nunca para a sua margem.
solar-stations-reported-hearing-one-downloaded = Estações que informaram ter ouvido esta, baixadas do WSPRnet. O único retorno que um balizador transmissor recebe.
solar-reverse-beacon-network-skimmers-what = Skimmers da Reverse Beacon Network: o que todos os outros estão ouvindo, em cada faixa, inclusive as em que este rádio não está. Mais grosseiros que o resto — uma linha do RBN não traz locators, então as duas pontas são posicionadas no centro do país, o que é próximo para um país pequeno e muito errado para um grande. Ligue em Configurações → Spots.
solar-decodes-station-made-each-measured = Decodificações feitas por esta estação. Cada uma é medida contra o piso do próprio modo, para que um relatório WSPR e um FT4 sejam comparáveis.
solar-memory = MEMÓRIA
solar-half-life = meia-vida{" "}
solar-how-long-reception-takes-count = Quanto tempo uma recepção leva para contar metade do valor. A memória da própria ionosfera é curta — uma abertura de duas horas não deveria discutir com uma de dois minutos atrás.
solar-live = AO VIVO
solar-follow-band-happens = Acompanha a faixa à medida que acontece
solar-replay = ⏸ REPRODUZIR
solar-replay-2 = ▶ REPRODUZIR
solar-replay-last-hour-decodes-over = Reproduz a última hora de decodificações, repetidamente
solar-min-ago = {" "}min atrás
solar-where-last-hour-globe-showing = Onde, na última hora, o globo está mostrando
solar-trail = rastro{" "}
solar-how-long-decode-s-arc = Por quanto tempo o arco de uma decodificação fica no globo atrás da cabeça
solar-speed = velocidade{" "}
solar-how-much-faster-than-real = Quanto mais rápido que o tempo real a reprodução roda
solar-qso-layer-off = Camada de QSO desligada
solar-no-decodes-yet = nenhuma decodificação ainda
solar-qth-not-set-enter-your = QTH não definido — informe seu quadrado locator em Configurações
solar-mode = MODO
solar-path = PERCURSO
solar-sent = ENVIADO
solar-rcvd = RECEBIDO
solar-grid = LOCATOR
solar-dxcc = DXCC
solar-geostationary-always-above-your-horizon = Geoestacionário — sempre acima do seu horizonte.
solar-no-passes-next-48-hours = Nenhuma passagem nas próximas 48 horas a partir do seu QTH.
solar-start = INÍCIO
solar-max-el = EL MÁX
solar-aos-los-are-azimuths-horizon = AOS/LOS são azimutes no horizonte. Os horários são UTC.
solar-locked-unlock = {" "}● TRAVADO — DESTRAVAR{" "}
solar-release-satellite-lock = Libera o travamento no satélite
solar-lock = {" "}TRAVAR{" "}
solar-track-satellite-main-window-doppler = Rastreia este satélite na janela principal: RX e TX com correção Doppler, uplink mapeado no transponder, comando do rotor
solar-no-frequencies-file-one-add = Não há frequências cadastradas para este — adicione-as em Configurações ▸ TLE.
solar-frequencies-yours = FREQUÊNCIAS · SUAS
solar-frequencies = FREQUÊNCIAS
solar-link = ENLACE
solar-downlink-mhz = DESCIDA (MHz)
solar-uplink-mhz = SUBIDA (MHz)
solar-doppler-shifts-these-few-khz = O Doppler os desloca em alguns kHz durante uma passagem em órbita baixa.
solar-sim = -- SIM
solar-satellite-planet-comet = satélite, planeta ou cometa
solar-planet-asteroid-comet = planeta, asteroide ou cometa
solar-clear-search = Limpar a busca
solar-no-match = nenhum resultado
solar-no-sounder = sem sondador
solar-heard = OUVIDO ≥
solar-kp = Kp / A
solar-x-ray = Raios X
solar-observed-above-sounder-better-than = observado pelo sondador — melhor que o modelado
solar-power-n-s = potência N/S
solar-edge-n-s = borda N/S
solar-kp-peak-24-h = pico de Kp em 24 h
solar-observed-forecast = observado | previsto
solar-bands-open = FAIXAS ABERTAS
solar-dxcc-coverage = COBERTURA DXCC
solar-ir-vis = IV+VIS
solar-ir-only = só IV
solar-no-deep-convection = sem convecção profunda
solar-t-1-storm = 1 tempestade
solar-glancing = {" "}· tangencial
solar-direction-estimated = {" "}· direção estimada

## Caixas de configuração dos modos digitais (src/app/panels/setup.rs)

setup-aprs-setup = Configuração APRS
setup-packet-setup = Configuração de pacote
setup-ft8-ft4-ft2-setup = Configuração FT8 / FT4 / FT2
setup-my-callsign = Meu indicativo
setup-my-grid = Meu locator
setup-station-call = Indicativo da estação
setup-ssid-oe3jjs-10-packet-station = Com um SSID — OE3JJS-10. Uma estação de pacote usa, por convenção, um SSID diferente do operador, e nada transmite até que isto esteja definido.
setup-speed = Velocidade
setup-t-9600-needs-radio-s-data = 9600 exige a porta de dados do rádio. Um caminho de microfone e alto-falante a destrói nas duas pontas — isso é do rádio, não do sdroxide.
setup-tx-delay = Atraso de TX
setup-flags-sent-before-frame-so = Flags enviadas antes de um quadro, para que a outra ponta possa nos ouvir e travar o clock dela. Em um equipamento CAT, só o sdroxide gasta de 165 a 240 ms para entrar no ar, e o tempo de prontidão de transmissão do próprio equipamento soma-se a isso.
setup-packet-length = Tamanho do pacote
setup-bytes = {" "}bytes
setup-most-single-frame-carries-shorter = O máximo que um único quadro carrega. Quadros menores sobrevivem a um caminho marginal — só o quadro atingido é reenviado — e custam mais sobrecarga. 128 em HF, 256 onde o caminho é sólido.
setup-frames-sent-before-waiting-acknowledgement = Quadros enviados antes de esperar uma confirmação. Mais enche um bom caminho; em um ruim, é mais para reenviar.
setup-extended = Estendido
setup-ask-mod-128 = Pedir mod-128
setup-extended-sequence-numbers-window-bigger = Números de sequência estendidos, para uma janela maior que sete. Muitos nós recusam o pedido com um DM, que se parece exatamente com uma estação que não quer falar com você — então deixe desligado, a menos que saiba que a outra ponta o quer.
setup-answer-calls = Responder chamadas
setup-accept-connections = Aceitar conexões
setup-off-winlink-client-which-dials = Desligado para um cliente Winlink, que disca para fora e não tem motivo para responder. Ligado para ser alcançável como par: uma chamada chega no painel de terminal e você conversa com quem a fez. Uma estação cujo link já está ocupado com uma sessão Winlink recusa chamadas até que ela termine.
setup-connect-text = Texto de conexão
setup-what-caller-greeted = com o que quem chama é recebido
setup-sent-station-connects-us-once = Enviado a uma estação que se conecta a nós, quando o link está ativo. Vazio não envia nada — o que parece defeito para quem chamou.
setup-default-via = Via padrão
setup-oe3xlr-1-oe3xms-1 = OE3XLR-1,OE3XMS-1
setup-digipeater-path-terminal-starts-path = O caminho de digipeaters com que o terminal começa. O caminho até o seu nó local é sempre o mesmo, e redigitá-lo é como um salto acaba esquecido.
setup-beacon = Baliza
setup-t-0-disables-timer = 0 desativa o temporizador
setup-beacon-text = texto da baliza
setup-kiss-server = Servidor KISS
setup-serve = Servir
setup-offer-modem-kiss-tnc-socket = Oferece este modem como um TNC KISS em um socket, para que o Pat, um cliente APRS ou a pilha AX.25 do Linux possam usar o rádio.
setup-aprs-call = Indicativo APRS
setup-callsign-station-beacons-under-ssid = O indicativo sob o qual esta estação transmite a baliza, com o seu SSID — OE3JJS-9 para um carro, -10 para um I-gate, -5 para um celular. Deixe vazio para usar o indicativo acima.
setup-no-callsign-station-will-not = Sem indicativo: esta estação não transmitirá nada — nem baliza, nem mensagem, nem confirmação.
setup-symbol = Símbolo
setup-two-characters-protocol-carries-them = Os dois caracteres como o protocolo os carrega: uma tabela (`/` ou `\`) e um código de símbolo. Um dígito ou uma letra na primeira posição é uma sobreposição, desenhada por cima do ícone da tabela alternativa.
setup-path = Caminho
setup-wide1-1-wide2-1 = WIDE1-1,WIDE2-1
setup-how-far-you-ask-repeated = Até onde você pede para ser repetido. Esta é a configuração mais consequente do canal: cada salto multiplica as transmissões que a rede inteira faz em uma frequência compartilhada.
setup-position = Posição
setup-my-grid-2 = A partir do meu locator
setup-beacon-centre-locator-above-reported = Transmite a baliza do centro do locator acima, informado com a ambiguidade que um locator de fato tem — um de seis caracteres tem alguns quilômetros de largura, e dizer isso é honesto. Desmarque para informar coordenadas exatas.
setup-latitude = Latitude
setup-longitude = Longitude
setup-locator = a partir do locator
setup-not-coordinate = não é uma coordenada
setup-comment = Comentário
setup-sent-every-beacon = enviado em toda baliza
setup-t-0-default-never-beacons-timer = 0 — o padrão — nunca transmite a baliza por temporizador. Trinta minutos é a convenção para uma estação fixa; uma estação em movimento transmite com mais frequência, mas cada baliza é tempo de canal de outra pessoa.
setup-compressed = Comprimido
setup-compressed-position-format-third-air = O formato de posição comprimido: um terço do tempo no ar e mais preciso. Todo receptor desde os anos 1990 o lê.
setup-first-goes-out-one-interval = A primeira sai um intervalo a partir de agora, não imediatamente.
setup-messages = Mensagens
setup-acknowledge = Confirmar
setup-answer-messages-addressed-you-acknowledgement = Responde às mensagens endereçadas a você. Uma confirmação é uma transmissão que esta estação faz sem ser solicitada, então uma configuração só de recepção deve desligá-la — e a baliza então deixa de afirmar que também é alcançável.
setup-flags-sent-ahead-every-frame = Flags enviadas antes de cada quadro, para que o modem da outra ponta ouça a portadora e trave o clock antes de os dados começarem. Precisa durar mais que tudo entre apertar transmitir e irradiar: só o sdroxide gasta nisso de 165 a 240 ms em um equipamento CAT, e um rádio que recebe o áudio pela rede armazena mais por cima. Pouco demais e a outra ponta nunca trava — a transmissão está lá e nada a decodifica.
setup-shared-packet-mode = compartilhado com o modo pacote
setup-keep-stations = Manter estações
setup-how-long-station-stays-map = Por quanto tempo uma estação permanece no mapa depois de ter sido ouvida pela última vez. Também é contra isto que o desvanecimento do mapa é medido, então uma janela curta mostra só o que está ao vivo.
setup-auto-reply = Resposta automática
setup-answer-snr-grid-status = Responder SNR? / LOCATOR? / STATUS?
setup-answer-direct-question-addressed-you = Responde a uma pergunta direta endereçada a você ou a @ALLCALL, com a resposta em vez de uma confirmação. Nunca responde ao tráfego de outra estação, e nunca responde a si mesma.
setup-heartbeat = Heartbeat
setup-off = Desligado
setup-turbo-does-not-beacon-local = O Turbo não transmite baliza — é a velocidade local e de VHF.
setup-sends-your-callsign-grid-so = Envia o seu indicativo e locator para que outros saibam que você pode ser recebido. A primeira sai um intervalo a partir de agora, não imediatamente.
setup-beacon-frequency = Frequência da baliza
setup-t-500-1000-hz = 500–1000 Hz
setup-move-each-beacon-free-slot = Move cada baliza para um slot livre na sub-faixa de heartbeat, como o JS8Call faz: é onde as estações que monitoram balizas procuram, e mantém um transmissor sem supervisão fora do QSO de outra pessoa. O slot é escolhido quando a baliza realmente sai, livre de tudo o que está sendo decodificado.
setup-working-freq = Frequência de trabalho
setup-beacon-where-you-are-working = Transmite a baliza onde você está trabalhando. Contra a convenção da faixa, mas mantém tudo o que você transmite em um só lugar.
setup-heartbeat-reply = Resposta ao heartbeat
setup-answer-heartbeats-signal-report = Responder aos heartbeats com um relatório de sinal
setup-tell-station-beaconed-how-well = Diz a uma estação que transmitiu a baliza o quão bem você a copiou. Desligado por padrão: uma faixa movimentada traz um heartbeat a cada slot, e responder a todos inundaria justamente a faixa que os heartbeats existem para manter silenciosa. Limitado a uma resposta por estação a cada 15 minutos, e nunca enquanto uma mensagem ainda está chegando ou enquanto você tem algo na fila para enviar.
setup-status-message = Mensagem de estado
setup-tx-period = Período de TX
setup-even = Par
setup-odd = Ímpar
setup-auto-sequence = Sequência automática
setup-decode-depth = Profundidade de decodificação
setup-fast = Rápido
setup-normal = Normal
setup-deep = Profundo
setup-how-hard-ft8-decoder-works =
    Quão a fundo o decodificador FT8 trabalha para sinais fracos.

    Rápido — uma passagem, sem subtração de sinal: o mais veloz, com menos decodificações.
    Normal — subtração multipassagem uniforme: um pouco mais rápido que Profundo, um pouco menos minucioso.
    Profundo — a multipassagem com pontos de controle: a maior quantidade de decodificações, e ~1,2 s em um slot movimentado.
setup-auto-tx-frequency = Frequência de TX automática
setup-choose-transmit-frequency-automatically-quietest = Escolhe a frequência de transmissão automaticamente: o ponto mais silencioso no período em que você transmite, em vez da frequência da estação que você está respondendo. Desligado NÃO a mantém — ele responde na frequência da estação chamada. Para mantê-la, use Manter frequência de TX.
setup-hold-tx-frequency = Manter frequência de TX
setup-pin-transmit-tone-where-nothing = Fixa o tom de transmissão onde está. Nada o move: não responder a uma estação, nem a fila de chamadas, nem chamar CQ, nem um clique em uma decodificação ou na cascata. Tem precedência sobre a Frequência de TX automática. Trocar de faixa é a única exceção: o deslocamento que você definiu por último na nova faixa volta com ela. Para quando a sua licença é mais estreita que o plano de faixas — em um dial britânico de 60 m, em 5357 kHz, a alocação termina em 5358,0, então o tom precisa ficar abaixo de 1000 Hz.
setup-tx-watchdog = Vigia de TX
setup-stop-transmitting-after-long-no = Para de transmitir depois deste tempo sem resposta e sem ação sua. 0 desativa.
setup-give-up-after = Desistir após
setup-calls = {" "}chamadas
setup-unanswered-calls-one-station-before = Chamadas sem resposta a uma estação antes de seguir adiante. Chamar CQ está isento. 0 desativa.
setup-contest = Concurso
setup-everyday-exchange-grid-report-rr73 = A troca do dia a dia: locator, relatório, RR73.
setup-european-vhf-contests-send-signal = Concursos europeus de VHF: envia um relatório de sinal, um número de série e o seu locator de SEIS caracteres, no layout de mensagem que o WSJT-X chama de EU VHF Contest. O CQ sai como CQ TEST.
setup-serial = Série
setup-serial-number-next-exchange-will = O número de série que a próxima troca levará. Ele avança sozinho à medida que cada contato é registrado e é lembrado entre sessões — defina-o à mão só para começar um concurso, ou para retomar de onde um diário de papel parou.
setup-needs-6-character-grid = ⚠ exige um locator de 6 caracteres
setup-eu-vhf-contest-messages-carry = As mensagens do EU VHF Contest levam um locator de seis caracteres (JN88DD, não JN88). Até que o Meu locator tenha um, a troca comum é enviada no lugar.
setup-dxpedition = DXpedition
setup-ordinary-ft8-operation = Operação FT8 comum.
setup-calling-dxpedition-running-fox-mode = Chamando uma DXpedition em modo Fox: chame de acima de 1000 Hz, desça para o Fox quando ele responder, e registre-o no RR73 dele sem enviar 73.
setup-run-pile-up-several-signals = Trabalha o pile-up: vários sinais ao mesmo tempo, uma fila de chamadores, atendidos primeiro os mais fortes e mais raros. CHAMAR CQ o inicia, PARAR QSO o encerra.
setup-fox-signals = Sinais Fox
setup-once = {" "}de uma vez
setup-simultaneous-transmissions-spaced-60-hz = Transmissões simultâneas, espaçadas 60 Hz umas das outras. Elas dividem a potência do transmissor, então mais sinais significa que cada um é mais fraco.
setup-tx-audio = Áudio TX
setup-how-loud-mode-s-burst =
    O quanto a rajada deste modo é entregue a um rádio que a modula por conta própria — e em FM isso é o desvio, porque um transmissor FM converte nível de áudio em oscilação de frequência e não tem ALC para segurá-la. Pacote de 1200 bauds quer cerca de 3 kHz onde a voz quer 5, então a escala total em uma entrada de dados configurada para voz desvia demais — o que soa perfeitamente normal para quem ouve e não decodifica para ninguém. Reduza até que outras estações informem o seu sinal, ou defina o nível no rádio.

    Este nível pertence somente a este modo. Um modo que você nunca definiu começa do valor de FM, e os modos de banda lateral começam dos próprios — assim, um desvio definido para 1200 bauds nunca cai no FT8.
setup-how-loud-mode-s-over =
    O quanto a transmissão deste modo é entregue a um rádio que a modula por conta própria — um equipamento CAT pela placa de som, um FLEX, um Icom pela porta de rede. Em banda lateral, isto é a excitação no modulador: reduza até que o ALC do equipamento mal se mova e defina a potência no rádio, porque o ALC trabalhando em um modo de envelope constante é o que espalha o sinal. A Excitação alcança aqui o registrador de potência do equipamento, e não o áudio dele, então este é o nível.

    Este nível pertence somente a este modo: FT8, RTTY, PSK e MCW mantêm cada um o seu, porque não carregam o modulador do mesmo jeito. O mesmo controle está na faixa de transmissão, onde você pode alcançá-lo enquanto transmite.
setup-deviation-fm = desvio, em FM
setup-drive-ssb = excitação, em SSB
setup-grid = Locator
setup-report = Relatório
setup-r-report = R+Relatório

## Janela AMP (src/app/amp.rs)

amp-t-10-w-scale-rescaled = , escala de 10 W, redimensionada
amp-t-10-w-scale = , escala de 10 W
amp-t-2-w-scale-rescaled = , escala de 2 W, redimensionada
amp-t-2-w-scale = , escala de 2 W
amp-t-100-w-scale-rescaled = , escala de 100 W, redimensionada
amp-t-100-w-scale = , escala de 100 W
amp-t-20-w-scale-rescaled = , escala de 20 W, redimensionada
amp-t-20-w-scale = , escala de 20 W
amp-rescaled = , redimensionada
amp-and = {" "}E{" "}
amp-not-while-transmitting-switching-relay = Não durante a transmissão: comutar um relé com RF presente pode danificá-lo. Solte a tecla antes.
amp-no-amplifier-set-up-choose = Nenhum amplificador configurado. Escolha a porta dele em Configurações → Amplificador.
amp-settings = CONFIGURAÇÕES
amp-dock = ACOPLAR
amp-put-amp-window-back-inside = Recolocar a janela AMP dentro da janela principal
amp-pop-out = DESTACAR
amp-give-amp-window-window-own = Dar à janela AMP uma janela própria, para movê-la a outra tela
amp-rig = , do equipamento
amp-rig-flagged-reading = , o equipamento sinalizou a leitura
amp-drv = , de DRV
amp-rig-s-swr = , da ROE do equipamento
amp-ptt-keys-amplifier-radio-off = O PTT aciona o amplificador junto com o rádio; DESLIGADO o deixa em desvio. Clique para trocar.
amp-switch-antenna-jack = Trocar o conector de antena
amp-band-down = Faixa abaixo
amp-band-up = Faixa acima
amp-tuner = SINTONIZADOR
amp-tuned = Sintonizado
amp-tune = SINTONIZAR
amp-start-tune-then-key-steady = Inicie uma sintonia — depois acione uma portadora contínua (CW, FM ou RTTY). Pressione de novo para parar.
amp-put-tuner-line-bypass = Colocar o sintonizador na linha, ou desviá-lo
amp-tuning = Sintonizando…
amp-bottom = INFERIOR
amp-pick-any-reading-either-needle = Escolha qualquer leitura para qualquer um dos ponteiros. O ponteiro superior é branco, o inferior azul.
amp-forward-power-measured-radio-amplifier = Potência direta, medida pelo rádio: o amplificador não informa nenhuma em DESLIGADO
amp-forward-power-taken-drv-amplifier = Potência direta, obtida de DRV: o amplificador não informa nenhuma em DESLIGADO, e em desvio a potência do rádio passa direto
amp-forward-power = Potência direta
amp-reflected-power-measured-radio-amplifier = Potência refletida, medida pelo rádio: o amplificador não informa nenhuma em DESLIGADO
amp-reflected-power-worked-out-fwd = Potência refletida, calculada a partir de FWD e da ROE: o amplificador não informa nenhuma em DESLIGADO
amp-reflected-power = Potência refletida
amp-standing-wave-ratio-red-past = Relação de onda estacionária: vermelho no limite de sintonia ou acima (Configurações → Geral). Verifique a antena
amp-standing-wave-ratio-measured-radio = Relação de onda estacionária, medida pelo rádio: o amplificador não informa nenhuma em DESLIGADO
amp-standing-wave-ratio = Relação de onda estacionária
amp-drive-radio = Excitação do rádio
amp-drain-supply = Tensão de dreno
amp-drain-current = Corrente de dreno
amp-temp = TEMP
amp-heat-spreader-temperature-yellow-49 = Temperatura do dissipador: amarelo a partir de 49 °C (120 °F), vermelho acima de 65 °C (149 °F), fora da linha acima de 70 °C (158 °F). Clique para °C/°F.
amp-c = °C
amp-f = °F
amp-performance = DESEMPENHO
amp-numbers-hold-peaks-bars-are = os números mantêm os picos · as barras são ao vivo
amp-input-power = POTÊNCIA DE ENTRADA
amp-supply-volts-drain-amps = Tensão de alimentação × corrente de dreno
amp-efficiency = EFICIÊNCIA
amp-output-power-input-power = Potência de saída ÷ potência de entrada
amp-gain = GANHO
amp-output-power-over-drive-decibels = Potência de saída sobre a excitação, em decibéis
amp-off-line-over-temperature = FORA DA LINHA · temperatura excessiva
amp-connected-transmitting = CONECTADO · TRANSMITINDO
amp-connected = CONECTADO
amp-connecting = Conectando…
amp-atu-bypassed = ATU DESVIADO
amp-no-inductance-needed = não precisa de indutância
amp-no-capacitance-needed = não precisa de capacitância
amp-c-out-antenna-side = C SAÍDA · lado da antena
amp-c-transmitter-side = C ENTRADA · lado do transmissor

## Painel SSTV (src/app/panels/sstv.rs)

sstv-chunks-received-lit-still-missing = Blocos recebidos (acesos) e ainda faltando (apagados)
sstv-sstv = SSTV
sstv-auto = Automático
sstv-signal = Sinal
sstv-station-sending-mode-sdroxide-does = Uma estação está transmitindo em um modo que o sdroxide não tem. O sinal e a sintonia estão bons — simplesmente não há decodificador para este. Nada a corrigir do seu lado.
sstv-listening = escutando…
sstv-restart-rx = Reiniciar RX
sstv-abandon-picture-being-received-listen = Abandona a imagem que está sendo recebida e escuta o próximo cabeçalho. Para uma transmissão que começou a decodificar no modo errado — o receptor fica comprometido com ela até esse modo acabar.
sstv-callsign-last-station-sent-fsk = O indicativo que a última estação enviou como identificação FSK depois da imagem.
sstv-tx-slant = Inclinação de TX
sstv-transmit-clock-trim-ppm-remove = Ajuste do clock de transmissão (ppm) para remover a inclinação no decodificador da outra ponta
sstv-reset-0-ppm = Redefinir para 0 ppm
sstv-fsk-id = ID FSK
sstv-send-your-callsign-tones-after = Envia o seu indicativo em tons depois de cada imagem — a identificação que os repetidores SSTV e outros programas leem. Acrescenta cerca de 2,5 segundos, e não envia nada até que você tenha definido um indicativo.
sstv-tx-lead = Antecedência de TX
sstv-silence-sent-after-keying-before = Silêncio enviado depois de acionar e antes do líder da imagem e do código VIS. Um decodificador que perde qualquer parte desse cabeçalho não mostra imagem nenhuma, então isto cobre o intervalo entre pedir o PTT a um equipamento e ele estar de fato no ar. 0 para um SDR que aciona instantaneamente.
sstv-live = AO VIVO
sstv-waiting-picture = aguardando uma imagem…
sstv-waiting-signal = aguardando um sinal…
sstv-no-low-audio = sem áudio / áudio baixo
sstv-received = RECEBIDAS
sstv-reading-radio-s-pictures = Lendo as imagens do rádio…
sstv-decoded-pictures-collect-here = As imagens decodificadas se acumulam aqui.
sstv-click-enlarge-right-click-delete = Clique para ampliar · clique com o botão direito para excluir
sstv-delete-picture = Excluir esta imagem
sstv-loading = carregando…
sstv-transmit = TRANSMITIR
sstv-click-edit-slot-s-message = Clique para editar a mensagem deste slot · clique duplo para carregar uma imagem
sstv-change-image = Trocar imagem…
sstv-load-image = Carregar imagem…
sstv-clear = Limpar
sstv-empty-slot-s-picture-message = Esvaziar a imagem deste slot (a mensagem permanece)
sstv-banner = Faixa…
sstv-what-printed-across-top-every = O que é impresso no topo de cada imagem que esta estação envia
sstv-fetching-slot-s-picture = Buscando a imagem deste slot…
sstv-load-image-into-slot = Carregar uma imagem neste slot →
sstv-drawn-slot-s-image = Desenhado sobre a imagem deste slot
sstv-tx = {"   "}TX{"   "}
sstv-abort-tx = {" "}CANCELAR TX{" "}
sstv-delete-picture-store = Excluir esta imagem do armazenamento
sstv-received-image = Imagem recebida
sstv-no-longer-store = não está mais no armazenamento
sstv-loading-full-size = carregando em tamanho real…
sstv-save-picture = Salvar imagem…
sstv-save-copy-computer = Salvar uma cópia neste computador
sstv-delete-sure = Excluir — tem certeza?
sstv-click-again-delete-good = Clique de novo para excluir definitivamente
sstv-delete = Excluir…
sstv-image-banner = Faixa da imagem
sstv-drawn-into-every-picture-station = Desenhada em cada imagem que esta estação transmite, sobre a mensagem do próprio slot.
sstv-draw-banner = Desenhar a faixa
sstv-substituted-when-picture-composed = Substituído quando a imagem é composta:
sstv-top-left = Superior esquerdo
sstv-top-right = Superior direito
sstv-colours = Cores
sstv-strip-top-edge-fades-black = A faixa, na sua borda superior — esmaece até o preto, ou até a cor do gradiente abaixo dela
sstv-both-texts = Ambos os textos
sstv-gradient = Gradiente
sstv-fade = esmaecer até
sstv-fade-strip-top-colour-second = Esmaece a faixa da sua cor superior até uma segunda cor, em vez de até o preto.
sstv-text-outline = Contorno do texto
sstv-draw-outline-around-banner-text = Desenha um contorno em torno do texto da faixa, para que ele continue legível sobre uma faixa movimentada ou clara.
sstv-text-gradient = Gradiente do texto
sstv-fade-banner-text-colour-second = Esmaece o texto da faixa da sua cor até uma segunda cor, através da largura da imagem.
sstv-rainbow-text = Texto arco-íris
sstv-all-text-overriding-colours-above = todo o texto, substituindo as cores acima
sstv-draw-every-text-picture-banner = Desenha todo texto da imagem — faixa e mensagem — como um arco-íris horizontal, ignorando as cores acima.
sstv-message = Mensagem
sstv-slot-message-s-text-colour = A cor do texto da mensagem do slot
sstv-outline = contorno em
sstv-height = Altura
sstv-how-tall-strip-pixels-transmitted = Quão alta é a faixa, em pixels da imagem transmitida. O texto é dimensionado a partir dela — um quadro SSTV tem só 320 pixels de largura, então uma faixa mais alta é o que a torna legível na outra ponta.
sstv-reset = Redefinir
sstv-back-callsign-left-version-right = Volta ao indicativo à esquerda e à versão à direita
sstv-waiting-station-s-digital-mode = Aguardando carregar as configurações de modo digital da estação…
sstv-rifp = RIFP
sstv-size = Tamanho
sstv-calling-frequency-draft-names = A frequência de chamada que o rascunho indica
sstv-encode = Codificar
sstv-how-picture-encoded-into-object = Como a imagem é codificada no objeto que o RIFP carrega. Automático tenta cada uma e envia a menor.
sstv-try-every-encoding-send-smallest = Tenta cada codificação, envia a menor (nunca com perdas)
sstv-gray = Cinza
sstv-grayscale-depth-rifp-s-raster = Profundidade de escala de cinza. A matriz do RIFP é em escala de cinza por definição — cor não tem lugar no manifesto dele.
sstv-dither = Dither
sstv-diffuse-quantisation-error-worth-below = Difunde o erro de quantização — vale a pena abaixo de 8 bits
sstv-caption = Legenda
sstv-sent-manifest-content-hint-shown = Enviada no manifesto como dica de conteúdo, e mostrada sob a imagem pelos receptores. Viaja como texto — não é desenhada na imagem.
sstv-what-picture = O que esta imagem é
sstv-repeat-data = Repetir dados
sstv-send-every-data-frame-many = Envia cada quadro de dados essa quantidade de vezes. O RIFP é unidirecional, sem pedidos de reparo, então esta é a única recuperação que um receptor tem.
sstv-chunk = Bloco
sstv-payload-octets-per-data-frame = Octetos de carga por quadro de dados (o perfil recomenda 192)
sstv-valid-frames-frames-failed-crc = Quadros válidos, quadros que falharam no CRC, e imagens completas verificadas
sstv-no-transfer-progress = nenhuma transferência em andamento
sstv-forget-incomplete-transfer = Esquecer esta transferência incompleta

## Janelas de memórias e de manipulador de voz (src/app/windows.rs)

windows-recall = Recuperar
windows-delete = Excluir
windows-edit-name-frequency-mode-antenna = Edita o nome, a frequência, o modo, a antena e a configuração de repetidora
windows-r = {" "}R
windows-repeater-shift-tone-1750-hz = O deslocamento de repetidora, o tom e a rajada de 1750 Hz guardados com este canal
windows-abandon-edit-escape = Descartar a edição (Escape)
windows-save = SALVAR
windows-keep-changes-enter = Manter as alterações (Enter)
windows-recall-channel-without-moving-antenna = Recupera este canal sem mover a antena
windows-antenna-switch-when-channel-recalled = A antena para a qual trocar quando este canal for recuperado. — deixa o que o rádio já está usando.
windows-shift = Deslocamento
windows-tone = Tom
windows-invert = INVERTER
windows-normal = NORMAL
windows-dcs-polarity = Polaridade DCS
windows-open-every-over-tone-burst = Abre cada transmissão com a rajada de tom enquanto este canal está recuperado
windows-memories = Memórias
windows-memory-name = nome da memória
windows-store = Guardar
windows-folder-name = nome da pasta
windows-new-folder = Nova pasta
windows-import = IMPORTAR
windows-read-channel-list-chirp-csv =
    Lê uma lista de canais de um arquivo CSV do CHIRP (.csv) — uma exportação de diretório de repetidoras, uma tabela de canais marítimos ou PMR, ou qualquer outra coisa que o CHIRP possa gravar.

    Cada canal traz a sua frequência, modo, deslocamento de repetidora e tom CTCSS ou DCS. Os canais que já estão nesta lista — mesma frequência, mesmo modo — são ignorados, então reimportar um diretório atualizado acrescenta o que é novo sem duplicar o que não é.
windows-chirp-csv = CSV do CHIRP
windows-export = EXPORTAR
windows-write-list-out-chirp-csv = Grava esta lista em um arquivo CSV do CHIRP, para carregar em um portátil ou guardar como cópia de segurança.
windows-no-memories-yet = nenhuma memória ainda
windows-drag-memory-onto-folder-file = arraste uma memória para uma pasta para arquivá-la, para baixo delas para desarquivá-la
windows-drop-here-unfile = solte aqui para desarquivar
windows-sort = Ordenar
windows-order-they-were-stored = Na ordem em que foram guardadas
windows-name-ignoring-case = Por nome, sem diferenciar maiúsculas
windows-frequency = Por frequência
windows-band-then-frequency-inside-each = Por faixa, depois por frequência dentro de cada uma
windows-descending-click-ascending = Decrescente — clique para crescente
windows-ascending-click-descending = Crescente — clique para decrescente
windows-delete-folder-memories-move-top = Excluir pasta — as memórias dela passam ao nível superior
windows-rename-folder = Renomear pasta
windows-empty-drop-memories-here = vazia — solte memórias aqui
windows-voice-keyer = Manipulador de voz
windows-rec-records-your-microphone-play = REC grava pelo seu microfone, TOCAR permite ouvir o que foi gravado, TX a põe no ar — assim como uma tecla do teclado numérico, um pad MIDI, ou o send_voice_mem do rigctld.
windows-stop-store = Parar e guardar
windows-stop = PARAR
windows-play = TOCAR
windows-stop-listening = Parar de ouvir
windows-nothing-recorded-slot = Nada gravado neste slot
windows-not-while-transmitting = Não durante a transmissão
windows-listen-message-nothing-transmitted = Ouvir esta mensagem — nada é transmitido
windows-stop-transmitting = Parar a transmissão
windows-tune-active-switch-off-first = TUNE está ativo — desligue-o primeiro
windows-transmit-message = Transmitir esta mensagem
windows-erase-recording = Apagar esta gravação
windows-rade-message-encoded-digital-voice = RADE: a mensagem é codificada pelo codec de voz digital, exatamente como seria uma transmissão ao vivo.

## Janela de Áudio TX: mensagens com valores

txaudio-rec-hint = Grava uma frase de teste pelo microfone — até { $max } s, sem acionar o transmissor. Clique de novo para parar. Ela é mantida sem processamento, então TOCAR pode levar a mesma frase por cada alteração que você fizer.
txaudio-phrase-length = frase de { $secs } s
# $clip é txaudio-microphone-may-clipping-before-gets ou vazio.
txaudio-auto-set-done = Ganho de entrada definido em { $gain } dB (os picos de voz estavam em { $loud } dBFS).{ $clip }
txaudio-listening = Ouvindo… { $secs } s
txaudio-auto-set-hint = Fale normalmente por { $secs } s — como faria no ar — e o ganho de entrada é definido para que a sua voz fique com picos em torno de { $target } dBFS. Clique de novo para cancelar.
txaudio-eq-point = { $hz } Hz, { $db } dB — arraste para mover
txaudio-not-a-profile = Não é um perfil de áudio TX: { $error }
txaudio-imported = "{ $name }" importado.
txaudio-saved = "{ $name }" salvo.
txaudio-deleted = "{ $name }" excluído.
txaudio-loaded-preset = Ponto de partida "{ $name }" carregado.

## Visão 3D do sistema solar: mensagens com valores

solar-age-offline = { $age } · offline
solar-radius-exaggeration = Exagero do raio da Terra/Lua (máx. { $max }× nesta escala de órbita lunar)
solar-brightest-cell = célula mais intensa ≈ { $peak } percursos · { $bands } { $bands ->
        [one] faixa
       *[other] faixas
    }
solar-hits-in-hour = { $hits } na hora
solar-passes-from = { $name } · PASSAGENS DE { $grid }
solar-elements-age = elementos com { $age } de idade · SGP4
solar-point-at = Apontar para { $az }° ({ $compass }), elevação { $el }°.
solar-published-as = publicado como { $name }
solar-fly-there = { $name } — ↵ para ir até lá
solar-sats-tracked = { $n } de { $total } rastreados
solar-bodies-found = { $n } de { $total } corpos
solar-sats-and-bodies = { $n } de { $total } rastreados · { $bodies } de { $bodies_total } corpos
solar-confidence-on-band = { $confidence } em { $band }
solar-kp-now = { $kp } agora
solar-kp-in = { $kp } em { $hours } h
solar-valid-age = válido { $valid } · { $age } de idade
solar-full-bar = barra cheia = { $share } do mundo
solar-own-paths-memory = para onde foram os percursos desta estação · memória de { $minutes } min
solar-n-storms = { $n } tempestades
solar-clouds-legend = NUVENS { $time } · { $age } de idade · { $channels } · { $storms } · relâmpagos simulados
solar-cme-eta = ETA { $eta } (+{ $hours } h)
solar-cme-arrived = a chegada foi em { $eta } (há { $hours } h)
# $glancing e $estimated são solar-glancing / solar-direction-estimated ou vazios.
solar-cme-earth-directed = CME DIRIGIDA À TERRA { $time } · { $speed } km/s · { $when }{ $glancing }{ $estimated }

## Caixas de configuração dos modos digitais: mensagens com valores

setup-mode-setup = Configuração de { $mode }
setup-transmits-as = Transmite como { $call }
setup-decimal-degrees = Graus decimais, negativos para { $hemisphere }. Cole um diretamente de um mapa.

## Janela AMP: mensagens com valores

amp-unreliable-at-swr = { $meters } NÃO CONFIÁVEL NESTA ROE
amp-lower-drive-first = Reduza a excitação primeiro: { $drive } W sobrecarregariam o amplificador na linha (o limite dele é { $limit } W).
amp-live-value = ao vivo { $value } { $unit }
amp-tripped = DISPARADO{ $why }

## Painéis de modos digitais, comuns: mensagens com valores

panels-top-paths = topo ≈ { $n } percursos
panels-no-agreed-frequency = Nenhuma frequência combinada para { $mode } — abra isto para salvar a em que você está
panels-mode-frequencies-hint = As { $n } frequências de { $mode } — escolher uma sintoniza o dial, e você pode salvar a sua
panels-mode-frequencies = { $mode } · frequências
panels-global-convention = Uma convenção global que o plano de faixas da Região IARU { $region } não coloca em dados estreitos — confira o seu próprio plano de faixas antes de transmitir aqui.
panels-outside-data-segment = Âmbar: fora do segmento de dados da Região { $region }.
panels-save-dial = ＋ Salvar { $mhz } MHz
panels-save-dial-hint = Lembra este dial sob { $mode } e o oferece aqui daqui em diante. Salvo na estação, então cada tela ligada a este rádio o tem.
panels-tr-period = Período T/R de { $period } segundos
# $mark é panels-burst-mark durante a transmissão, senão vazio.
panels-this-turn = Esta vez: { $into } s de { $slot }. As decodificações chegam no fim dele, e é então que a próxima transmissão pode começar.{ $mark }
panels-burst-mark = {" "}A marca é onde a rajada termina, { $secs } s depois do início.
panels-tr-period-burst = Período T/R de { $slot } segundos, rajada de { $burst } s

## Janelas de memórias e de manipulador de voz: mensagens com valores

windows-n-channels = { $n } { $n ->
        [one] canal
       *[other] canais
    }
windows-slot-n = Slot { $n }
windows-record-from-mic = Gravar pelo microfone (até { $max } s)

## Painel SSTV: mensagens com valores

sstv-picture-too-big = Esta imagem tem { $size } MB — o limite é { $limit } MB.
sstv-auto-mode = Automático ({ $mode })
sstv-not-decoded = { $what } — não decodificado
sstv-last = última: { $mode }
# $place é sstv-store-on-radio ou sstv-store-in.
sstv-saved-where = As imagens decodificadas são salvas { $place } e se acumulam aqui.
sstv-older-in-store = { $n } mais antigas no armazenamento
sstv-older-load-more = { $n } mais antigas — carregar mais
sstv-delete-picture-from = Excluir esta imagem { $place }
sstv-rifp-profile = { $profile } · ~{ $khz } kHz ocupados · o dial é o centro do canal
sstv-rifp-too-wide = ⚠ { $profile } ocupa ~{ $khz } kHz — largo demais para um segmento de faixa estreita
sstv-rifp-no-frequency = O RIFP não atribui frequência, e o sdroxide o transmitirá onde você sintonizar. Um canal de { $khz } kHz só cabe onde a operação em banda larga ou FM é permitida — { $segments } — e não em um segmento de faixa estreita, menos ainda em HF. Mesmo dentro desses, as condições da sua própria licença podem ser mais restritas. Você é o operador; confira as suas próprias regras.
sstv-rifp-tx-frame = ● Quadro TX { $frame }/{ $frames } · faltam { $secs } s
sstv-rifp-sent-as = enviado como { $encoding } · { $octets } octetos
sstv-rifp-rx-stats = quadros { $frames } · ruins { $bad } · imagens { $pictures }
sstv-rifp-session-idle = sessão { $session } · ociosa há { $secs } s
sstv-rifp-session-waiting = sessão { $session } · blocos retidos, ainda aguardando o manifesto · ociosa há { $secs } s
sstv-store-on-radio = no rádio, em { $dir }
sstv-store-in = em { $dir }

## Decodificações (src/app/panels/decodes.rs)

decodes-decodes = DECODIFICAÇÕES
decodes-adif = ADIF
decodes-sort = Ordenar
decodes-single-list = Lista única
decodes-every-decode-one-list-newest = Cada decodificação em uma só lista, com a vez mais recente primeiro, em vez de blocos de vezes pares/ímpares. Os chips de Ordenar então ordenam a lista inteira em vez de cada vez, e cada linha traz o horário do seu slot — ciano para um slot par, dourado para um ímpar.
decodes-cq-only = Somente CQ
decodes-only-stations-calling-cq-only = Somente estações chamando CQ — e somente as chamadas a que você pode responder: um CQ dirigido (DX, EU, JA, POTA, TEST …) aparece quando cita você e fica oculto quando cita outra pessoa.
decodes-new-only = Somente novas
decodes-only-stations-would-new-entity = Somente estações que seriam novas: entidade, slot de faixa, locator ou indicativo
decodes-auto-tx-frq = FRQ TX automática
decodes-overridden-hold-tx-lift-hold = Substituída por Manter TX. Libere a manutenção para escolher para que lado a frequência de transmissão se move.
decodes-pick-our-transmit-frequency-automatically = Escolhe nossa frequência de transmissão automaticamente: o ponto mais silencioso no período em que transmitimos, em vez da frequência de quem estamos respondendo — eles transmitem no outro período, então a frequência deles não diz nada sobre quem está lá quando acionamos. Desligado NÃO mantém a frequência: responde na frequência da estação chamada. Para mantê-la, use Manter TX.
decodes-hold-tx = Manter TX
decodes-pin-transmit-tone-where-nothing =
    Fixa o tom de transmissão onde está. Nada o move: não responder a uma estação, nem a fila de chamadas, nem chamar CQ, nem um clique em uma decodificação ou na cascata. Desligue para mover e depois ligue de novo.

    Trocar de faixa é a única exceção, e é ato seu: o deslocamento que você definiu por último na nova faixa volta com ela, então um valor de 60 m não é levado para 20 m, e o de 20 m não é levado para 60 m.

    Para quando a licença é mais estreita que o plano de faixas — em um dial britânico de 60 m, em 5357 kHz, a alocação termina em 5358,0, então o tom precisa ficar abaixo de 1000 Hz, e qualquer um dos dois movimentos automáticos sairá da faixa entre uma transmissão e a seguinte.
decodes-type-transmit-offset-hz-then = Digite o deslocamento de transmissão em Hz, depois pressione Enter. Quando a sua licença é mais estreita que o plano de faixas, toda a faixa abaixo da borda fica à sua escolha, então nada é sugerido aqui.
decodes-clear-rx = {" "}LIMPAR RX{" "}
decodes-empty-decode-list-nothing-air = Esvazia a lista de decodificações. Nada que está no ar para.
decodes-even = PAR
decodes-text = TEXTO
decodes-dxcc = DXCC
decodes-band = FAIXA
decodes-grid = LOCATOR
decodes-dupe = DUPLICADA
decodes-reply = RESPOSTA
decodes-queued-click-remove = Na fila — clique para remover
decodes-work-station-after-current-one = Trabalhar esta estação depois da atual
decodes-qsos-worked-since-sdroxide-was = QSOs trabalhados desde que o sdroxide foi iniciado
decodes-setup = ⚙ CONFIG
decodes-qso-complete = ✓ QSO CONCLUÍDO
decodes-contact-complete-log-held-here = O contato está concluído e no log. Ele fica aqui por alguns minutos para que a mensagem final possa ser reenviada se a outra estação repetir a dela — nada mais é devido.
decodes-tx = ● TX
decodes-dxpedition-mode-transmit-frequency-held = Modo DXpedition. A frequência de transmissão é mantida fora da metade do Fox na banda de passagem (abaixo de 1000 Hz) até que o Fox responda.
decodes-watchdog = VIGIA
decodes-transmitting-stopped-no-reply-no = Transmissão interrompida: nenhuma resposta e nenhuma ação durante o período do vigia. Chame CQ ou escolha uma mensagem para retomar.
decodes-well-inside-tolerance = Bem dentro da tolerância.
decodes-no-active-qso-pick-decode = nenhum QSO ativo — escolha uma decodificação para responder, ou Chamar CQ
decodes-ft8-engine-idle = Motor FT8 ocioso
decodes-being-worked = sendo trabalhada
decodes-clear = LIMPAR
decodes-no-messages = — nenhuma mensagem —
decodes-call-cq = {"  "}CHAMAR CQ{"  "}
decodes-stop-qso = {" "}PARAR QSO{" "}
decodes-stop-tx = {" "}PARAR TX{" "}
decodes-free-text-13-chars = texto livre (13 caracteres)
decodes-send = ENVIAR

## sat (src/app/sat.rs)

sat-pass-profile = PERFIL DE PASSAGEM
sat-satellite = SATÉLITE
sat-satellites = SATÉLITES
sat-passes = PASSAGENS
sat-set-your-grid-passes-settings = Defina o seu locator para as passagens (Configurações ▸ Geral).
sat-next = PRÓXIMA
sat-peak = PICO ≥
sat-all-satellites = todos os satélites
sat-off-only-satellites-frequencies-file = Desligado: somente satélites com frequências cadastradas — os que você pode trabalhar — e qualquer um que você tenha colado. Ligado: todo satélite da lista, que podem ser milhares com uma assinatura e levam um tempo para pesquisar.
sat-no-passes-window-lower-peak = Nenhuma passagem nesta janela. Reduza o pico ou amplie a janela.
sat-length = DURAÇÃO
sat-peak-2 = PICO
sat-from-to = DE → PARA
sat-up-now = ● visível agora
sat-unlock = DESTRAVAR
sat-release-lock-dial-stays-where = Libera o travamento; o dial fica onde está
sat-azimuth = AZIMUTE
sat-elevation = ELEVAÇÃO
sat-range = DISTÂNCIA
sat-rate = TAXA
sat-downlink = DESCIDA
sat-doppler-rx = DOPPLER RX
sat-uplink = SUBIDA
sat-doppler-tx = DOPPLER TX
sat-no-pass-inside-next-48 = Nenhuma passagem nas próximas 48 horas a partir do seu QTH.
sat-elements-stale-corrections-suspended-until = ⚠ Elementos desatualizados — correções suspensas até uma atualização do TLE
sat-doppler-correction-needs-iq-front = A correção Doppler exige um front end I/Q — um equipamento CAT só rastreia
sat-rotator-not-connected = Rotor: não conectado
sat-doppler = DOPPLER
sat-apply-computed-correction-rx-tx = Aplicar a correção calculada ao RX e ao TX
sat-rotator = ROTOR
sat-steer-antenna-through-rotctld-client = Comanda a antena pelo cliente rotctld (Configurações ▸ Servidores)
sat-name-norad = nome ou NORAD
sat-set-your-grid-passes-settings-2 = defina o seu locator para as passagens (Configurações ▸ Geral)
sat-norad = NORAD
sat-pass = PASSAGEM
sat-always-visible = sempre visível
sat-not-here = não daqui
sat-no-elements = sem elementos
sat-pick-satellite-see-links-lock = Escolha um satélite para ver os enlaces dele e travar.
sat-no-frequencies-file-one-add = Não há frequências cadastradas para este — adicione-as em Configurações ▸ TLE.
sat-link = ENLACE
sat-downlink-mhz = DESCIDA (MHz)
sat-uplink-mhz = SUBIDA (MHz)
sat-mode = MODO
sat-tune = SINTONIZAR
sat-set-dial-mode-link-no = Define o dial e o modo para este enlace — sem travamento, sem correção
sat-locked = {" "}TRAVADO{" "}
sat-lock = {" "}TRAVAR{" "}
sat-track-satellite-doppler-correction-rx = Rastreia este satélite: correção Doppler no RX e no TX, subida mapeada no transponder, e a visão 3D o acompanha

## wefax (src/app/panels/wefax.rs)

wefax-wefax = WEFAX
wefax-stop = {" "}■ PARAR{" "}
wefax-end-chart-now-save-what = Encerra a carta agora e salva o que chegou
wefax-start = {" "}● INICIAR{" "}
wefax-start-chart-now-without-waiting = Inicia uma carta agora, sem esperar um tom de início
wefax-phasing = fasando…
wefax-subcarrier-offset-1900-hz-tune = Deslocamento da subportadora a partir de 1900 Hz. Sintonize para aproximadamente zero: a portadora do fax vai de 1500 Hz (preto) a 2300 Hz (branco), e um receptor a algumas centenas de hertz de distância corta a imagem para preto ou branco sólidos.
wefax-auto-start = INÍCIO AUTOMÁTICO
wefax-begin-chart-when-300-hz = Começa uma carta quando o tom de início de 300 Hz (IOC 576) ou de 675 Hz é ouvido — e encerra a em andamento e fasa a nova quando chega a próxima transmissão, tenha ou não vindo antes um tom de parada.
wefax-auto-stop = PARADA AUTOMÁTICA
wefax-end-450-hz-stop-tone = Encerra no tom de parada de 450 Hz. Desligue para continuar gravando durante uma estação que envia várias cartas em sequência.
wefax-phase = FASE
wefax-slant = INCLINAÇÃO
wefax-sample-clock-trim-if-chart = Ajuste do clock de amostragem. Se a carta inclina para a esquerda, aumente; para a direita, reduza. Uma placa de som 100 ppm fora desloca uma carta de um quarto de hora quase uma linha inteira de lado.
wefax-view = VISÃO
wefax-scale-chart-panel-width = Ajustar a carta à largura do painel
wefax-whole = INTEIRA
wefax-shrink-chart-until-all-view = Reduz a carta até que ela inteira caiba na tela de uma vez
wefax-half-size = Meio tamanho
wefax-one-screen-pixel-per-fax = Um pixel da tela por pixel do fax — role para ver detalhes
wefax-twice-size-scroll-move-around = Tamanho dobrado; role para se mover pela carta
wefax-height = ALTURA
wefax-stretch-picture-vertically-chart-came = Estica a imagem verticalmente. Uma carta que saiu achatada ou esticada geralmente está sendo decodificada na taxa de linhas errada — isto a torna legível, e os chips LPM a corrigem de verdade. Clique duas vezes para digitar um valor.
wefax-reset = REDEFINIR
wefax-back-picture-s-own-proportions = Volta às proporções da própria imagem
wefax-follow = SEGUIR
wefax-keep-newest-lines-view-scrolling = Mantém as linhas mais recentes na tela. Rolar para cima desliga isto para que você possa ler o que já chegou; rolar de volta até o fim liga de novo.
wefax-tune-fax-schedule-usb-wait = Sintonize uma programação de fax em USB e aguarde um tom de início, ou pressione INICIAR para começar no meio da carta.
wefax-saved = SALVAS
wefax-path = CAMINHO
wefax-reading-radio-s-charts = Lendo as cartas do rádio…
wefax-finished-charts-collect-here = As cartas concluídas se acumulam aqui.
wefax-delete-chart = Excluir esta carta
wefax-loading = carregando…
wefax-stations = ESTAÇÕES
wefax-broadcast-radiofax-schedules-picking-one = Programações de radiofax de radiodifusão — escolher uma sintoniza o dial
wefax-frequencies-are-published-carrier-dial = As frequências são a portadora publicada; o dial fica 1,9 kHz abaixo dela, o que é feito para você. As programações mudam e as estações encerram — trate isto como onde começar a procurar, não como uma tabela de horários.
wefax-delete-chart-store = Excluir esta carta do armazenamento
wefax-newer = ◀ MAIS NOVA
wefax-chart-received-after-one = A carta recebida depois desta
wefax-older = MAIS ANTIGA ▶
wefax-chart-received-before-one = A carta recebida antes desta
wefax-no-longer-store = não está mais no armazenamento
wefax-loading-full-size = carregando em tamanho real…
wefax-save = SALVAR
wefax-save-copy-chart-computer = Salvar uma cópia desta carta neste computador
wefax-sure = CERTEZA?
wefax-click-again-delete-chart-good = Clique de novo para excluir esta carta definitivamente
wefax-delete = EXCLUIR
wefax-file-chart-was-saved = O arquivo em que esta carta foi salva

## aprs (src/app/panels/aprs.rs)

aprs-starting-aprs-modem = iniciando o modem APRS…
aprs-aprs = APRS
aprs-no-callsign = sem indicativo
aprs-nothing-will-transmitted-no-beacon = Nada será transmitido — nenhuma baliza, nenhuma mensagem, nem mesmo uma confirmação — até que esta estação tenha um indicativo. Defina um em Configurações → Geral, ou um específico do APRS com o SSID dele em CONFIG.
aprs-no-position = sem posição
aprs-beacon-needs-somewhere-report-fill = Uma baliza precisa de um lugar para informar. Preencha o seu locator em Configurações → Geral, ou informe coordenadas em CONFIG.
aprs-setup = CONFIG
aprs-callsign-symbol-digipeater-path-position = Indicativo, símbolo, caminho de digipeaters, posição e baliza
aprs-how-often-beacon-your-position = Com que frequência transmitir a sua posição, sem supervisão. `off` — o padrão — nunca transmite a baliza: escolher um modo não pode colocar uma estação no ar. Trinta minutos é a convenção para uma estação fixa, com mais frequência para uma em movimento, e cada baliza é tempo de canal de outra pessoa.
aprs-beacon = {" "}BALIZA{" "}
aprs-station-has-no-callsign-see = Esta estação não tem indicativo — veja o aviso à esquerda.
aprs-station-has-no-position-report = Esta estação não tem posição para informar — veja o aviso à esquerda.
aprs-send-one-position-report-now = Envia um relatório de posição agora, sem esperar o intervalo. Ele espera o canal ficar livre como tudo o mais.
aprs-stations = ESTAÇÕES
aprs-nothing-heard-yet = nada ouvido ainda
aprs-heard-direct = ouvida diretamente
aprs-two-characters-station-sent-say = Os dois caracteres que a estação enviou para dizer o que ela é
aprs-killed = CANCELADO
aprs-station-put-object-here-has = A estação que colocou este objeto aqui o cancelou
aprs-sender-deliberately-reported-square-rather = O remetente informou deliberadamente um quadrado, e não um ponto. O mapa desenha o quadrado.
aprs-digipeaters-last-frame-came-through = Os digipeaters por onde passou o último quadro. Um `*` marca o que de fato o repetiu.
aprs-message = {" "}MENSAGEM{" "}
aprs-address-message-box-station = Endereça a caixa de mensagem a esta estação
aprs-center = {" "}CENTRALIZAR{" "}
aprs-put-station-middle-map-double = Coloca esta estação no meio do mapa. Clique duas vezes no mapa para devolver a visão ao ajuste automático.
aprs-channel = CANAL
aprs-messages = MENSAGENS
aprs-frames-arrived-failed-their-check = Quadros que chegaram e falharam na verificação de sequência — uma colisão, um desvanecimento, ou um sinal fraco demais para ler. Uma contagem subindo sem nada decodificado significa que o modem *está* ouvindo o canal: confira o medidor de nível e abra o squelch do rádio.
aprs-frames-read-cleanly-off-channel = Quadros lidos limpos do canal que não eram APRS — a sessão de pacote de outra pessoa, ou um formato que esta versão não decodifica.
aprs-every-frame-channel-arrived-other = Cada quadro do canal, como chegou — inclusive o tráfego de outras pessoas e tudo o que o decodificador não conseguiu ler.
aprs-nothing-channel-yet = nada no canal ainda
aprs-no-messages = Nenhuma mensagem.
aprs-waiting-channel = aguardando o canal
aprs-sent-retried-still-waiting-acknowledged = enviada e reenviada, ainda aguardando confirmação
aprs-air-waiting-acknowledged = no ar, aguardando confirmação
aprs-acknowledged-far-end = confirmada pela outra ponta
aprs-refused-far-end-not-retried = recusada pela outra ponta — não reenviada
aprs-no-ack = sem confirmação
aprs-no-answer-after-every-retry = nenhuma resposta após todas as tentativas
aprs-send = {" "}ENVIAR{" "}
aprs-station-has-no-callsign-so = Esta estação não tem indicativo, então nada pode ser transmitido. Defina um em Configurações → Geral, ou um específico do APRS com o SSID dele em CONFIG.
aprs-needs-station-address-something-say = Precisa de uma estação para endereçar e de algo a dizer.
aprs-send-keep-retrying-until-far = Envia, e continua tentando até que a outra ponta confirme. Mensagens são a única coisa neste canal que recebe resposta.
aprs-busy = OCUPADO
aprs-stations-heard-still-inside-window = Estações ouvidas que ainda estão dentro da janela definida em CONFIG APRS, e se outra estação está no canal agora. Nada aciona enquanto estiver ocupado.
aprs-tx = ● TX
aprs-over-progress-frames-waiting-channel = A transmissão em andamento, os quadros aguardando o canal ficar livre, ou o tempo até a próxima baliza agendada.
aprs-audio-reaching-modem-flat-means = Áudio chegando ao modem. Plano significa que nada está chegando — confira se a saída de dados do rádio é a que o sdroxide está escutando. Acende em verde enquanto o modem ouve uma portadora.

## vdl2 (src/app/panels/vdl2.rs)

vdl2-starting-vdl2-decoder = iniciando o decodificador VDL2…
vdl2-aircraft-datalink = enlace de dados de aeronaves
vdl2-tune-middle-vdl2-group-decoder = Sintoniza no meio do grupo VDL2. A janela própria do decodificador desliza a partir daí para abranger o máximo possível dos quatorze canais que o receptor alcança.
vdl2-setup = CONFIG
vdl2-which-channels-listen-how-hard = Em quais canais escutar, quão forte uma rajada precisa ser, e quanto log manter
vdl2-save-vdl2-message-log-file = Salvar o log de mensagens VDL2 em um arquivo
vdl2-vdl2-channel = Canal VDL2
vdl2-messages = MENSAGENS
vdl2-address-flight-registration-label = endereço, voo, matrícula ou rótulo
vdl2-held = RETIDO
vdl2-hold = RETER
vdl2-hold-message-log-where-so = Mantém o log de mensagens onde está, para que uma delas possa ser lida sem que a próxima transmissão a empurre para cima na tela. O decodificador continua rodando e os contadores continuam andando; soltar mostra tudo o que chegou nesse meio-tempo.
vdl2-nothing-decoded-yet = nada decodificado ainda
vdl2-nothing-matches-filter = nada corresponde ao filtro
vdl2-stations = ESTAÇÕES
vdl2-nobody-heard-yet = ninguém ouvido ainda
vdl2-vdl2-setup = Configuração VDL2
vdl2-channels = Canais
vdl2-every-25-khz-slot-136 = Cada slot de 25 kHz de 136,650 a 136,975 MHz. Um conversor de descida para cada um, todos dentro da mesma janela do receptor. Desligar um economiza um pouco de tempo de processador; não torna os outros mais sensíveis.
vdl2-burst-threshold = Limiar de rajada
vdl2-above-each-channel-s-own = acima do piso de ruído próprio de cada canal
vdl2-lower-catches-weaker-transmissions-costs = Um valor menor captura transmissões mais fracas e custa tempo de processador com ruído; um maior as perde. O piso é aprendido por canal, e uma alteração aqui não o descarta.
vdl2-keep-log = Manter no log
vdl2-messages-2 = {" "}mensagens
vdl2-track-most = Rastrear no máximo
vdl2-stations-2 = {" "}estações
vdl2-longest-silent-go-first = as que estão mais tempo em silêncio saem primeiro
vdl2-forget-station-after = Esquecer uma estação após
vdl2-nothing-heard = sem nada ouvido dela
vdl2-show-unread-payloads = Mostrar cargas não lidas
vdl2-hex = em hexadecimal
vdl2-frames-carrying-x-25-clnp = Quadros que carregam X.25, CLNP ou as aplicações de enlace de dados acima deles. O SDRoxide os identifica e mostra os bytes em vez de lê-los, e ocultá-los esconderia quanto do tráfego eles são.
vdl2-registration = Matrícula
vdl2-flight = Voo
vdl2-label = Rótulo
vdl2-block = Bloco
vdl2-sequence = Sequência
vdl2-mode = Modo
vdl2-more-blocks-follow = mais blocos seguem
vdl2-check = Verificação
vdl2-message-crc-good = CRC da mensagem correto
vdl2-message-crc-not-checked = CRC da mensagem não verificado
vdl2-parity = Paridade
vdl2-link-control-no-payload = controle de enlace, sem carga
vdl2-name = NOME
vdl2-addr = END
vdl2-type = TIPO
vdl2-msgs = MSGS

## cw (src/app/panels/cw.rs)
cw-tone-being-copied-tone-transmitted = O tom que está sendo copiado e o tom transmitido — em CW são a mesma frequência. Clique na cascata para movê-lo sobre um sinal.
cw-down-10-hz = Descer 10 Hz
cw-up-10-hz = Subir 10 Hz
cw-qrg-main-readout-tuning-line = QRG: a leitura principal e a linha de sintonia ficam na frequência em que se opera. Clique para o dial, com um tom de sidetone abaixo dele — o que a maioria dos rádios mostra.
cw-put-main-readout-tuning-line =
    Coloca a leitura principal e a linha de sintonia sobre o sinal, em vez de sobre o dial, para que a frequência mostrada seja a que os dois operadores citariam, e a linha de sintonia fique no meio da banda de passagem. A sintonia não muda; só os números se movem.

    Clicar em um sinal o coloca no cursor apenas com a precisão que o passo de clique permite — Controles → arredondamento do clique para sintonizar, 10 Hz por padrão. Um passo grosso deixa o sinal fora do tom em até metade dele, e a leitura informa isso.
cw-sending-speed-read-off-signal = Velocidade de envio lida do sinal
cw-signal-noise-500-hz-same = Sinal para ruído em 500 Hz — o mesmo número que um relatório cita
cw-how-far-off-cursor-signal = Quão longe do cursor o sinal realmente está. O decodificador o segue; a banda de passagem não, então empurre o cursor se isto crescer.
cw-listening = — escutando —
cw-tx = ● TX
cw-trainer = TREINADOR
cw-learn-morse-read-characters-words = Aprenda Morse: leia caracteres e palavras, toque-os na sua velocidade, e um exercício de Koch que acrescenta um caractere por vez. Toca aqui, pelos alto-falantes deste computador — nada aciona o rádio.
cw-nothing-copied-yet = — nada copiado ainda —
cw-key-send-characters-you-send = Manipule para enviar — os caracteres que você envia aparecem aqui.
cw-type-line-return-sends = Digite uma linha, Enter envia…
cw-type-here-send = Digite aqui para enviar…
cw-key = {" "}MANIPULADOR ●{" "}
cw-key-2 = {" "}MANIPULADOR{" "}
cw-hold-key-bound-cw-straight = Segure a tecla associada ao manipulador reto de CW — Barra de espaço por padrão, e qualquer tecla que você quiser em Configurações → Controles — como um manipulador reto: abaixada enquanto é mantida, levantada ao soltar, em vez de digitar texto. A caixa de transmissão fica travada enquanto isto está ligado, e todo o manipulador automático passa para a tecla: o texto que estava na fila é descartado.
cw-radio-sends-own-keyer-text =
    Este rádio envia pelo próprio manipulador: o texto segue pela porta de controle e o equipamento cronometra os elementos, então não há nada entre o manipulador reto e o ar para uma mão conduzir.

    Para manipular à mão, defina Acionamento de CW como "Placa de som (MCW)" em Configurações → Rádio. O equipamento então fica em uma banda lateral e o sidetone do próprio manipulador é transmitido como áudio, que é o caminho que o manipulador reto conduz.
cw-watchdog = VIGIA
cw-straight-key-was-held-down = O manipulador reto ficou abaixado tempo demais — uma liberação perdida, e não uma mão — então a portadora foi cortada e a transmissão desligada. Pressione a tecla de novo para continuar.
cw-tx-2 = {"  "}TX LIGADO{"  "}
cw-tx-3 = {"   "}TX{"   "}
cw-send-what-box-now-without = Envia agora o que está na caixa, sem esperar o Enter
cw-hold-key-down-between-characters = Mantém a tecla abaixada entre os caracteres, para que nada digitado espere
cw-call-cq = {" "}CHAMAR CQ{" "}
cw-nocall = SEMCHAMADA
cw-clear = {" "}LIMPAR{" "}
cw-stop-sending-drop-whatever-has = Para de enviar e descarta o que ainda não saiu. Com o manipulador reto ligado, a leitura de retorno do que foi manipulado vai junto.
cw-sidetone = SIDETONE
cw-play-keyed-tone-through-computer = Toca o tom manipulado pelos alto-falantes deste computador, além de enviá-lo, para que você ouça o que está mandando. Em `Placa de som (MCW)` o tom vai para o rádio e para mais nada, então sem isto você envia em silêncio. Desligue onde o monitor do próprio rádio ou uma cópia fora do ar já faz o trabalho.
cw-hold-what-typed-until-return = Segura o que é digitado até o Enter, e então envia a linha de uma vez, em vez de manipular cada caractere à medida que é digitado. Vale ter em um transceptor que se aciona sozinho a partir do texto, onde cada entrega ao manipulador dele é mais um ciclo de transmissão e recepção.
cw-your-own-message-buttons-contest = Seus próprios botões de mensagem — uma troca de concurso, uma resposta com nome e QTH, TNX 73 GL. Cada um envia o texto inteiro de uma vez, e F2–F10 acionam os nove primeiros. Eles acompanham a configuração da estação, então um cliente remoto também os tem.
cw-cw-messages = MENSAGENS CW
cw-clear-rx = {" "}LIMPAR RX{" "}
cw-empty-receive-window-straight-key = Esvazia a janela de recepção e a leitura de retorno do manipulador reto. Nada que está no ar para.
cw-lock = TRAVAR
cw-decode-speed-set-here-instead = Decodifica na velocidade definida aqui, em vez de lê-la do sinal. Ajuda um sinal fraco demais para que a busca de velocidade se fixe.
cw-farnsworth-send-characters-full-speed = Farnsworth: envia os caracteres em velocidade plena e estica apenas os intervalos entre eles, para que sejam ouvidos no ritmo certo mas cheguem devagar o bastante para serem anotados.
cw-off-normal-spacing = Desligado — espaçamento normal
cw-keying-speed = Velocidade de manipulação
cw-idle-0 = OCIOSO 0
cw-how-long-transmit-held-after = Por quanto tempo a transmissão é mantida depois do último caractere, ou depois de o manipulador reto ser solto, antes de a portadora cair. Um valor maior cobre as pausas de quem digita devagar; um menor sai da frequência mais cedo; Desligado a derruba assim que tudo na fila já saiu.
cw-off-drop-once = Desligado — derrubar de imediato

## textmodem (src/app/panels/text_modem.rs)

textmodem-tune-down-10-hz = Sintonizar 10 Hz para baixo
textmodem-tune-up-10-hz = Sintonizar 10 Hz para cima
textmodem-tx = ● TX
textmodem-listening = — escutando —
textmodem-type-line-return-sends = Digite uma linha, Enter envia…
textmodem-type-here-transmit = Digite aqui para transmitir…
textmodem-tx-2 = {"  "}TX LIGADO{"  "}
textmodem-tx-3 = {"   "}TX{"   "}
textmodem-call-cq = {" "}CHAMAR CQ{" "}
textmodem-nocall = SEMCHAMADA
textmodem-clear = {" "}LIMPAR{" "}
textmodem-hold-what-typed-until-return = Segura o que é digitado até o Enter, e então envia a linha de uma vez, em vez de transmitir cada caractere no ar à medida que é digitado. Permite que uma linha seja relida e corrigida antes que qualquer parte saia.
textmodem-your-own-message-buttons-working = Seus próprios botões de mensagem — condições de trabalho, o tempo, uma resposta padrão. Cada um envia o texto inteiro de uma vez, e F2–F10 acionam os nove primeiros. Eles acompanham a configuração da estação, então um cliente remoto também os tem.
textmodem-messages = MENSAGENS
textmodem-hell = HELL
textmodem-contrast = Contraste
textmodem-harder-softer-dots-redraws-whole = Pontos mais fortes ou mais fracos — redesenha a faixa inteira
textmodem-width = Largura
textmodem-t-2row = {" "}2LINHAS{" "}
textmodem-draw-every-column-twice-stacked = Desenha cada coluna duas vezes, empilhadas. O Hell não tem sincronismo vertical, então isto mantém uma cópia completa do texto legível qualquer que seja a fase.
textmodem-rev = {" "}INV{" "}
textmodem-reverse-video-light-dots-dark = Vídeo invertido — pontos claros em papel escuro
textmodem-clear-rx = {" "}LIMPAR RX{" "}
textmodem-hold-channel-idle-sends-blank = Mantém o canal: ocioso envia papel em branco, para que a faixa continue rolando
textmodem-hold-what-typed-until-return-2 = Segura o que é digitado até o Enter, e então envia a linha de uma vez, em vez de pintar cada caractere na faixa à medida que é digitado. Uma correção digitada ao vivo já está no papel; uma feita antes do Enter nunca esteve.
textmodem-shift = Deslocamento
textmodem-shift-5-hz = Deslocamento −5 Hz
textmodem-shift-5-hz-2 = Deslocamento +5 Hz
textmodem-baud = Baud
textmodem-reverse-swap-mark-space-needed = Inverter: troca marca e espaço. Necessário quando o sinal é recebido na banda lateral oposta à que foi enviado, o que inverte os tons.
textmodem-track-tuning-error-automatically-instead = Acompanha o erro de sintonia automaticamente, em vez de ficar preso ao cursor.
textmodem-tones = Tons
textmodem-mode = Modo
textmodem-off-keyed = acionado liga/desliga

## ais (src/app/panels/ais.rs)

ais-starting-ais-decoder = iniciando o decodificador AIS…
ais-tune-between-two-ais-channels = Sintoniza entre os dois canais AIS — 161,975 (AIS 1, marítimo 87B) e 162,025 (AIS 2, 88B). O dial fica no meio porque nada transmite ali, então o pico de CC de um receptor de zero-IF não cai em nenhum dos dois.
ais-setup = CONFIG
ais-channels-timeouts-trail-length-how = Canais, tempos limite, comprimento do rastro e até onde os vetores alcançam
ais-vessels = EMBARCAÇÕES
ais-nothing-heard-yet = nada ouvido ainda
ais-virtual-nothing-there = virtual — não há nada ali
ais-position-stale = posição desatualizada
ais-search-rescue-transmitter-man-overboard = um transmissor de busca e salvamento, baliza de homem ao mar ou EPIRB AIS
ais-call-sign = indicativo de chamada
ais-station-time = hora da estação
ais-dgnss = {" "}(DGNSS)
ais-first-heard = ouvido pela primeira vez
ais-center = CENTRALIZAR
ais-put-vessel-middle-chart = Coloca esta embarcação no meio da carta
ais-last-message-station-nmea-sentences = A última mensagem desta estação, como as sentenças NMEA que um receptor colocaria no fio. Cole-a em qualquer outro decodificador AIS para conferir o que o sdroxide fez dela.
ais-ais-setup = Configuração AIS
ais-channels = Canais
ais-drop-chart-after = Remover da carta após
ais-without-position-report = sem relatório de posição
ais-five-minutes-default-not-ads = Cinco minutos por padrão, não os dez segundos do ADS-B: uma embarcação fundeada informa uma vez a cada três minutos, e uma janela menor apagaria a maior parte de um porto entre dois relatórios perfeitamente bons. Depois disso a embarcação sai da carta e a linha dela fica cinza — não esmaecida, porque um símbolo apagado em uma posição antiga ainda é uma afirmação sobre onde o navio está, na mesma tinta que as verdadeiras.
ais-drop-list-after = Remover da lista após
ais-nothing-heard-all = sem nada ouvido
ais-message-carrying-ship-s-name = A mensagem que traz o nome de um navio volta a cada seis minutos, então uma janela curta na lista continua descartando embarcações justamente antes de elas dizerem como se chamam.
ais-trail-length = Comprimento do rastro
ais-history-behind-each-target = de histórico atrás de cada alvo
ais-minutes-rather-than-points-because = Em minutos, e não em pontos, porque as taxas de relatório do AIS abrangem duas ordens de grandeza: uma contagem fixa seria oitenta segundos de uma balsa e duas horas de um petroleiro fundeado, desenhados do mesmo jeito. Zero desliga os rastros.
ais-speed-vector = Vetor de velocidade
ais-how-far-ahead-vector-reaches = até onde o vetor alcança à frente
ais-slot-threshold = Limiar do slot
ais-above-channel-s-learned-noise = acima do piso de ruído aprendido do canal
ais-track-most = Rastrear no máximo
ais-vessels-2 = {" "}embarcações
ais-ais-receive-only-here-always = O AIS é somente recepção aqui e sempre será: é um serviço de segurança da vida, e colocar tráfego falso de embarcações nele não é algo que uma licença cubra.
ais-fill-my-grid-digimode-setup = Preencha Meu locator na configuração de modos digitais para distâncias e rumos, e para que a carta se enquadre em torno de onde você está, e não em torno do que estiver mais longe.
ais-name = NOME
ais-mmsi = MMSI
ais-type = TIPO

## qo100 (src/app/qo100.rs)

qo100-receiver-cannot-reach-10489-750 = Este receptor não alcança 10489,750 MHz sozinho — configure antes um deslocamento de LNB/conversor (Configurações ▸ Rádio)
qo100-run-fast-spectral-tracker-every = Roda o rastreador espectral rápido: a cada segundo, procura na janela de estacionamento os dois lóbulos simétricos da baliza e informa onde está a portadora
qo100-telemetry = TELEMETRIA
qo100-also-run-ao-40-uncoded = Também roda o decodificador de quadros sem codificação do AO-40 — palavra de sincronismo, CRC e o texto de telemetria — com uma leitura passo a passo (portadora → sincronismo → CRC) de até onde cada passagem chega
qo100-auto = AUTO
qo100-auto-correct-let-tracker-adjust = Correção automática: deixa o rastreador ajustar sozinho o deslocamento do conversor/LNB — um laço fechado lento que, com uma estimativa limpa e estável, empurra a baliza de volta para 10489,750 MHz e a mantém ali conforme o LNB deriva. Reabre o front end cada vez que age.
qo100-narrower-search-smaller-slice = Mais estreita — pesquisa uma fatia menor
qo100-wider-when-beacon-isn-t = Mais larga — para quando a baliza não é encontrada na largura atual
qo100-clear-mark = limpar marcação
qo100-drop-hand-placed-beacon-mark = Remove a marcação da baliza colocada à mão e volta à leitura do próprio decodificador
qo100-lo = lo −
qo100-lo-2 = lo +
qo100-hi = hi −
qo100-hi-2 = hi +
qo100-radio-s-own-tuning-range = A faixa de sintonia própria deste rádio não alcança 10489,750 MHz — ele precisa de um deslocamento de LNB/conversor (Configurações ▸ Rádio ▸ Conversor) antes que esta janela possa fazer qualquer coisa.
qo100-open-settings-radio = Abrir Configurações ▸ Rádio
qo100-tune-10489-750-mhz = Sintonizar em 10489,750 MHz
qo100-tip-if-you-can-see = dica: se você vê a baliza na faixa mas ela não trava, clique duas vezes nela — isso a marca como 10489,750 MHz e permite que APLICAR corrija para ela
qo100-tracker = RASTREADOR
qo100-no-twin-lobe-shape-parking = nenhuma forma de dois lóbulos na janela de estacionamento
qo100-shape = FORMA
qo100-drift = DERIVA
qo100-need-longer-run-estimates = — (precisa de uma sequência maior de estimativas)
qo100-cycles = CICLOS
qo100-receiver = RECEPTOR
qo100-target = ALVO
qo100-measured = MEDIDO
qo100-clicked = {"  "}(clicado)
qo100-tracker-2 = {"  "}(rastreador)
qo100-not-locked-yet = ainda não travado
qo100-converter-offset = DESLOCAMENTO DO CONVERSOR
qo100-decode = DECODIFICAR
qo100-decode-filling-first-frame-buffer = DECODIFICAR — enchendo o primeiro buffer de quadros…
qo100-decode-off-turn-telemetry-chip = DECODIFICAR — desligado. Ligue o chip TELEMETRIA (linha superior) para a leitura portadora → sincronismo → CRC.
qo100-apply-correction = {" "}APLICAR CORREÇÃO{" "}
qo100-write-converter-lnb-offset-puts = Grava o deslocamento do conversor/LNB que coloca a baliza, onde você clicou, em 10489,750 MHz, e reabre o receptor — a mesma breve interrupção que Configurações ▸ Rádio ▸ Aplicar causa
qo100-write-converter-lnb-offset-puts-2 = Grava o deslocamento do conversor/LNB que coloca a estimativa do rastreador em 10489,750 MHz, e reabre o receptor — a mesma breve interrupção que Configurações ▸ Rádio ▸ Aplicar causa
qo100-waiting-second-crc-valid-frame = Aguardando um segundo quadro com CRC válido antes de oferecer para gravar isto — um travamento sozinho pode ser uma coincidência
qo100-write-corrected-converter-lnb-offset = Grava o deslocamento corrigido do conversor/LNB e reabre o receptor — uma breve interrupção, a mesma que Configurações ▸ Rádio ▸ Aplicar causa
qo100-decoder-runs-receiving-station-readout = o decodificador roda na estação receptora — sua leitura não é enviada a clientes remotos
qo100-starting = iniciando…
qo100-searching-first-window-fills-after = pesquisando — a primeira janela se completa em cerca de 24 s, depois se repete

## js8 (src/app/panels/js8.rs)

js8-multi = MULTI
js8-decode-every-js8-speed-not = Decodifica todas as velocidades do JS8, e não só aquela em que você transmite. As quatro velocidades dividem a sub-faixa e são quatro formas de onda diferentes, então sem isto uma estação em outra velocidade é invisível — e uma troca entre duas velocidades não pode acontecer de jeito nenhum. Custa cerca de quatro vezes a CPU de recepção.
js8-hb-auto = HB AUTO
js8-turbo-does-not-beacon-local = O Turbo não transmite baliza — é a velocidade local e de VHF
js8-beacon-your-callsign-grid-every = Transmite o seu indicativo e locator a cada 15 minutos
js8-until-next-heartbeat = Até o próximo heartbeat
js8-setup = ⚙ CONFIG
js8-tx = ● TX
js8-heard = OUVIDAS
js8-nothing-heard-yet = — nada ouvido ainda —
js8-dxcc = DXCC
js8-band = FAIXA
js8-grid = LOCATOR
js8-dupe = DUPLICADA
js8-reply = RESPOSTA
js8-no-messages = — nenhuma mensagem —
js8-cq = {" "}CQ{" "}
js8-hb = {" "}HB{" "}
js8-clear = {" "}LIMPAR PARA{" "}
js8-forget-selected-station-composer-goes = Esquece a estação selecionada — o compositor volta para @ALLCALL
js8-already-addressing-allcall = Já endereçando @ALLCALL
js8-stop = {" "}PARAR{" "}
js8-send = {" "}ENVIAR{" "}
js8-message = Mensagem…

## logbook (src/app/logbook.rs)

palette-hint = Ir para… um modo, uma faixa, uma janela, uma configuração, uma memória ou uma frequência (14.074)
palette-nothing = nada com esse nome
palette-group-window = Janela
palette-group-settings = Configurações
palette-group-memory = Memória
palette-group-tune = Sintonia
palette-tune-to = Sintonizar em { $mhz } MHz
palette-win-bands = Condições de propagação
palette-win-awards = Prêmios
palette-win-windows = Lista de janelas
check-open = VERIFICAÇÃO DA ESTAÇÃO
    .hint = Uma página que mostra o que está em ordem e o que não está — indicativo, enlace de rádio, receptor, áudio, relógio, consultas — com um botão em cada linha que leva a onde se corrige. Ela lê o que o programa já sabe; não mede nada e não transmite nada.
check-title = VERIFICAÇÃO DA ESTAÇÃO
check-verdict-ok = Nada aqui precisa de atenção.
check-verdict-warn = Funcionando, mas há algo a observar.
check-verdict-fail = Algo não vai funcionar até ser resolvido.
check-fix-settings = AJUSTES
check-fix-unmute = ATIVAR SOM
check-footnote = Lido a partir do que o programa sabe agora. Nada é transmitido e nada é medido de novo; uma linha que diz «não medido» não foi verificada.
check-callsign = Indicativo
check-callsign-missing = Nenhum indicativo definido. Os modos digitais não transmitirão, os contatos são registrados sem o indicativo da estação, e spots e alertas não conseguem saber quais mensagens são para você.
check-callsign-odd = «{ $call }» não parece um indicativo.
check-grid = Quadrado locator
check-grid-missing = Nenhum quadrado locator definido. Distâncias, rumos, os mapas e a troca do próprio FT8 precisam dele.
check-grid-odd = «{ $grid }» não é um locator Maidenhead — quatro ou seis caracteres, como JN48 ou JN48qm.
check-link = Enlace de rádio
check-link-ok = O rádio está conectado e se identificou.
check-link-silent = Nenhum rádio respondeu ainda. Se isso não mudar em alguns segundos, o rádio escolhido em Ajustes não é o que está conectado ou não está ligado.
check-receiver = Receptor
check-receiver-ok = Recebendo: o espectro está chegando.
check-receiver-never = Nenhum espectro chegou desde que o programa foi iniciado.
check-receiver-stalled = Nada chega do receptor há { $seconds } s. O dispositivo parou de transmitir dados — um cabo, uma porta USB que entrou em suspensão ou outro programa que o pegou.
check-level = Nível de entrada
check-level-ok = O conversor não está saturado.
check-level-overload = O conversor do receptor está cortando. Tudo o que ele ouve fica distorcido, sobretudo os sinais fortes — reduza o ganho de RF ou desligue um pré-amplificador.
check-audio = Áudio do rádio
check-audio-ok = Nenhum problema informado com as placas de som do rádio.
check-muted = Alto-falante
check-muted-off = O receptor não está silenciado.
check-muted-on = O receptor está silenciado, por isso não se ouve nada.
check-transmit = Transmissão
check-transmit-ok = Este rádio pode transmitir.
check-transmit-rx-only = Somente recepção: este rádio, ou a forma como está conectado, não tem um transmissor que o programa possa acionar.
check-clock = Relógio do computador
check-clock-unknown = Não medido. Ouça uma frequência FT8 movimentada por um minuto; as estações decodificadas ali mostrarão quanto este relógio está desviado.
check-clock-ok = { $offset } s em relação às estações ouvidas no FT8 — dentro da tolerância.
check-clock-marginal = { $offset } s em relação às estações ouvidas no FT8. Ainda decodificável, mas vale corrigir: no FT4 e no FT2 já custa contatos.
check-clock-bad = { $offset } s em relação às estações ouvidas no FT8. Desviado o suficiente para que outros não decodifiquem você e você decodifique pouco — ajuste o relógio do computador por um servidor de horário.
check-lookup = Consulta de indicativo
check-lookup-ok = Um callbook está configurado.
check-lookup-none = Nenhum callbook configurado, então nomes e locators não são preenchidos para você. Opcional.
check-spots = Fontes de spots
check-spots-ok = { $n } fonte(s) ligada(s).
check-spots-none = Nenhuma fonte de spots ligada, então nada é marcado no panadaptador. Opcional.
welcome-title = BEM-VINDO
welcome-open = GUIA DE BOAS-VINDAS
    .hint = O roteiro da primeira inicialização de novo: o que configurar, o que costuma dar errado e o que há no programa.
welcome-step-welcome = Boas-vindas
welcome-step-station = Estação
welcome-step-radio = Rádio
welcome-step-transmit = Antes de transmitir
welcome-step-online = Online
welcome-step-tour = Roteiro
welcome-step-check = Verificação
welcome-back = ◀ VOLTAR
welcome-next = AVANÇAR ▶
welcome-finish = CONCLUIR
    .hint = Fecha o roteiro e não o mostra mais na inicialização. Ele continua em Ajustes → Geral.
welcome-reopen-note = Ajustes → Geral o traz de volta.
welcome-now = Situação atual
welcome-hello = Bem-vindo ao SDR Oxide
welcome-intro = Um transceptor definido por software: panadaptador e cascata, todos os modos de voz e digitais, do SSB ao FT8, JS8, SSTV e Winlink, um diário, mapas, fontes de spots — para SDRs e para transceptores comuns, aqui ou a partir de outra máquina.
welcome-intro-how = Este roteiro leva alguns minutos. Ele pede as duas coisas sem as quais o programa não funciona, ajuda a conectar o rádio, alerta sobre os problemas em que quem começa costuma esbarrar e mostra onde fica cada coisa. Cada página pode ser pulada e feita depois.
welcome-language-note = Muda o idioma do programa de imediato. Também em Ajustes → Interface.
welcome-intro-safe = Nada disto transmite
    .hint = O roteiro só abre ajustes e janelas. O programa nunca aciona o transmissor até que você pressione algo que diga que o fará.
welcome-station-why = Seu indicativo e seu locator Maidenhead são lidos em toda parte: os modos digitais os colocam em cada mensagem (e não transmitem sem indicativo), o diário os grava em cada contato, os mapas e as distâncias partem do locator, e spots e alertas usam o indicativo para saber quais mensagens são para você.
welcome-station-save-note = SALVAR os guarda; as mesmas caixas estão em Ajustes → Geral.
welcome-station-grid-how = Não sabe o seu locator?
    .hint = Quatro caracteres (IN80) bastam para começar; seis (IN80dk) o colocam a poucos quilômetros. Qualquer mapa online de «locator Maidenhead» o fornece a partir do seu endereço — ou deixe em branco por enquanto e volte depois.
welcome-radio-intro = O que você vai usar? Escolha o mais próximo — a página então lista o que costuma dar errado com esse tipo de rádio e abre o lugar onde ele é configurado.
welcome-radio-pick = Escolha um para ver suas notas de configuração.
welcome-radio-pitfalls = Problemas comuns com este tipo de rádio
welcome-radio-manual = O capítulo 15 do manual traz notas para cada modelo de rádio, e o capítulo 14 é a solução de problemas.
welcome-kind-usb = Um SDR por USB
    .hint = RTL-SDR, Airspy, HackRF, SDRplay RSP, RX-888, LimeSDR, Fobos, ou qualquer um que o SoapySDR controle.
welcome-kind-cat = Um transceptor com cabo CAT
    .hint = Icom, Yaesu, Kenwood, Elecraft, Xiegu, QRP Labs… controlado por uma porta serial/USB, com o áudio em uma placa de som USB.
welcome-kind-net = Um rádio na rede
    .hint = Hermes Lite 2 / ANAN (OpenHPSDR), FlexRadio, Icom por LAN, ADALM-Pluto, SunSDR ou Thetis por TCI, rtl_tcp.
welcome-kind-remote = Outro SDR Oxide, funcionando como servidor
    .hint = O rádio está ligado a outro computador que roda sdroxide --server; este aqui é a tela de controle.
welcome-kind-none = Ainda sem rádio
    .hint = Ouça já por meio de um receptor público na internet (KiwiSDR, Web-888, SpyServer).
welcome-open-radio = AJUSTES DO RÁDIO
    .hint = Escolha a interface, preencha a sua seção e depois APLICAR. As linhas abaixo ficam verdes quando o rádio responde.
welcome-open-remote = AJUSTES REMOTOS
    .hint = O endereço, a porta e o login do servidor.
welcome-open-public = SDRs PÚBLICOS
    .hint = Um mapa e uma lista de receptores na internet; escolha um e ele abre como um rádio.
welcome-tip-usb-driver = O driver USB ou as permissões
    .hint = No Windows, a maioria dos SDRs USB (RTL-SDR, Airspy, HackRF, HydraSDR) precisa do driver WinUSB associado com o Zadig. No Linux, instale a regra udev incluída (o .deb faz isso) e reconecte. No macOS não é preciso nada.
welcome-tip-usb-busy = Outro programa está com o dispositivo
    .hint = Só um programa de cada vez pode abrir um SDR. Feche o SDR#, o SDR++, o GQRX, um rtl_tcp em execução — ou um segundo sdroxide — antes de APLICAR.
welcome-tip-usb-port = Amostras perdidas
    .hint = Taxas de amostragem altas exigem uma porta USB 3 (o RX-888 sempre exige), um cabo curto e nenhum hub sem alimentação. Se o registro informar amostras perdidas, reduza primeiro a taxa de amostragem.
welcome-tip-usb-gain = Ganho demais
    .hint = O ganho de RF no máximo sobrecarrega o conversor e enche a banda de sinais falsos. Comece no meio e recue até que a linha «Nível de entrada» abaixo continue verde.
welcome-tip-usb-sdrplay = O SDRplay precisa do próprio serviço
    .hint = Um RSP só é alcançado pelo serviço SDRplay API v3, que precisa estar instalado e em execução. sdroxide --probe indica qual peça está faltando.
welcome-tip-cat-port = Porta serial ou velocidade erradas
    .hint = Muitos rádios oferecem duas portas em um mesmo cabo USB — escolha a CAT (Enhanced/Standard) — e a taxa em baud precisa corresponder ao menu do próprio rádio.
welcome-tip-cat-family = Família CAT errada
    .hint = Os comandos Kenwood e Yaesu se parecem, mas não são iguais. Um Kenwood configurado como Yaesu pode entrar em transmissão e não parar; um Elecraft configurado como Kenwood passa para SSB comum no FT8.
welcome-tip-cat-audio = Placa de som errada
    .hint = O áudio de recepção e de transmissão passa pelo codec USB do próprio rádio («USB Audio CODEC»), não pelos alto-falantes e microfone do computador. Dois codecs idênticos? A solução de problemas do manual os diferencia.
welcome-tip-cat-data = Os modos digitais precisam do modo DATA
    .hint = Para FT8 e similares, o rádio precisa pegar o áudio de transmissão da USB: USB-D / DATA-U, ou «DATA MOD = USB» no menu. Em USB comum, ele transmite o microfone.
welcome-tip-cat-pipewire = Linux: o servidor de som segura a placa
    .hint = O PipeWire/PulseAudio pode reservar para si a placa de som do rádio, e o programa então mostra «aguardando espectro». A solução de problemas do manual mostra como liberá-la.
welcome-tip-net-subnet = Não estão na mesma rede
    .hint = A descoberta é um broadcast e não atravessa roteador nem VPN. Se o rádio não for encontrado, digite o endereço IP manualmente.
welcome-tip-net-firewall = O controle funciona, mas sem espectro
    .hint = Espectro e áudio trafegam por UDP. Um firewall que deixa a conexão passar mas bloqueia UDP resulta num rádio que responde e não mostra nada — libere o sdroxide no firewall.
welcome-tip-net-icom = Icom por LAN exige configurar o rádio antes
    .hint = No rádio: Network Control ATIVADO, e um nome de usuário de rede e uma senha — o mesmo par vai no sdroxide. Leia o endereço IP na tela Network do rádio; não há descoberta.
welcome-tip-net-one = Um programa controlador de cada vez
    .hint = Um Flex já reivindicado pelo SmartSDR, ou um rádio TCI controlado por outro cliente, pode recusar um segundo. Feche o outro programa ou ative o multiFLEX.
welcome-tip-remote-server = O servidor está configurado no computador do rádio
    .hint = Execute lá o sdroxide --server com um nome de usuário e uma senha configurados; sem eles, qualquer um que chegue à porta pode operar a estação.
welcome-tip-remote-port = A porta precisa estar acessível
    .hint = O padrão é 4950. Pela internet ela precisa de redirecionamento de porta no roteador, ou melhor, uma VPN — e wss:// com certificado se estiver exposta.
welcome-tip-remote-browser = Um navegador também funciona
    .hint = O servidor também entrega a interface completa a um navegador web e a celulares; o áudio ali precisa de https ou localhost.
welcome-tip-none-public = Receptores públicos
    .hint = Milhares de receptores KiwiSDR, Web-888 e SpyServer estão abertos a ouvintes. Só recepção — mas todos os decodificadores, o diário e os mapas funcionam, então é um bom jeito de aprender o programa.
welcome-tip-none-later = Quando o rádio chegar
    .hint = Volte a esta página em Ajustes → Geral → GUIA DE BOAS-VINDAS.
welcome-tx-intro = A maior parte dos «não funciona» no ar é uma destas. Nenhuma aparece como erro, por isso vale a pena lê-las uma vez.
welcome-tip-tx-clock = O relógio do computador
    .hint = FT8, FT4, JS8 e WSPR funcionam em janelas de tempo fixas. Dentro de meio segundo está bom; acima de 1,5 s os outros param de decodificar você e você decodifica pouco — a banda parece morta. Deixe o sistema sincronizar o relógio com um servidor de horário (NTP).
welcome-tip-tx-level = Nível de áudio e ALC
    .hint = Nos modos digitais, mantenha o ALC do rádio em zero ou perto disso: áudio demais espalha o sinal sobre outras estações enquanto o medidor de potência parece normal. Abaixe o nível até que o ALC quase não se mova.
welcome-tip-tx-drive = Comece com pouca excitação
    .hint = A excitação é uma porcentagem do máximo do rádio. Comece baixo, em carga fantasma ou com antena casada, e suba observando potência e SWR.
welcome-tip-tx-lockout = O bloqueio de banda
    .hint = O transmissor só transmite dentro das faixas de radioamador da sua região IARU. Defina a região em Ajustes → Geral, ou uma frequência perfeitamente legal pode ser recusada.
welcome-tip-tx-trswitch = Um SDR ao lado de um transceptor
    .hint = Um receptor SDR que divide a antena com um transmissor será destruído na primeira transmissão, a menos que algo o desconecte. Instale a chave T/R antes de acionar qualquer coisa.
welcome-tip-tx-cable = Cabos de áudio virtuais
    .hint = Com um cabo virtual ou Flex DAX, defina 48000 Hz nas duas pontas. Uma taxa diferente descarta amostras, e os modos temporizados então não decodificam nada enquanto a cascata parece perfeita.
welcome-open-region = REGIÃO IARU
    .hint = Ajustes → Geral: a região define quais frequências o transmissor pode usar.
welcome-open-trswitch = CHAVE T/R
    .hint = O relé que protege um SDR somente de recepção enquanto a estação transmite.
welcome-online-intro = Tudo opcional, e tudo desligado até que você ative. Cada um torna o programa mais útil:
welcome-online-lookup = CONSULTA DE INDICATIVO
    .hint = QRZ.com ou HamQTH preenchem nomes, localizações e locators conforme você trabalha as estações.
welcome-online-spots = FONTES DE SPOTS
    .hint = Cluster DX, POTA, SOTA, PSK Reporter — os spots aparecem no panadaptador; clique em um para sintonizar e preencher o diário.
welcome-online-uploads = ENVIOS DE LOG
    .hint = LoTW, eQSL, Club Log, QRZ e HamQTH, com um clique a partir do diário, para confirmações e premiações.
welcome-online-alerts = ALERTAS
    .hint = Um som ou notificação quando um indicativo, entidade DXCC ou quadrado procurado é ouvido.
welcome-online-privacy = Nada é enviado a lugar nenhum até que um serviço seja configurado aqui; os downloads de condições de propagação e de horários de transmissão têm chaves próprias nos Ajustes.
welcome-tour-intro = Os modos, o diário e os spots estão na tela principal. Estes são fáceis de deixar passar. Cada ABRIR leva direto para lá, e o roteiro fica aberto ao lado.
welcome-tour-open = ABRIR
welcome-tour-bands = Condições de propagação
    .hint = Dados solares e quais bandas estão abertas agora.
welcome-tour-winlink = Correio Winlink
    .hint = E-mail por rádio pela internet, pacote ou HF — integrado, sem Pat nem modem externo.
welcome-tour-public = SDRs públicos
    .hint = Ouça por receptores em qualquer lugar do mundo.
welcome-tour-servers = WSJT-X e outros programas
    .hint = Os servidores TCI e rigctld integrados deixam o WSJT-X, o JTDX, programas de diário e outros usarem este rádio.
welcome-tour-controls = Teclas e controladores
    .hint = Cada atalho pode ser redefinido; controladores MIDI e HID e botões de sintonia também funcionam.
welcome-tour-profiles = Perfis
    .hint = Salve como a estação está configurada — concurso, portátil, satélite — e restaure com um clique.
welcome-tour-palette = Paleta de comandos
    .hint = Ctrl+K (⌘K): digite um modo, uma banda, uma janela, um ajuste ou uma frequência e pressione Enter.
welcome-tour-help = Ajuda
    .hint = Os atalhos de teclado e os controles na tela.
welcome-check-all-good = Nada precisa de atenção. Bons contatos no ar!
welcome-check-left = Ainda para ver — cada um tem um botão que leva a onde se corrige:
welcome-check-hint = A página completa, com cada linha. Também em Ajustes → Geral.
welcome-next-steps = Para onde ir agora
welcome-next-listen = Primeiro ouça
    .hint = Sintonize pela banda, experimente os botões de banda, arraste as bordas da passagem na cascata. Nada transmite até que você pressione PTT, TUNE ou o botão de chamada do próprio modo.
welcome-next-ft8 = Um primeiro contato no FT8
    .hint = Escolha FT8, ouça um minuto (isso também mede o seu relógio), depois pressione RESPONDER numa estação que chama CQ. A troca acontece sozinha e o contato é registrado ao terminar.
welcome-next-palette = Perdido? Pressione Ctrl+K
    .hint = A paleta de comandos encontra qualquer modo, banda, janela ou ajuste pelo nome.
welcome-next-manual = O manual do usuário
    .hint = docs/USER_MANUAL.md, com um capítulo para cada modo e notas para cada rádio. Passe o cursor sobre qualquer controle para ver a explicação também.
logbook-logbook = DIÁRIO
logbook-new-entry = + NOVO REGISTRO
logbook-adif = ADIF
logbook-import = IMPORTAR
logbook-import-qsos-adif-adi-adif = Importa QSOs de um arquivo ADIF (.adi ou .adif)
logbook-new-qso = NOVO QSO
logbook-edit-qso = EDITAR QSO
logbook-call = Indicativo
logbook-lookup = CONSULTAR
logbook-look-up-name-qth-grid = Consulta nome / QTH / locator
logbook-grid = Locator
logbook-freq-mhz = Freq MHz
logbook-mode = Modo
logbook-rst-sent = RST enviado
logbook-rst-rcvd = RST recebido
logbook-name = Nome
logbook-state = Estado
logbook-country = País
logbook-date-utc = Data UTC
logbook-time = Hora
logbook-pwr-w = Pot W
logbook-contest = Concurso
logbook-s-sent = Nº enviado
logbook-s-rcvd = Nº recebido
logbook-comment = Comentário
logbook-save = {" "}SALVAR{" "}
logbook-cancel = CANCELAR
logbook-lotw = LoTW ✓
logbook-eqsl = eQSL ✓
logbook-card = cartão ✓
logbook-lotw-2 = LoTW ↑
logbook-eqsl-2 = eQSL ↑
logbook-qrz = QRZ ↑
logbook-hamqth = HamQTH ↑
logbook-club-log = Club Log ↑
logbook-wavelog = Wavelog ↑
logbook-delete-entry = Excluir este registro
logbook-edit = EDITAR
logbook-upload-qso-configured-logs = Envia este QSO para os logs configurados
logbook-no-qsos-yet-run-ft8 = nenhum QSO ainda — rode FT8/FT4 ou adicione um registro manual

## scanner (src/app/scanner.rs)

scanner-scanner = Scanner
scanner-work-through-stored-memory-channels = Percorre os canais de memória guardados
scanner-work-through-slice-band-channel = Percorre uma fatia de uma faixa em uma grade de canais
scanner-sweep-range-into-panorama = Percorrer uma faixa sem parar para formar um panorama
scanner-panorama-hint = Percorre sem parar; cada varredura completa é uma linha da faixa de banda inteira (WIDE).
scanner-stop = PARAR
scanner-start = INICIAR
scanner-band = Banda
scanner-from = De
scanner-step = Passo
scanner-mode = Modo
scanner-range-empty-high-edge-has = Esta faixa está vazia — a borda superior precisa estar pelo menos um canal acima da inferior.
scanner-skip-while-holding-pass-over = PULAR enquanto estiver mantido para passar por cima deste canal daqui em diante
scanner-skipping = Pulando
scanner-clear = LIMPAR
scanner-stop-passing-over-any-them = Parar de passar por cima de qualquer um deles
scanner-stop-passing-over-channel = Parar de passar por cima deste canal
scanner-no-memory-channels-stored-yet = Nenhum canal de memória guardado ainda — guarde alguns na janela MEM.
scanner-skip-channel-pass-over = PULAR um canal para passar por cima dele
scanner-skip = PULAR
scanner-look-all-channels-fall-inside = Procura todos os canais que caem dentro de uma janela de receptor, na mesma transformada de que o panadaptador é feito, e só sintoniza naqueles em que algo está. Uma lista em uma faixa passa então a custar uma sintonia por volta, não importa o tamanho, em vez de um tempo de acomodação por canal. A varredura ainda escuta cada candidato antes de parar, então o que a interrompe não muda — mas a varredura mede pela FFT, e não pelo filtro do receptor, então confira o limiar se ele começar a parar em nada.
scanner-radio-hands-over-demodulated-audio = Este rádio entrega áudio demodulado e não tem espectro próprio para pesquisar, então a varredura de memórias visita cada canal de qualquer forma.
scanner-fast = RÁPIDO
scanner-read-list-off-spectrum-instead = lê a lista a partir do espectro, em vez de visitar cada canal
scanner-folders = Pastas
scanner-scan-every-folder-every-folder = Varre toda pasta, e toda pasta criada daqui em diante. Escolha pastas para varrer só essas.
scanner-unfiled = Sem pasta
scanner-nothing-filed-under-folders-you = Nada está arquivado nas pastas que você escolheu.
scanner-stops = Para em
scanner-use-receiver-s-own-squelch = Usa o squelch próprio do receptor, para que a varredura pare exatamente onde o áudio abriria
scanner-stop-snr = Para onde um canal fica isto acima do ruído de fundo ao redor; acompanha ganho e ruído da banda.
scanner-squelch-off-scan-will-stop = o squelch está desligado — a varredura parará no primeiro sinal que examinar
scanner-channel-power-signal-has-reach = Potência do canal que um sinal precisa alcançar para parar a varredura
scanner-listens = Escuta por
scanner-settles = Espera por trecho
scanner-settles-hint = Espera após ressintonizar, antes de ler o espectro. Mais longa: imagem mais estável, varredura mais lenta.
scanner-pano-width = Largura do trecho
scanner-pano-width-hint = Parte da largura do receptor usada por trecho. Menos: longe da queda nas bordas; mais: varredura mais rápida. Sobreposição de 10 %.
scanner-how-long-stay-candidate-before = Quanto tempo permanecer em um candidato antes de julgá-lo. Abaixo de cerca de um décimo de segundo o medidor de nível não se estabiliza e sinais fracos são perdidos
scanner-resumes = Retoma
scanner-carry-once-signal-drops = Continua assim que o sinal cai
scanner-carry-after-fixed-time-regardless = Continua após um tempo fixo, seja como for
scanner-stay-until-you-press-next = Fica até você pressionar PRÓXIMO
scanner-how-long-stay = Quanto tempo permanecer
scanner-how-long-wait-after-signal = Quanto tempo esperar depois que o sinal cai
scanner-move-don-t-stop-channel = Segue adiante, e não para de novo neste canal

## adsb (src/app/panels/adsb.rs)

adsb-starting-ads-b-decoder = iniciando o decodificador ADS-B…
adsb-ads-b = ADS-B
adsb-t-1090-mhz-mode-s = Modo S de 1090 MHz
adsb-tune-receiver-1090-000-mhz = Sintoniza o receptor em 1090,000 MHz, o enlace de descida ADS-B mundial. Há apenas um canal.
adsb-setup = CONFIG
adsb-timeouts-trail-length-how-far = Tempos limite, comprimento do rastro e até onde os vetores de velocidade alcançam
adsb-aircraft = AERONAVES
adsb-nothing-heard-yet = nada ouvido ainda
adsb-ground = em solo
adsb-position-stale = posição desatualizada
adsb-gnss-altitude = Altitude GNSS
adsb-ground-speed = velocidade no solo
adsb-vertical-rate = razão vertical
adsb-first-heard = ouvido pela primeira vez
adsb-center = CENTRALIZAR
adsb-put-aircraft-middle-map = Coloca esta aeronave no meio do mapa
adsb-last-frame-accepted-aircraft = O último quadro aceito desta aeronave
adsb-ads-b-setup = Configuração ADS-B
adsb-drop-map-after = Remover do mapa após
adsb-without-position-report = sem relatório de posição
adsb-past-aircraft-comes-off-map = Depois disto, a aeronave sai do mapa e a linha dela fica cinza. Não é esmaecida: um quadrado apagado em uma posição antiga ainda é uma afirmação sobre onde um avião está, na mesma tinta que as verdadeiras.
adsb-drop-list-after = Remover da lista após
adsb-nothing-heard-all = sem nada ouvido
adsb-trail-length = Comprimento do rastro
adsb-points = {" "}pontos
adsb-history-dots-behind-each-target = pontos de histórico atrás de cada alvo
adsb-speed-vector = Vetor de velocidade
adsb-how-far-ahead-leader-line = até onde a linha guia alcança à frente
adsb-one-minute-usual-radar-convention = Um minuto é a convenção usual de radar: a linha tem exatamente o comprimento da distância que a aeronave percorre nesse tempo, então duas guias iguais são duas velocidades iguais em qualquer zoom. Zero as desliga.
adsb-track-most = Rastrear no máximo
adsb-aircraft-2 = {" "}aeronaves
adsb-aircraft-ground-are-placed-against = Aeronaves no solo são posicionadas em relação à própria posição da estação — um squitter de superfície não tem decodificação inequívoca por conta própria — então preencha Meu locator na configuração de modos digitais se o aeroporto próximo não mostrar nada.
adsb-call = INDICATIVO
adsb-icao = ICAO
adsb-v-s = V/S

## drm (src/app/drm.rs)

drm-waiting-receiver = aguardando o receptor…
drm-not-drm-set-mode-drm = Não está em DRM. Defina o modo como DRM em uma transmissão digital de ondas curtas — o dial fica no centro do canal, não em uma banda lateral.
drm-no-drm-signal-locked-decoder = Nenhum sinal DRM travado. O decodificador precisa de alguns segundos em uma portadora limpa: o DRM entrelaça ao longo de 400 ms ou 2 s antes que qualquer parte possa ser lida.
drm-station-s-audio-codec-not = O codec de áudio desta estação não decodifica aqui. O xHE-AAC precisa do libfdk-aac no sistema — não pode ser embutido, porque a licença dele e a deste programa são incompatíveis. Instale-o (Debian/Ubuntu: libfdk-aac2, Arch: libfdk-aac, macOS: brew install fdk-aac, Windows: libfdk-aac-2.dll ao lado do sdroxide.exe) e reinicie. Todo o resto desta tela está decodificando normalmente.
drm-quality = QUALIDADE
drm-last-minute = último minuto
drm-waiting-locked-signal = aguardando um sinal travado
drm-audio-frames-decoding-but-station = Quadros de áudio decodificando — mas o codec desta estação não pode ser decodificado aqui
drm-audio-frames-decoding = Quadros de áudio decodificando
drm-samples-reaching-decoder = Amostras chegando ao decodificador
drm-time = HORA
drm-symbol-timing-recovered = Temporização de símbolo recuperada
drm-frame = QUADRO
drm-transmission-frames-found = Quadros de transmissão encontrados
drm-fast-access-channel-what-transmission = Canal de Acesso Rápido — o que é a transmissão
drm-service-description-channel-what-services = Canal de Descrição de Serviços — quais são os serviços
drm-audio = ÁUDIO
drm-constellation = CONSTELAÇÃO
drm-waiting-decoded-symbols = aguardando símbolos decodificados
drm-sampled-across-frame = amostrada ao longo do quadro
drm-whole-frame = quadro inteiro
drm-mode = MODO
drm-robustness-mode-channel-width-ground = Modo de robustez e largura de canal. A é para um percurso de onda de superfície e carrega mais; D é para um percurso de onda celeste muito espalhado e carrega menos.
drm-interleave = ENTRELAÇAMENTO
drm-how-far-transmission-spreads-each = Quanto a transmissão espalha cada quadro no tempo. Longo aguenta desvanecimentos mais profundos e leva proporcionalmente mais tempo para adquirir.
drm-protection = PROTEÇÃO
drm-offset = DESLOCAMENTO
drm-residual-sample-clock-error-against = Erro residual do clock de amostragem em relação ao transmissor. Grande e constante significa que a referência do receptor está fora, e não a transmissão.
drm-doppler = DOPPLER
drm-delay = ATRASO
drm-doppler-delay-spread-path-how = Espalhamento Doppler e de atraso do percurso — quão depressa ele se move e quão distantes chegam os seus ecos.
drm-service = SERVIÇO
drm-decode-service-multiplex = Decodifica este serviço do multiplex

## atchat (src/app/panels/atchat.rs)

atchat-starting-atchat-net-station = iniciando a estação AtCHAT NET…
atchat-atchat = ATCHAT
atchat-net-multi-station-keyboard-file = NET — teclado e arquivo multiestação
atchat-net = {" "}NA NET{" "}
atchat-net-click-leave-state-kept = Na net — clique para sair (o estado é mantido; clique de novo para voltar)
atchat-rejoin = {" "}VOLTAR{" "}
atchat-off-net-click-rejoin = Fora da net — clique para voltar
atchat-off-net = fora da net
atchat-carrier = ● PORTADORA
atchat-another-station-transmitting-channel-busy = Outra estação está transmitindo — o canal está ocupado.
atchat-tx = ● TX
atchat-setup = ⚙ CONFIG
atchat-station-callsign-virtual-channel-address = Indicativo da estação e o endereço do canal virtual
atchat-virtual-channel = CANAL VIRTUAL
atchat-work-net-over-tcp-endpoint = Trabalha a net por um endpoint TCP em vez do rádio — um loopback compatível com channel_server para desenvolvimento sem rádio. Desligado coloca a estação no ar.
atchat-roster = LISTA
atchat-nobody-heard-yet = ninguém ouvido ainda
atchat-chat = {" "}CHAT{" "}
atchat-common-channel-lines-here-go = O canal comum — as linhas daqui vão para todos (ALL)
atchat-log = {" "}LOG{" "}
atchat-station-s-own-air-activity = A atividade no ar da própria estação — eleição de mestre, envelhecimento da lista, retentativas ARQ
atchat-no-activity-logged-yet = Nenhuma atividade registrada ainda.
atchat-no-messages-yet = Nenhuma mensagem ainda.
atchat-message-everyone-all = mensagem para todos (ALL)
atchat-send = {" "}ENVIAR{" "}
atchat-transfers = TRANSFERÊNCIAS
atchat-all = → ALL
atchat-sent-file-follows-active-chat = Um arquivo enviado segue a aba de chat ativa
atchat-send-file = {" "}ENVIAR ARQUIVO{" "}
atchat-send-file-image-over-air = Envia um arquivo ou imagem pelo ar, com bloco-CRC-ARQ
atchat-nothing-flight = nada em trânsito
atchat-images = IMAGENS
atchat-no-images-received = Nenhuma imagem recebida.


## rds (src/app/rds.rs)

rds-station = ESTAÇÃO
rds-diagnostics = DIAGNÓSTICO
rds-which-programme-type-table-read = Qual tabela de tipo de programa usar para ler esta estação. O RDS é usado em todo lugar fora da América do Norte; o RBDS acrescenta o indicativo que o código de identificação de uma estação dos EUA soletra. O Automático segue o código de país estendido que a estação envia, e até que um chegue não soletra indicativo algum — as faixas de identificação que ele leria são compartilhadas com os códigos RDS de nove países, então selecione RBDS para nomear uma estação que não envia código de país.
rds-waiting-receiver = aguardando o receptor…
rds-no-rds-station-not-every = Nenhum RDS nesta estação. Nem todo transmissor o transmite, e um fraco pode transmiti-lo sem que ele chegue.
rds-rds-only-carried-wfm-broadcast = O RDS só é transmitido por estações de radiodifusão WFM.
rds-holding = RETIDO
rds-block-sync-has-dropped-last = A sincronia de blocos caiu — esta é a última estação lida.
rds-identity = IDENTIDADE
rds-programme = PROGRAMA
rds-described = DESCRITO COMO
rds-content = CONTEÚDO
rds-music = Música
rds-speech = Fala
rds-traffic = TRÂNSITO
rds-announcement-air-now = anúncio no ar agora
rds-carried-station = transmitido por esta estação
rds-not-carried = não transmitido
rds-radio-text = TEXTO DE RÁDIO
rds-station-clock = RELÓGIO DA ESTAÇÃO
rds-country-code = CÓDIGO DE PAÍS
rds-also = TAMBÉM EM
rds-also-tagged = TAMBÉM MARCADO
rds-sync = SINC
rds-no-sync = SEM SINC
rds-blocks-failed-their-check-plus = Blocos que falharam na verificação, mais os que o corretor de erros teve de reparar. As correções contam como erros de propósito: um número subindo é o primeiro sinal de que uma estação está prestes a parar de decodificar.
rds-front-end-overloading = FRONT END SOBRECARREGADO
rds-converter-clipping-which-puts-distortion = O conversor está cortando, o que espalha distorção por todo o multiplex — inclusive na subportadora de dados de 57 kHz. Reduza o ganho de RF, ou insira um atenuador: o RDS sai do ar cerca de um decibel antes de qualquer coisa ficar audível, então uma estação que soa perfeita pode ainda estar forte demais para que seus dados sobrevivam.
rds-group-types = TIPOS DE GRUPO
rds-recent-groups-newest-last = GRUPOS RECENTES — mais novos por último
rds-nothing-decoded-yet = nada decodificado ainda

## winlink (src/app/winlink.rs)

winlink-mail = E-MAIL
winlink-abort = ABORTAR
winlink-stop-session-progress = Interrompe a sessão em andamento
winlink-connect = CONECTAR
winlink-compose = ESCREVER
winlink-connecting = conectando…
winlink-not-connected-yet = ainda não conectado
winlink-loading = carregando…
winlink-date = DATA
winlink-from = DE
winlink-subject = ASSUNTO
winlink-no-subject = (sem assunto)
winlink-delete-message = Excluir esta mensagem
winlink-arch = ARQ
winlink-move-archive = Mover para o arquivo
winlink-select-message = selecione uma mensagem
winlink-reply = RESPONDER
winlink-delete = EXCLUIR
winlink-to = Para
winlink-cc = Cc
winlink-subject-2 = Assunto
winlink-callsign-smtp-someone-example-org = Um indicativo, ou SMTP:alguem@exemplo.org para e-mail da internet. Separe vários com vírgulas.
winlink-file-outbox = ARQUIVAR NA CAIXA DE SAÍDA
winlink-discard = DESCARTAR
winlink-filed-messages-go-out-next = as mensagens arquivadas saem na próxima sessão
winlink-mail-log = LOG DE E-MAIL
winlink-session-running = sessão em andamento…
winlink-no-session-yet = nenhuma sessão ainda

## wspr (src/app/panels/wspr.rs)

wspr-receptions = RECEPÇÕES
wspr-save-wspr-reception-list-csv = Salvar a lista de recepções WSPR como CSV
wspr-wspr = WSPR
wspr-weak-signal-propagation-beacon = baliza de propagação de sinal fraco
wspr-time-left-two-minute-slot = Tempo restante neste slot de dois minutos. No fim dele, a gravação vai para o decodificador e a próxima decisão de transmissão é tomada.
wspr-tx = ● TX
wspr-decoding = decodificando…
wspr-band = Faixa
wspr-last-slot = Último slot
wspr-nothing-heard = nada ouvido
wspr-receive-only = somente recepção
wspr-next-slot = Próximo slot
wspr-waiting-radio-take-change = aguardando o rádio aceitar a mudança…
wspr-hop-next = Saltar em seguida
wspr-transmit = TRANSMITIR
wspr-receive-only-2 = Somente recepção.
wspr-power = POTÊNCIA
wspr-what-you-actually-radiate = o que você de fato irradia
wspr-goes-out-message-everyone-who = Isto sai na mensagem, e todos que o ouvem julgam o percurso por ela — então um número otimista aqui torna as medições deles erradas, além das suas.
wspr-roam = VARIAR
wspr-pick-different-offset-inside-200 = Escolhe um deslocamento diferente dentro da janela de 200 Hz a cada transmissão. Esta é a convenção: duzentos hertz compartilhados por todos só funcionam se ninguém se estacionar no meio. Desligado mantém onde está o cursor.
wspr-band-hop = SALTO DE FAIXA
wspr-move-dial-band-band-between = Move o dial de faixa em faixa entre os slots, para que um receptor amostre o espectro inteiro em vez de uma fatia dele. Girar o VFO você mesmo a pausa — pressione isto duas vezes para retomar.
wspr-upload = ENVIAR
wspr-send-what-station-decodes-wsprnet = Envia o que esta estação decodifica para o wsprnet.org. Informar o que você ouve é o que torna um receptor WSPR parte da rede; não coloca nada no ar.
wspr-who-heard-me = QUEM ME OUVIU
wspr-ask-wsprnet-org-every-few = Pergunta ao wsprnet.org a cada poucos minutos quais estações decodificaram esta. O WSPR não tem nenhuma forma de confirmação, então esta é a única maneira de uma baliza aprender algo sobre o próprio alcance.
wspr-bands = FAIXAS
wspr-none-selected-hopping-has-nowhere = nenhuma selecionada — o salto não tem para onde ir
wspr-set-your-callsign-grid-general = Defina o seu indicativo e locator na aba Geral das Configurações antes de transmitir.

## packet (src/app/panels/packet.rs)

packet-monitor = MONITOR
packet-busy = OCUPADO
packet-frames-arrived-but-failed-their = Quadros que chegaram mas falharam na verificação de sequência — uma colisão, um desvanecimento, ou um sinal fraco demais para ser lido.
packet-setup = ⚙ CONFIG
packet-station-callsign-speed-tx-delay = Indicativo da estação, velocidade, atraso de TX, o caminho de digipeaters, a baliza e o servidor KISS
packet-starting-packet-modem = iniciando o modem de pacote…
packet-nothing-heard-yet = nada ouvido ainda
packet-terminal = TERMINAL
packet-connected = {" "}CONECTADO{" "}
packet-calling = {" "}CHAMANDO{" "}
packet-idle = {" "}OCIOSO{" "}
packet-extended-sequence-numbers-window-up = Números de sequência estendidos — uma janela de até 127 quadros.
packet-frames-sent-not-yet-acknowledged = Quadros enviados e ainda não confirmados pela outra ponta.
packet-retries-against-link-s-limit = Retentativas contra o limite deste enlace. Uma contagem subindo enquanto os quadros não confirmados ficam parados é o que um percurso com desvanecimento parece deste lado.
packet-mail-window-using-link = A janela E-MAIL está usando o enlace.
packet-one-radio-one-channel-one = Um rádio, um canal, um enlace. Uma sessão Winlink e este terminal são duas formas de usá-lo, e quem pedir primeiro fica com ele.
packet-digipeaters-nearest-first-separated-commas = Digipeaters, o mais próximo primeiro, separados por vírgulas — OE3XLR-1,OE3XMS-1. Deixe vazio para uma estação que você ouve diretamente, ou para usar o caminho padrão das configurações de pacote.
packet-disconnect = {" "}DESCONECTAR{" "}
packet-hang-up-properly-wait-far = Desligue corretamente, e espere a outra ponta concordar. Trocar de modo, em vez disso, encerra a sessão sem avisar ninguém.
packet-connect = {" "}CONECTAR{" "}
packet-mail-window-has-link-finish = A janela E-MAIL tem o enlace. Termine ou interrompa essa sessão primeiro.
packet-station-has-no-callsign-yet = Esta estação ainda não tem indicativo — defina um em CONFIG, acima do painel do monitor. Nada transmite até que seja definido.
packet-call-station-connected-mode-node = Chama esta estação em modo conectado — um nó, um BBS ou outro operador.
packet-needs-callsign-call = Precisa de um indicativo para chamar.
packet-clear = {" "}LIMPAR{" "}
packet-empty-transcript-link-untouched-connected = Esvazia a transcrição. O enlace não é afetado — uma estação conectada continua conectada.
packet-not-connected-type-callsign-above = Não conectado. Digite um indicativo acima e pressione CONECTAR para chamar um nó ou um BBS.
packet-type-here = digite aqui
packet-not-connected = não conectado
packet-send = {" "}ENVIAR{" "}
packet-send-line-node-bbs-reads = Envia a linha. Um nó ou um BBS lê uma linha por vez.

## fsq (src/app/panels/fsq.rs)

fsq-speed = Velocidade
fsq-call = Chamada
fsq-callsign-directed-fsqcall-messages-defaults = Indicativo para mensagens dirigidas (FSQCALL); usa o seu indicativo por padrão
fsq-tx = ● TX
fsq-contacts = CONTATOS
fsq-heard = OUVIDOS
fsq-none = — nenhum —
fsq-allcall = ALLCALL
fsq-image = IMAGEM
fsq-clear = LIMPAR
fsq-forget-pictures-received-so-far = Esquece as imagens recebidas até agora
fsq-send-image = Enviar imagem…
fsq-right-click-remove = Clique com o botão direito para remover
fsq-remove-picture = Remover esta imagem
fsq-listening = — escutando —
fsq-message = Mensagem…
fsq-send = {" "}ENVIAR{" "}
fsq-call-cq = {" "}CHAMAR CQ{" "}
fsq-heard-2 = {" "}? ouvido{" "}
fsq-clear-2 = {" "}LIMPAR{" "}
fsq-fsq-contacts = Contatos FSQ
fsq-add = Adicionar

## publicsdr (src/app/publicsdr.rs)

publicsdr-not-stated = não informado
publicsdr-public-sdrs = SDRS PÚBLICOS
publicsdr-available = DISPONÍVEL
publicsdr-hide-receivers-are-full-ones = Oculta os receptores que estão lotados, e os cujo operador não abriu canais para aplicativos além de um navegador
publicsdr-band = NA FAIXA
publicsdr-only-receivers-cover-current-dial = Somente receptores que cobrem a frequência atual do dial
publicsdr-low-bw = BAIXA LARG
publicsdr-map-cities = CIDADES
publicsdr-map-cities-tip = Cidades neste mapa, sim ou não.
publicsdr-view-list = LISTA
publicsdr-view-map = MAPA
publicsdr-view-map-tip = Os mesmos receptores em um mapa-múndi. Clique em um ponto para escolher todos os receptores sob ele.
publicsdr-map-pick = Clique em um ponto para escolher os receptores sob ele.
publicsdr-map-ghost = { $name }: fora do diretório agora — visto aqui pela última vez em { $seen }
publicsdr-map-names = NOMES
publicsdr-map-names-tip = Mostrar os nomes dos receptores no mapa. Desligado: só os escolhidos são nomeados.
publicsdr-check-online = ON-LINE
publicsdr-check-silent = MUDO
publicsdr-check-waiting = TESTANDO…
publicsdr-check-no-answer = Sem resposta em 20 segundos.
publicsdr-take-spyserver-low-bandwidth-shape = Usa um SpyServer na sua forma de baixa largura de banda: uma janela I/Q estreita que segue o dial, mais a visão de banda do próprio servidor, em vez de megabits de I/Q de banda larga. Sem efeito em um KiwiSDR, que só tem essa forma.
publicsdr-refresh = ⟳ ATUALIZAR
publicsdr-fetch-both-directories-again = Busca os dois diretórios de novo
publicsdr-name-place-antenna-band = nome, local, antena, faixa
publicsdr-clear-search = Limpar a busca
publicsdr-fetching-receiver-lists = Buscando as listas de receptores…
publicsdr-machine-radio-attached-does-not = A máquina à qual este rádio está conectado não responde a perguntas sobre dispositivos, então também não consegue buscar as listas.
publicsdr-no-match = nenhum resultado
publicsdr-nothing-matches-try-turning-filter = nada corresponde — tente reativar um dos filtros

## hd (src/app/hd.rs)

hd-hd-radio = HD Radio
hd-waiting-receiver = aguardando o receptor…
hd-not-hd-radio-set-mode = Não está em HD Radio. Defina o modo como HD Radio em uma portadora de radiodifusão FM que traga os sidecars digitais — o dial fica no centro da portadora analógica, e não ao lado dela.
hd-no-hd-radio-lock-digital = Nenhum travamento de HD Radio. As bandas laterais digitais são transmitidas cerca de 20 dB abaixo da portadora analógica, então uma estação audível ainda pode ser ruidosa demais para decodificar: o decodificador precisa de alguns segundos em uma limpa.
hd-sync = SINC
hd-ofdm-frame-timing-recovered = Temporização de quadro OFDM recuperada
hd-audio = ÁUDIO
hd-audio-frames-decoding = Quadros de áudio decodificando
hd-quality = QUALIDADE
hd-lower = INFERIOR
hd-upper = SUPERIOR
hd-last-minute = último minuto
hd-waiting-locked-signal = aguardando um sinal travado
hd-mer-l = MER I
hd-modulation-error-ratio-lower-digital = Razão de erro de modulação da banda lateral digital inferior.
hd-mer-u = MER S
hd-modulation-error-ratio-upper-digital = Razão de erro de modulação da banda lateral digital superior.
hd-cber = CBER
hd-channel-bit-error-ratio-after = Razão de erro de bit do canal depois do código interno. Abaixo de cerca de 1e-3 o código externo consegue corrigi-la; acima disso o áudio começa a falhar.
hd-offset = DESLOCAMENTO
hd-residual-carrier-frequency-offset-large = Deslocamento residual da frequência da portadora. Grande e constante significa que a referência do receptor está fora, e não a transmissão.
hd-psmi = PSMI
hd-primary-service-mode-indicator-which = Indicador de Modo de Serviço Primário: qual dos modos híbridos de serviço FM a estação está transmitindo.
hd-programme = PROGRAMA
hd-restricted-access = {" "}— acesso restrito

## bands (src/app/bands.rs)

bands-bands = FAIXAS
bands-daylight = ☀ DIA
bands-night = ☾ NOITE
bands-no-forecast-yet-fetching-hamqsl = · sem previsão ainda — buscando no HAMQSL.com
bands-forecast-download-off = · download da previsão desligado (Configurações → UI)
bands-band = FAIXA
bands-condx = CONDX
bands-paths = PERCURSOS
bands-reach = ALCANCE
bands-best = MELHOR
bands-condx-global-forecast-rest-what = CONDX é uma previsão global; o resto é o que de fato foi ouvido. Uma linha vazia significa que nada foi decodificado — o que pode significar que a faixa estava fechada, ou apenas que ninguém estava nela.
bands-ibp-beacons = BALIZAS IBP
bands-meteor-showers = CHUVAS DE METEORO
bands-peak = PICO

## spots (src/app/spots.rs)

spots-pota = POTA
spots-sota = SOTA
spots-freedv = FREEDV
spots-spots = SPOTS
spots-longwave-shortwave-broadcast-stations-air = Estações de radiodifusão de ondas longas e curtas no ar agora
spots-view = NA TELA
spots-only-spots-inside-panadapter-span = Somente spots dentro da largura do panadaptador
spots-setup = ⚙ CONFIG
spots-feeds-lookup-upload-settings = Feeds, consulta e configurações de envio
spots-call-station-site-frequency = indicativo, estação, local, frequência
spots-clear-search = Limpar a busca
spots-no-match = nenhum resultado
spots-no-spots-enable-feed-setup = nenhum spot — ative um feed em ⚙ CONFIG
spots-nothing-matches-search = nada corresponde à busca

## ism (src/app/ism.rs)

ism-ism-devices = DISPOSITIVOS ISM
ism-decoding = DECODIFICANDO
ism-needs-wideband-iq-source-cat = Precisa de uma fonte I/Q de banda larga; um equipamento CAT envia somente áudio
ism-how-far-burst-must-stand = Quanto uma rajada precisa se sobressair acima do piso de ruído próprio do canal antes de ser decodificada
ism-give-each-band-width-normally = Dê a cada faixa a largura que ela normalmente precisa — um quarto de megahertz para as faixas OOK em 315, 345 e 433 MHz, uma inteira para 868 e 915.
ism-several-hundred-more-device-decoders = Várias centenas de decodificadores de dispositivos a mais, do projeto rtl_433, embutidos. Lê os dispositivos OOK de 433 MHz que os decodificadores próprios do sdroxide não alcançam de jeito nenhum. Onde os dois podem ler um dispositivo, o rtl_433 conhece muito mais variantes e assume.
ism-t-1-your-own = 1 seu
ism-device-decoders-registered-how-many = Decodificadores de dispositivos registrados, quantos vieram do seu rtl433_flex.conf, e quantas transmissões foram decodificadas desde que começou.
ism-reload-decoders = RECARREGAR DECODIFICADORES
ism-re-read-rtl433-flex-conf = Relê o rtl433_flex.conf depois de editá-lo. Os dispositivos já ouvidos continuam na lista.
ism-rtl-433-knows-more-variants = O rtl_433 conhece mais variantes destes do que os decodificadores próprios do sdroxide, então as lê enquanto escuta aqui. Escolha outra faixa e os próprios do sdroxide voltam a assumir.
ism-decoder-your-rtl433-flex-conf = Um decodificador no seu rtl433_flex.conf foi recusado. Os outros foram carregados. Corrija a linha e pressione RECARREGAR DECODIFICADORES.
ism-waiting-decoder = aguardando o decodificador…
ism-where-decoder-actually-listening-follows = Onde o decodificador está de fato escutando. Isto segue o centro de hardware do receptor, não o VFO — em um front end largo os dois podem estar bem distantes, e é a janela que decide quais canais são alcançáveis.
ism-transmissions-gate-opened-how-many = Transmissões em que a porta abriu, e quantas produziram um quadro válido. Muitas rajadas e nenhuma decodificação significa que a faixa está ocupada com dispositivos que o sdroxide ainda não consegue ler. Nenhuma significa que nada chega ao limiar — baixe-o, ou confira se o dial está na faixa.
ism-nothing-heard-yet = nada ouvido ainda
ism-click-tune-receiver =
    {""}

    Clique para sintonizar o receptor nele

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

navtex-messages = MENSAGENS
navtex-sync = SINC
navtex-character-phase-locked-signal-being = A fase do caractere está travada — um sinal está sendo lido.
navtex-no-character-phase-yet-no = Nenhuma fase de caractere ainda: nenhum sinal, ou não este.
navtex-characters-whose-first-copy-was = Caracteres cuja primeira cópia estava corrompida e foram tirados da repetição cinco slots depois, e aqueles em que as duas estavam ruins. Um caractere NAVTEX é enviado duas vezes; isso é toda a sua correção de erro.
navtex-swap-mark-space-tones-signal = Troca os tons de marca e espaço, para um sinal recebido na outra banda lateral. Desligado é a banda lateral superior na frequência do canal, que é o que todas as instruções publicadas de sintonia do serviço dizem.
navtex-starting-navtex-receiver = iniciando o receptor NAVTEX…
navtex-nothing-yet-station-transmits-ten = nada ainda — uma estação transmite por dez minutos a cada quatro horas
navtex-cut-short = interrompida
navtex-message = MENSAGEM
navtex-time-day-stated-message-utc = Hora do dia informada na mensagem (UTC): somente exibição
navtex-receiving = RECEBENDO
navtex-monitor = MONITOR
navtex-everything-decoded-message-not = tudo decodificado, seja mensagem ou não

## rfpaint (src/app/panels/rf_paint.rs)

rfpaint-rf-paint = PINTURA RF
rfpaint-scan-speed = Velocidade de varredura
rfpaint-how-fast-text-image-scanned = Com que rapidez o texto/imagem é varrido na cascata. Mais baixo é mais lento e mais legível; 100% = taxa base, 25% (centro) é o padrão.
rfpaint-abort = Abortar
rfpaint-text-paint = PINTURA DE TEXTO
rfpaint-type-text-paint = Digite o texto para pintar…
rfpaint-preview-waterfall = PRÉVIA NA CASCATA
rfpaint-type-text-preview = digite um texto para visualizar
rfpaint-transmit = {"  "}TRANSMITIR{"  "}
rfpaint-image-paint = PINTURA DE IMAGEM
rfpaint-no-image-loaded = nenhuma imagem carregada
rfpaint-load-image = Carregar imagem…
rfpaint-load-image-preview = carregue uma imagem para visualizar

## hfdl (src/app/hfdl.rs)

hfdl-listen = ESCUTAR
hfdl-switches-engine-s-hfdl-downconverter = Liga ou desliga o conversor de descida e o decodificador HFDL do motor. A decodificação custa uma faixa de 24 kHz e uma thread de trabalho, esteja a janela aberta ou não.
hfdl-channel-assigned-frequency-listen-anything = O canal (a frequência atribuída) em que escutar. Qualquer coisa na faixa funciona — as estações HFDL transmitem de 2,8 a 22 MHz — contra uma faixa fixa de 24 kHz centrada aqui.
hfdl-save-hfdl-decode-log-file = Salvar o log de decodificação HFDL em um arquivo
hfdl-riverhead-northern-atlantic-north-america = Riverhead (Atlântico Norte / América do Norte) — o padrão, e o canal em que o decodificador foi validado.
hfdl-hfdl-assigned-frequency = Uma frequência atribuída HFDL.
hfdl-one-your-saved-channels-click = Um dos seus canais salvos — clique para sintonizar, clique com o botão direito para remover.
hfdl-hfdl-selected-but-decoding-off = HFDL está selecionado, mas a decodificação está desligada — pressione ESCUTAR acima
hfdl-decodes = DECODIFICAÇÕES
hfdl-nothing-decoded-yet-ground-station = Nada decodificado ainda — o squitter de uma estação de solo se repete a cada ~32 s assim que há uma no canal.
hfdl-no-decode-matches-filter = Nenhuma decodificação corresponde ao filtro.
hfdl-decoding = DECODIFICANDO
hfdl-decoding-off = DECODIFICAÇÃO DESLIGADA
hfdl-dbfs = — dBFS

## rade (src/app/panels/rade.rs)

rade-rade = RADE
rade-freedv-v1-digital-voice = Voz digital FreeDV V1
rade-tx = ● TX
rade-mute-analog = SILENCIAR ANALÓGICO
rade-mute-demodulated-audio-so-only = Silencia o áudio demodulado, para que só a fala decodificada seja ouvida. De outro modo, o sinal bruto passa sempre que o modem não tem o que tocar — esse chiado é como se encontra uma transmissão antes de ela sincronizar, então deixe isto desligado ao sintonizar.
rade-sync = SINC
rade-how-far-received-signal-sits = Quão longe o sinal recebido está de onde o modem o espera. Valores grandes ainda decodificam — o laço de aquisição os acompanha —, mas empurrar o dial para trazer isto perto de zero dá a melhor margem.
rade-snr = SNR —
rade-callsign-station-put-end-over =
    O indicativo que a estação colocou no quadro End-of-Over — o único ponto de uma transmissão RADE em que um é enviado. É também o que é informado ao FreeDV Reporter, então uma estação que você ouve aparece no qso.freedv.org como ouvida por você.

    Mantido por um minuto depois que o sinal some, e descartado quando você começa uma transmissão própria: ele nomeia a estação na frequência, e não todas as estações que o receptor já ouviu.
rade-stop-talking = PARAR DE FALAR
rade-talk-ptt = FALAR (PTT)
rade-open-close-rade-over-modem = Abre ou fecha uma transmissão RADE. O modem precisa de cerca de 120 ms de fala antes que o primeiro quadro saia, e envia um quadro de fim de transmissão quando você para, então a transmissão continua um pouco depois do botão.
rade-rade-v1-occupies-roughly-1060 = RADE V1 ocupa cerca de 1060–1880 Hz da banda de passagem — banda lateral superior, ou inferior em 160, 80 e 40 m, onde o RADE mantém a prática de fonia e o sdroxide troca para você. Coloque o sinal dentro da faixa sombreada na cascata; o modem o encontra a partir daí.

## frame (src/app/frame.rs)

frame-acknowledge = Confirmar
frame-dismiss = Dispensar
frame-get-sdroxide-com = Obtenha em sdroxide.com
frame-reconnecting = Reconectando…
frame-not-connected = Não conectado.
frame-reconnect-now = Reconectar agora
frame-transmit-lockout-disabled = ⚠ BLOQUEIO DE TRANSMISSÃO DESATIVADO
frame-engine-was-started-oob-tx = Este motor foi iniciado com --oob-tx. O bloqueio das faixas de amador está desligado: ele acionará o transmissor em qualquer frequência que o hardware suporte.
frame-transmitting-outside-your-licence-offence = Transmitir fora da sua licença é infração em todo país que emite uma. Continue somente se você estiver autorizado a usar as frequências em que está prestes a acionar — uma licença MARS/CAP ou comercial, uma autorização experimental, ou uma carga fantasma.
frame-i-understand = {"  "}ENTENDO{"  "}
frame-restart-without-oob-tx-put = Reinicie sem --oob-tx para restaurar o bloqueio.

## smeter (src/widgets/smeter.rs)

smeter-alc-how-hard-transmitter-being = ALC — com que força o transmissor está sendo excitado, em porcentagem da potência máxima que ele pode entregar. 100 % é a escala total, não a potência total em watts: quanto isso vale na antena depende do rádio e da Excitação.
smeter-radio-reports-no-forward-power =
    {""}

    Este rádio não informa potência direta, então o valor ao lado do ALC é novamente a leitura do ALC, e não watts.
smeter-no-swr-radio-has-no =
    {""}

    Sem ROE: este rádio não tem ponte para medi-la, e o sdroxide não a inventará — ele só pode mostrar o que o próprio rádio informa. Um equipamento com medidor de ROE (pelo CAT, TCI ou um link de LAN) e uma placa HPSDR preenchem isto; um transmissor SDR comum não consegue.
smeter-no-signal-report-yet-receiver = Nenhum relatório de sinal ainda — o receptor não entregou uma leitura de medidor.
smeter-s9-73-dbm-each-s =
    {""}

    S9 é −73 dBm e cada unidade S abaixo dele é 6 dB. O valor em dBm é tão honesto quanto a calibração do front end — um receptor não calibrado informa dBFS com um rótulo de dBm.
smeter-ovld-converter-into-rails-so =
    {""}

    OVLD: o conversor está nos seus limites, então o nível mostrado subestima o sinal. Reduza o ganho do front end.
smeter-table-held = , tabela mantida
smeter-ps = PS --

## multi (src/multi.rs)

multi-tx = ● TX
multi-mute-radio-s-audio = Silenciar o áudio deste rádio
multi-show-radio-here = Mostrar este rádio aqui
multi-show-on-map = Mostrar no mapa
multi-add-radio-computer-settings-radio = Adicionar um rádio neste computador (Configurações → Rádio para adicionar um em uma estação)
multi-add-radio = Adicionar um rádio
multi-take-radio-off-row = Tirar este rádio desta fileira. Ele continua funcionando; os anéis o desligam.
multi-open-pane = Abrir outro painel com um rádio que não está na tela
multi-pick-radio = Rádio ▼
multi-pick-radio-tip = Colocar um rádio nesta fileira. Os anéis marcam os rádios com o link aberto.
multi-resting-connection-tip = Conexão em repouso: escolher disca { $url }
multi-new-radio = Novo rádio…
multi-asked-station-another-radio = Pedi à estação outro rádio…
multi-client-cannot-open-connection = Este cliente não consegue abrir uma conexão.

## net (src/app/net.rs)

net-fail = FALHA
net-will-retry = {" "}(tentará de novo)
net-full-resync-downloading = Ressincronização completa: baixando…
net-downloading = Baixando…
net-full-resync-log-already-matches = Ressincronização completa: o log já coincide com o Wavelog
net-full-resync-review-changes-below = Ressincronização completa: revise as alterações abaixo
net-full-resync-discarded-nothing-changed = Ressincronização completa descartada; nada mudou
net-adif-import-failed-file-could = Falha na importação ADIF: o arquivo não pôde ser lido
net-channel-import-failed-file-could = Falha na importação de canais: o arquivo não pôde ser lido
net-channel-import-nothing-read-chirp = Importação de canais: nada para ler — um arquivo CSV do CHIRP começa com uma linha de cabeçalho com os nomes das colunas, uma das quais deve ser Frequency

## widgets (src/app/panels/widgets.rs)

widgets-free-text = texto livre
widgets-hashed-callsign-not-yet-heard = indicativo com hash, ainda não ouvido por inteiro
widgets-entity-unknown = entidade desconhecida
widgets-band = esta faixa
widgets-new-entity-never-worked-any = Entidade nova — nunca trabalhada, em nenhuma faixa
widgets-new-grid-square = Novo quadrado locator
widgets-not-worked-before = Não trabalhado antes
widgets-worked-before-but-not-band = Trabalhado antes, mas não nesta faixa
widgets-call-queue = Na fila de chamadas
widgets-image = Imagem

## pi4 (src/app/panels/pi4.rs)

pi4-receptions = RECEPÇÕES
pi4-save-pi4-reception-list-csv = Salvar a lista de recepções PI4 como CSV
pi4-next-generation-beacon = Beacon de Próxima Geração
pi4-time-left-one-minute-beacon = Tempo restante neste ciclo de baliza de um minuto. A mensagem PI4 é procurada assim que a janela teve tempo de se encher além de onde ela terminaria.
pi4-decoding = decodificando…
pi4-band = Faixa
pi4-last-cycle = Último ciclo
pi4-nothing-heard = nada ouvido
pi4-receive-only-tune-so-beacon = Somente recepção. Sintonize para que a identificação CW da baliza fique em 800 Hz de áudio — a convenção própria da rede — e os quatro tons PI4 caiam onde este decodificador os procura por padrão.

## awards (src/app/awards.rs)

awards-awards = PRÊMIOS
awards-band = Faixa
awards-all = Todas
awards-dxcc = DXCC
awards-grids = Locators
awards-worked-all-states = Trabalhou Todos os Estados
awards-cq-zones-waz = Zonas CQ (WAZ)
awards-dxcc-entities = Entidades DXCC

## acars (src/app/panels/acars.rs)

acars-acars = ACARS
acars-audio-decoder-s-passband = Áudio na banda de passagem do decodificador.
acars-blocks-framed-blocks-whose-check = Blocos enquadrados, e blocos cuja sequência de verificação falhou.
acars-starting-acars-receiver = iniciando o receptor ACARS…
acars-channel = CANAL
acars-tune-dial-here = Sintonize o dial aqui.
acars-nothing-yet-acars-quiet-between = Nada ainda. O ACARS fica silencioso entre aeronaves — deixe-o em um canal e aguarde uma rajada.
acars-check-failed = verificação falhou
acars-block-check-sequence-did-not = A sequência de verificação do bloco não coincidiu — o texto pode estar errado.
acars-no-text = (sem texto)

## login (src/login.rs)

login-sdr-oxide = SDR OXIDE
login-username = Usuário
login-password = Senha
login-remember-device = Lembrar neste dispositivo
login-signs-without-asking-next-time = Entra sem perguntar na próxima vez, e deixa a aba da visão 3D entrar sem perguntar de jeito nenhum. A senha fica guardada em texto puro, então desligue isto em uma máquina que outras pessoas usam.
login-keeps-password-device-clear = Mantém a senha neste dispositivo, em texto puro.
login-remember-station = Lembrar esta estação
login-remember-me = LEMBRAR DE MIM
login-station-keeps-sign-in = É a estação que guarda o seu login, não esta página — nada a preencher nos outros rádios e nada a esquecer ao sair.
login-not-this-time = Desta vez não: este navegador esquece ao fechar, então você será solicitado de novo na próxima sessão e nada deste login fica no dispositivo.
login-checking = {" "}VERIFICANDO…{" "}
login-sign = {" "}ENTRAR{" "}
login-wrong-password-shuts-server-s = Uma senha errada fecha a porta do servidor por alguns segundos, então isto pode levar um momento.

## help (src/help.rs)

help-find = BUSCAR
help-find-manual = buscar no manual
help-previous-match-shift-enter-shift = Ocorrência anterior (Shift+Enter, Shift+F3)
help-next-match-enter-f3 = Próxima ocorrência (Enter, F3)
help-no-matches = nenhuma ocorrência
help-clear-search-esc = Limpar a busca (Esc)
help-sdroxide-manual = MANUAL DO SDROXIDE
help-contents = CONTEÚDO

## aismap (src/ais_map.rs)

aismap-t-1-station-heard-waiting-report = 1 estação ouvida — aguardando que ela informe uma posição
aismap-double-click-reframe = clique duas vezes para reenquadrar
aismap-zoom-out-wheel-pinch-do = Reduzir o zoom — a roda ou um pinça também fazem isto
aismap-zoom-about-middle-chart = Ampliar em torno do meio da carta
aismap-frame-everything-being-tracked-again = Enquadrar tudo o que está sendo rastreado de novo — um clique duplo também faz isto

## specview (src/widgets/spectrum_view.rs)

specview-waiting-spectrum = aguardando o espectro…
specview-hounds = CÃES

## spectrum (src/app/spectrum.rs)

spectrum-window-has-no-gpu-renderer = Esta janela não tem renderizador de GPU, então a cascata fica na largura padrão.
spectrum-machine-drawing-without-gpu-every = Esta máquina desenha sem GPU — cada coluna da cascata é trabalho do processador, que já o divide com o rádio.
spectrum-window-renders-through-opengl-which = Esta janela renderiza por OpenGL, que é o caminho de compatibilidade do sdroxide — um Raspberry Pi, um chip gráfico antigo, ou um navegador sem WebGPU. Uma cascata mais larga não vale a taxa de quadros nesses casos.
spectrum-wider-than-renderer-can-draw = Mais larga do que este renderizador consegue desenhar.

## macros (src/app/panels/macros.rs)

macros-nocall = SEMCHAMADA
macros-each-button-sends-whole-text = Cada botão envia o texto inteiro de uma vez. F2–F10 acionam os nove primeiros, desde que nada na tela esteja com o teclado.
macros-remove-button = Remover este botão

## gridtracker (src/app/grid_tracker.rs)

gridtracker-grid-tracker = RASTREADOR DE LOCATORES
gridtracker-shade-squares-heard-live-decode = Sombreia os quadrados ouvidos na lista de decodificações ao vivo, e não só os do diário — o que está no ar agora, em ciano sob os quadrados trabalhados.
gridtracker-drag-pan-wheel-zoom-amber = Arraste para deslocar, a roda para ampliar. Âmbar = trabalhado, verde = confirmado, ciano = ouvido.

## remote (src/remote.rs)

remote-server-busy-another-client-connected = servidor ocupado — outro cliente está conectado
remote-transmitting-but-client-s-microphone = Transmitindo, mas o microfone deste cliente não produz áudio nenhum — nada está sendo modulado. Em um navegador, confira se a página recebeu permissão para o microfone e se nenhum outro aplicativo o está usando.
remote-connection-closed = conexão encerrada

## solarwin (src/solar3d/mod.rs)

solarwin-sdroxide-solar-system = sdroxide — sistema solar

## solarbtn (src/app/solar.rs)

solarbtn-solar-system-3d-view-sun = Visão 3D do sistema solar — Sol, Terra, Lua, manchas solares e CMEs (janela separada)
solarbtn-solar-system-3d-view-sun-2 = Visão 3D do sistema solar — Sol, Terra, Lua, manchas solares e CMEs (nova aba do navegador)

## widespec (src/widgets/wide_spectrum.rs)


## speech (src/app/speech.rs)

speech-loading-voice = Carregando a voz…
speech-browser-client-cannot-speak-yet = o cliente de navegador ainda não consegue falar

## freqdisplay (src/widgets/freq_display.rs)

freqdisplay-double-click-type-frequency = Clique duas vezes para digitar uma frequência
freqdisplay-tap-type-frequency = Toque para digitar uma frequência

## solarstate (src/solar3d/state.rs)

solarstate-home = INÍCIO
solarstate-dwarf-planets = PLANETAS ANÕES

## adsbmap (src/adsb_map.rs)

adsbmap-double-click-reframe = clique duas vezes para reenquadrar

## aprsmap (src/aprs_map.rs)

aprsmap-double-click-reframe = clique duas vezes para reenquadrar

## solarnet (src/solar3d/net.rs)

solarnet-connection-closed = conexão encerrada

## alerts (src/app/alerts.rs)

alerts-could-not-start-alert-thread = não foi possível iniciar a thread de alerta

## hfdlmap (src/hfdl_map.rs)

hfdlmap-no-aircraft-located-yet = nenhuma aeronave localizada ainda

## worldmap (src/widgets/worldmap.rs)

worldmap-double-click-reframe = CLIQUE DUAS VEZES PARA REENQUADRAR

## chrome (src/chrome.rs)



## duopower (src/app/duo_power.rs)

duopower-sets-fdm-duo-s-own = Define a configuração de potência própria do FDM-DUO (menu 34 TX POWER). Excitação e Tune então definem a potência dentro dela, e o SDRoxide envia a configuração de novo a cada conexão.

## Janela de condições de faixa: uma data de pico.

# $month vai de 1 a 12.
bands-peak-date = { $day } { $month ->
        [1] jan
        [2] fev
        [3] mar
        [4] abr
        [5] mai
        [6] jun
        [7] jul
        [8] ago
        [9] set
        [10] out
        [11] nov
       *[12] dez
    }

## download (src/download.rs)

download-browser-could-not-read-file = o navegador não conseguiu ler esse arquivo

## morse (src/app/morse.rs)

morse-translate = TRADUZIR
morse-practice = PRATICAR
morse-learn = APRENDER
morse-morse = MORSE
morse-text-morse = Texto → Morse
morse-morse-text = Morse → Texto
morse-per-character-space-between-characters = `.` e `-` por caractere, um espaço entre caracteres, ` / ` entre palavras. O alfabeto ITU, dígitos, pontuação e os prosignos comuns.
morse-reference-translator-available-desktop-build = O tradutor de referência está disponível na versão para desktop.
morse-play-what-you-type-your = Toca o que você digita, nos seus próprios alto-falantes
morse-play = TOCAR
morse-stop = PARAR
morse-every-character-learned-well-done = Todos os caracteres aprendidos — muito bem.
morse-drill-needs-sound-which-desktop = O exercício precisa de som, que a versão para desktop tem.
morse-reveal = REVELAR
morse-reset = REDEFINIR
morse-replay = REPETIR
morse-what-did-you-hear = O que você ouviu?
morse-check = VERIFICAR
morse-press-new-hear-character = Pressione NOVO para ouvir um caractere.
morse-tone = Tom
morse-speed = Velocidade
morse-spacing = Espaçamento
morse-characters-go-out-speed-above = Os caracteres saem na velocidade acima; esta é a velocidade em que os *intervalos* são enviados, para que um iniciante ouça a letra inteira antes de conseguir enviá-la nessa velocidade.
morse-playback-available-desktop-build = A reprodução está disponível na versão para desktop.

## Painel ADS-B: linhas de detalhe das aeronaves.

adsb-row-altitude = altitude
adsb-row-track = rota
adsb-row-turning = fazendo curva
adsb-row-squawk = squawk
adsb-row-range = alcance
adsb-row-position = posição
adsb-row-signal = sinal
adsb-row-frames = quadros
rade-transmitting = transmitindo
decodes-odd = ÍMPAR
decodes-new = NOVA
js8-new = NOVA

## Painel AIS: linhas de detalhe das embarcações.

ais-row-status = estado
ais-row-speed = velocidade
ais-row-course = rumo
ais-row-heading = proa
ais-row-turning = fazendo curva
ais-row-altitude = altitude
ais-row-size = tamanho
ais-row-draught = calado
ais-row-destination = destino
ais-row-range = alcance
ais-row-signal = sinal
ais-row-type = tipo
ais-row-position = posição
ais-row-messages = mensagens
ism-off = DESLIGADO
textmodem-frequency-shifted = deslocado em frequência
winlink-to-heading = PARA

## adsbmap: mensagens com valores (src/adsb_map.rs)

adsbmap-ft-min =
    {""}
    { $vr } pés/min

## aismap: mensagens com valores (src/ais_map.rs)

aismap-heard-no-position-newer-than = { $heard } ouvidas — nenhuma posição mais recente que { $drop_map_s } s
aismap-stations-heard-waiting-position-report = { $heard } estações ouvidas — aguardando um relatório de posição
aismap-km-across = { $lon_span }° · ~{ $km } km de largura

## alerts: mensagens com valores (src/app/alerts.rs)

alerts-alarming = alarmando em { $device }
alerts-alarm-device-unavailable = dispositivo de alarme indisponível: { $error }

## awards: mensagens com valores (src/app/awards.rs)

awards-worked = { $w } trabalhadas
awards-confirmed = { $value } confirmadas

## bands: mensagens com valores (src/app/bands.rs)

bands-forecast-hamqsl-com-old = · previsão do HAMQSL.com, de { $value } atrás
bands-ncdxf-iaru-next-beacon-s = NCDXF/IARU · próxima baliza em { $now } s
bands-hearing-means-path-open-each = { $callsign } em { $location } — ouvi-la significa que o percurso de { $band } está aberto. Cada baliza sobe uma faixa a cada 10 s, então esta linha muda a cada slot.
bands-radiants-placed = radiantes posicionados para { $lat }°, { $lon }°
bands-zhr = ZHR { $zhr }
bands-radiant = radiante { $alt_deg }° { $az_deg }
bands-radiant-down = radiante abaixo ({ $alt_deg }°)
bands-km-s-parent-peak-rate =
    { $value } ({ $code }) — { $velocity_kms } km/s, pai { $parent }. Taxa de pico ZHR { $zhr } por volta de { $peak }.

    Um radiante acima do horizonte significa que os rastros podem alcançar você; uma chuva rápida deixa rastros ionizados de vida mais longa para dispersão meteórica em 6 m e 2 m, e as breves aberturas em 10 m.

## drm: mensagens com valores (src/app/drm.rs)

drm-not-decoding = { $channel } não decodificando
drm-symbols = { $value } símbolos · { $frame }
drm-not-decodable = { $value } — não decodificável
drm-broadcaster-s-clock-utc = relógio da emissora { $year }-{ $month }-{ $day } { $hour }:{ $minute } UTC

## duopower: mensagens com valores (src/app/duo_power.rs)

duopower-max-fdm-duo-s-full =
    MAX: a potência total do FDM-DUO, cerca de 8 W. A configuração a usar com o HR500 na linha.

    { $base }
duopower-t-5-w-hr500-line-will =
    5 W. Com o HR500 na linha, ele não atingirá a potência total na maioria das faixas com esta configuração: use MAX, e Excitação para reduzi-la.

    { $base }
duopower-w-running-barefoot-qrp =
    { $code } W, para operar sem amplificador ou em QRP.

    { $base }

## frame: mensagens com valores (src/app/frame.rs)

frame-re-enables-transmit-after-1 = Reativa a transmissão após { $swr }:1. Verifique a antena primeiro: se a falha continuar, a próxima transmissão também para.
frame-sdroxide-has-been-released = O SDRoxide { $version } foi lançado — esta é a { $sdroxide_version }.
frame-reconnecting-s = Reconectando em { $value }s…

## gridtracker: mensagens com valores (src/app/grid_tracker.rs)

gridtracker-worked = { $worked } trabalhados
gridtracker-confirmed = { $confirmed } confirmados
gridtracker-heard = { $heard } ouvidos

## hd: mensagens com valores (src/app/hd.rs)

hd-decode = Decodificar { $value } ({ $codec_label })

## hfdl: mensagens com valores (src/app/hfdl.rs)

hfdl-khz-already-has-chip = { $cur_khz } kHz já tem um chip
hfdl-save-current-channel-khz-one = Salvar o canal atual ({ $cur_khz } kHz) como um dos seus próprios chips
hfdl-aircraft = { $aircraft } aeronaves
hfdl-bursts = { $bursts } rajadas
hfdl-decodes-2 = { $decodes } decodificações
hfdl-fixed = { $fec } corrigidas

## ism: mensagens com valores (src/app/ism.rs)

ism-not-decoded-yet = { $hint } — ainda não decodificado
ism-listen-tunes-radio-there-one = Escutar em { $value } — sintoniza o rádio ali. Uma faixa por vez: elas estão distantes demais para qualquer receptor cobrir duas ao mesmo tempo.
ism-watch-hz-around-band-centre = Observa { $value } Hz em torno do centro da faixa, seja o que for que a faixa pediria. Mais estreito serve a um receptor que não consegue entregar a largura própria da faixa; mais largo alcança dispositivos mais afastados do centro. O seu receptor precisa entregar cerca de um terço a mais que isto.
ism-rtl-433 = rtl_433 — { $why }
ism-rtl-433-khz = rtl_433 { $value } · { $khz } kHz
ism-your-own = { $flex } seu
ism-decoders-decoded = { $decoders } decodificadores ({ $flex }), { $decodes } decodificados
ism-handled-rtl-433 = tratado pelo rtl_433: { $names }
ism-flex = flex: { $error }
ism-window-mhz-mhz-wide = janela { $mhz } MHz, { $e62 } MHz de largura
ism-bursts-decoded = { $bursts } rajadas, { $decodes } decodificadas
ism-devices = { $ism_reports } dispositivos
ism-id-mhz-db-heard-first =
    { $protocol } id { $device }
    { $e6 } MHz { $snr_db } dB ouvido { $value }×
    ouvido pela primeira vez há { $first_at }

## logbook: mensagens com valores (src/app/logbook.rs)

logbook-confirmations-downloaded-newly-confirmed = Confirmações: { $recs } baixadas, { $matched } recém-confirmadas
logbook-qso = { $qso_log } QSO
logbook-worked-before = ⚠ TRABALHADA ANTES ({ $dupe_band })
logbook-utc-qso = { $oldest }–{ $newest } UTC · { $value } QSO

## app: mensagens com valores (src/app/mod.rs)

app-radio = rádio { $id }

## morse: mensagens com valores (src/app/morse.rs)

morse-could-not-start-morse-audio = não foi possível iniciar a thread de áudio Morse: { $error }
morse-no-audio-output = sem saída de áudio: { $error }
morse-about-s = cerca de { $farnsworth_wpm } s
morse-characters-play = Caracteres em uso: { $set }
morse-score-run = Pontuação: { $correct } / { $total }. Sequência: { $streak } de { $value }.
morse-correct = correto — { $value }
morse-was = era { $value }

## net: mensagens com valores (src/app/net.rs)

net-synced-z-new-already-log = Sincronizado às { $when }Z: { $added } novos, { $matched } já no log
net-wavelog-2 = Wavelog: { $line }
net-full-resync-stopped-nothing-changed = Ressincronização completa interrompida, nada mudou: { $error }
net-full-resync-applied-added-updated = Ressincronização completa aplicada: { $add } acrescentados, { $update } atualizados, { $remove } removidos, { $mark } dos seus marcados como presentes no Wavelog
net-wavelog-3 = Wavelog: { $line }
net-adif-import-failed = Falha na importação ADIF: { $error }
net-not-unicode-read = {" "}(não Unicode; lido como { $enc })
net-received-reports-swl-left-out = , { $n } relatórios recebidos (SWL) deixados de fora
net-adif-import-added-duplicates-skipped = Importação ADIF: { $added } acrescentados, { $skipped } duplicados ignorados{ $reports }{ $assumed }
net-channel-import-failed = Falha na importação de canais: { $error }
net-line-s-skipped = , { $n } linha(s) ignorada(s)
net-not-unicode-read-2 = {" "}(não Unicode; lido como { $enc })
net-channel-import-channel-s-read = Importação de canais: { $n } canal(is) lido(s){ $skipped }{ $assumed }

## acars: mensagens com valores (src/app/panels/acars.rs)

acars-frames-bad = { $frames } quadros · { $bad } ruins
acars-mode = modo { $mode }

## adsb: mensagens com valores (src/app/panels/adsb.rs)

adsb-aircraft-3 = { $aircraft } aeronaves
adsb-frames = { $frames } quadros
adsb-preambles = { $preambles } preâmbulos
adsb-bad-crc = { $bad_crc } CRC ruins
adsb-tune = SINTONIZAR { $mhz }
adsb-ft-barometric = { $ft } pés barométricos

## ais: mensagens com valores (src/app/panels/ais.rs)

ais-vessels-3 = { $vessels } embarcações
ais-messages = { $messages } mensagens
ais-slots = { $bursts } slots
ais-bad-fcs = { $bad_fcs } FCS ruins
ais-ais-mhz = AIS { $value } ({ $mhz } MHz) — { $value2 }
ais-ais-mhz-slots-messages-floor = AIS { $value } ({ $mhz } MHz) — { $bursts } slots, { $messages } mensagens, piso { $floor_dbfs } dBFS
ais-ais = AIS { $value }
ais-tune = SINTONIZAR { $mhz }
ais-ships-heard-are-khz-off = As embarcações ouvidas estão { $khz } kHz fora de frequência ({ $mhz } ppm em 162 MHz) — ajuste a correção de frequência do front end, ou as transmissões acima de cerca de 5 kHz deixarão de decodificar
ais-over-ground = { $value }° sobre o solo
ais-dbfs-db-snr = { $rssi_dbfs } dBFS, { $snr_db } dB de SNR
ais-last-type-ais = { $messages } (último tipo { $last_type }, AIS { $channel })
ais-ais-2 = AIS { $ch }
ais-mhz-marine-channel-ship-alternates = { $mhz } MHz — canal marítimo { $ch }. Um navio alterna entre os dois, então desligar um reduz pela metade a frequência com que cada embarcação é ouvida.

## aprs: mensagens com valores (src/app/panels/aprs.rs)

aprs-object = objeto de { $reported_by }
aprs-via = via { $via }
aprs-object-reported = objeto informado por { $reported_by }
aprs-digit-s-blanked = ± { $ambiguity } dígito(s) apagado(s)
aprs-wind-m-s = vento { $value }° { $value2 } m/s
aprs-gust = rajada { $value }
aprs-rain-mm-h = chuva { $value } mm/h
aprs-frame-s-last-ago = { $packets } quadro(s), último há { $last_heard }
aprs-via-2 = via { $via }
aprs-bad = { $bad_frames } ruins
aprs-other = { $non_aprs } outros
aprs-sent = enviado ×{ $tries }
aprs-stn = { $stations } est.
aprs-queued = { $tx_queue } na fila

## atchat: mensagens com valores (src/app/panels/atchat.rs)

atchat-master = mestre { $value }
atchat-last-heard-s-ago-click = { $status } — última escuta há { $age_s }s · clique para abrir uma aba de mensagem direta
atchat-close-tab = fechar a aba de { $peer }
atchat-direct-message = mensagem direta para { $value }
atchat-cannot-display = não é possível exibir { $path }

## cw: mensagens com valores (src/app/panels/cw.rs)

cw-click-decoder-neural-one-copies =
    { $hint }

    Clique para o decodificador { $neural }. O neural lê mais fundo e capta CW manipulado à mão que um ajuste de tempo não aceitaria; o de tempo é o único que lê as letras acentuadas — Ä, Ö, Å, Ü, É — porque o modelo não tem classe de saída para elas.
cw-idle-s = OCIOSO { $idle }s

## decodes: mensagens com valores (src/app/panels/decodes.rs)

decodes-save-decodes-list-adif-received = Salvar as { $n } decodificações da lista como ADIF — relatórios recebidos, não contatos. Decodificações que não nomeiam remetente ficam de fora.
decodes-save-decodes-list-csv-one = Salvar as { $n } decodificações da lista como CSV, uma linha cada
decodes-session-qso = Sessão: { $session } QSO
decodes-save-whole-logbook-qso-adif = Salvar o diário inteiro ({ $logged } QSO) como ADIF
decodes-save-whole-logbook-qso-text = Salvar o diário inteiro ({ $logged } QSO) como texto
decodes-hz-slots = { $audio_hz } Hz · { $odd } slots
decodes-your-slot-timing-against-stations =
    A sua temporização de slot em relação às estações que você ouve.
    { $after }

    Abrange toda a cadeia de recepção, então uma cadeia de áudio ou de rede lenta conta o mesmo que um relógio errado. Abaixo de 0,5 s é confortável.
decodes-pile-up = PILE-UP { $fox_queue }
decodes-queue = FILA { $call_queue }
decodes-db-hz-click-remove =
    { $waiting } · { $snr_db } dB · { $audio_hz } Hz{ $unwrap_or_default }
    Clique para remover

## js8: mensagens com valores (src/app/panels/js8.rs)

js8-every-speed-being-decoded-marks = Todas as velocidades estão sendo decodificadas; ▸ marca { $speed }, a em que você transmite
js8-every-speed-being-decoded-click = Todas as velocidades estão sendo decodificadas. Clique para transmitir em { $speed }
js8-transmit-decode = Transmitir e decodificar em { $speed }
js8-beaconing-every-min-click-stop = Transmitindo baliza a cada { $hb_min } min — clique para parar
js8-last-beacon-went-out-hz = A última baliza saiu em { $hz } Hz — um slot livre na sub-faixa de heartbeat de { $value }–{ $value2 } Hz, escolhido para ficar longe dos sinais que estão sendo decodificados.
js8-frames-s = { $tx_frames_pending }/{ $tx_frames_total } quadros · { $left }s
js8-decoded = Decodificado em { $speed }
js8-frames = … ({ $frames } quadros)
js8-reply-2 = Responder a { $value }: “{ $value2 }”
js8-address-composer = Endereçar o compositor a { $value }

## macros: mensagens com valores (src/app/panels/macros.rs)

macros-f-sends = F{ $i }: envia “{ $text }”
macros-sends = Envia “{ $text }”
macros-most = { $value } é o máximo

## navtex: mensagens com valores (src/app/panels/navtex.rs)

navtex-repaired-lost = { $repaired } reparados · { $lost } perdidos
navtex-characters-lost = { $lost } caracteres perdidos

## packet: mensagens com valores (src/app/panels/packet.rs)

packet-bad = { $bad_frames } ruins
packet-link-layer = A camada de enlace está em { $link }
packet-via = {" "}via { $via }
packet-unacked = { $unacked } sem confirmação
packet-retry = retentativa { $retries }

## pi4: mensagens com valores (src/app/panels/pi4.rs)

pi4-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] baliza
       *[other] balizas
    }
pi4-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } atrás

## rade: mensagens com valores (src/app/panels/rade.rs)

rade-snr-db = SNR { $snr_db } dB
rade-offset-hz = deslocamento { $freq_offset_hz } Hz
rade-heard = ouvido { $call }
rade-samples-dropped-between-receiver-decoder = ⚠ { $dropped } amostras descartadas entre o receptor e o decodificador — esta máquina não está acompanhando a decodificação neural.

## textmodem: mensagens com valores (src/app/panels/text_modem.rs)

textmodem-char-s = { $chars_per_sec } caract./s
textmodem-char-s-hz-wide = { $value } — { $chars_per_sec } caract./s, { $bandwidth_hz } Hz de largura, { $keyed }

## vdl2: mensagens com valores (src/app/panels/vdl2.rs)

vdl2-frames = { $frames } quadros
vdl2-bursts = { $bursts } rajadas
vdl2-sync = { $syncs } sincronismo
vdl2-hdlc-bad = { $hdlc_bad } HDLC ruins
vdl2-bad-fcs = { $fcs_bad } FCS ruins
vdl2-mhz-bursts-frames-noise-floor = { $mhz } MHz, { $what } — { $bursts } rajadas, { $frames } quadros, piso de ruído { $floor_dbfs } dBFS
vdl2-tune = SINTONIZAR { $mhz }
vdl2-held-2 = RETIDO +{ $since }
vdl2-use-worldwide-where-every-link = { $what } — em uso no mundo todo, e onde todo enlace começa. A que manter se você mantiver só uma.
vdl2-assigned-which-channels-carry-anything = Atribuído a um { $what }. Quais canais transportam algo depende de onde você está: deixe todos ligados, a menos que saiba de outra forma.
vdl2-mhz-db-snr-dbfs-evm = { $mhz } MHz · { $snr_db } dB SNR · { $rssi_dbfs } dBFS · EVM { $evm_deg }° · { $freq_err_hz } Hz · { $rs_corrected } símbolos RS corrigidos
vdl2-characters = { $parity_errors } caracteres
vdl2-parameter-sdroxide-does-not-read = { $unknown } { $unknown ->
        [one] parâmetro
       *[other] parâmetros
    } que o SDRoxide não lê — mostrados acima como identificador e bytes

## wefax: mensagens com valores (src/app/panels/wefax.rs)

wefax-lines = { $lines } linhas
wefax-lines-held = { $live_size } linhas retidas
wefax-pixels-per-line = { $width } pixels por linha
wefax-shift-picture-pixels = Desloca a imagem { $px } pixels
wefax-charts-are-saved-click-copy =
    As cartas são salvas { $place }

    Clique para copiar o caminho
wefax-finished-charts-are-saved-collect = As cartas concluídas são salvas { $place } e se acumulam aqui.
wefax-pixels-right-click-delete =
    { $when_full }
    { $size } × { $size2 } pixels
    { $value }

    Clique com o botão direito para excluir
wefax-older-store = { $older } mais antigas no armazenamento
wefax-older-load-more = { $older } mais antigas — carregar mais
wefax-published-carrier-khz-dial-khz = Portadora publicada { $f } kHz → dial { $value } kHz USB
wefax-delete-chart-2 = Excluir esta carta { $dir }

## widgets: mensagens com valores (src/app/panels/widgets.rs)

widgets-cq-zone-itu-zone = { $continent } · zona CQ { $cq_zone } · zona ITU { $itu_zone }
widgets-new-entity = Entidade nova em { $band_label }
widgets-worked-before = Trabalhado antes em { $band_label }
widgets-calling-cq-includes-you = Chamando CQ { $target } — inclui você
widgets-calling-cq-not-aimed-you = Chamando CQ { $target } — não dirigido a você

## wspr: mensagens com valores (src/app/panels/wspr.rs)

wspr-rx-heard-us = { $heard_us } rx · { $heard_us2 } ouviram a gente
wspr-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] baliza
       *[other] balizas
    }
wspr-beaconing-duty = transmitindo baliza — { $wspr_power_dbm }, ciclo de { $wspr_tx_percent }%
wspr-listening-duty = escutando (ciclo de { $wspr_tx_percent }%)
wspr-beacon-one-two-minute-slot = Baliza em um slot de dois minutos a cada { $pct } — { $pct2 }% deles, espaçados igualmente — em { $wspr_power_dbm }. O slot em que o ciclo começa é sorteado a partir do seu indicativo, então duas estações rodando este programa não começam juntas.
wspr-transmitting-set-general-tab-settings = Transmitindo como { $call } em { $sent } — definido na aba Geral das Configurações.
wspr-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } atrás

## publicsdr: mensagens com valores (src/app/publicsdr.rs)

publicsdr-antenna =
    { $address }
    { $device }
    antena: { $antenna }
    { $e3 }
publicsdr-receivers-fetched-ago = { $entries } receptores · { $per_net } · buscado há { $age }
publicsdr-showing-search-narrow = mostrando { $shown } de { $n } — pesquise para restringir
publicsdr-match = { $n } resultado
publicsdr-map-status = { $shown } no mapa · { $unplaced } sem posição
publicsdr-opening-another-radio = Abrindo { $entry } como outro rádio…

## qo100: mensagens com valores (src/app/qo100.rs)

qo100-khz-3 = +{ $park_lo_hz } … +{ $park_hi_hz } kHz
qo100-before-switching-tune-so-beacon = Antes de LIGAR: sintonize para que os dois lóbulos da baliza fiquem entre +{ $park_lo_hz } e +{ $park_hi_hz } kHz na faixa (a pista sombreada).
qo100-receiver-currently-listening-around-mhz = O receptor está escutando em torno de { $mhz } MHz — longe da baliza de 10489,750 MHz, então não há nada aqui para desenhar. O decodificador continua pesquisando de qualquer forma.
qo100-receiver-currently-covers-only-khz = o receptor cobre só ±{ $khz } kHz aqui — o resto da faixa fica em branco
qo100-null-db-sym-snr-db = nulo { $est_null_depth_db } dB sim { $est_symmetry } snr { $est_snr_db } dB
qo100-hz-s-hz-s-de = { $est_drift_hz_s } Hz/s { $est_drift_accel_hz_s2 } Hz/s² (desrotacionado antes da decodificação)
qo100-hz-s-de-rotated-before = { $est_drift_hz_s } Hz/s (desrotacionado antes da decodificação)
qo100-found-empty = { $est_updates } encontradas, { $est_misses } vazias
qo100-last-s-ago = {"   "}última { $auto_last_hz } (há { $auto_last_unix }s)
qo100-over-correction = { $auto_total_hz } em { $auto_applies } { $auto_applies ->
        [one] correção
       *[other] correções
    }{ $last }
qo100-sync-32-err = sincronismo { $sync_bit_errors } / 32 erros ×{ $sync_matches }
qo100-frame-buffer = buffer de quadros { $frame_fill }%
qo100-blocks-tried-decoded = { $blocks_tried } blocos tentados, { $blocks_locked } decodificados
qo100-last-applied-s-ago-hz = última aplicada há { $ago }s: { $old } → { $new } Hz ({ $old2 } Hz)
qo100-locked-block-tried-locked = travado — { $blocks_tried } { $blocks_tried ->
        [one] bloco tentado
       *[other] blocos tentados
    }, { $blocks_locked } travados
qo100-searching-block-tried-none-locked = pesquisando — { $blocks_tried } { $blocks_tried ->
        [one] bloco tentado
       *[other] blocos tentados
    }, nenhum travado ainda

## rds: mensagens com valores (src/app/rds.rs)

rds-auto = Automático ({ $standard })
rds-type = tipo { $class }
rds-groups = · { $groups } grupos
rds-blocks-error = · { $ber } % dos blocos com erro
rds-clean-corrected-lost = { $blocks_ok } limpos · { $blocks_corrected } corrigidos · { $blocks_bad } perdidos

## sat: mensagens com valores (src/app/sat.rs)

sat-utc-local = { $unix } UTC (local { $offset })
sat-aos = AOS { $rise_unix } { $rise_unix2 }
sat-los = LOS { $set_unix } { $set_unix2 }
sat-above-horizon = { $rise_unix } acima do horizonte
sat-min = em { $value } min
sat-h-min = em { $value } h { $value2 } min
sat-searching = pesquisando… { $next } de { $entries }
sat-locked-2 = ● TRAVADO — { $t }
sat-pass-until-max = Passagem até { $set_unix } · máx. { $max_el }°
sat-next-pass-rises-max = Próxima passagem { $rise_unix } { $rise_unix2 } ({ $now }) · sobe em { $rise_az }° { $rise_az2 } · máx. { $max_el }°
sat-rigctld-client-connected-satellite-software = ⚠ { $clients } { $clients ->
        [one] cliente rigctld conectado
       *[other] clientes rigctld conectados
    } — um software de satélite comandando o dial ali corrigiria o Doppler duas vezes
sat-rotator-antenna-az-el = Rotor: antena em { $az }° az { $el }° el
sat-rotator-2 = Rotor: { $error }
sat-up-now-2 = ● visível agora, { $el }°
sat-inv = { $mode } · inv
sat-norad-2 = NORAD { $norad_id }

## scanner: mensagens com valores (src/app/scanner.rs)

scanner-holding-mhz = retendo { $here } MHz
scanner-scanning-mhz = varrendo · { $here } MHz
scanner-pano-first = Panorama { $range } MHz · primeira varredura …
scanner-pano-sweeps = Panorama { $range } MHz · varredura { $n } · { $secs } s
scanner-pano-clear = Descartar o panorama e varrer de novo
scanner-forward = Avançar: seguir para cima
scanner-backward = Voltar: inverter e seguir para baixo

## spectrum: mensagens com valores (src/app/spectrum.rs)

spectrum-renderer-will-not-hold-texture = Este renderizador não suporta uma textura mais larga que { $max_texture_dim } pixels.
spectrum-mem = Mem: { $f } / { $value }
spectrum-mem-2 = Mem: { $value }

## speech: mensagens com valores (src/app/speech.rs)

speech-voice-2 = Voz: { $voice }
speech-speech-unavailable = A fala está indisponível: { $error }

## spots: mensagens com valores (src/app/spots.rs)

spots-transmissions = { $stations } transmissões

## winlink: mensagens com valores (src/app/winlink.rs)

winlink-last-session-failed = última sessão falhou — { $err }
winlink-last-session-received-sent = última sessão: { $last_received } recebidas, { $last_sent } enviadas
winlink-empty = { $to_lowercase } está vazia
winlink-showing = mostrando { $entries } de { $total }
winlink-fetching = buscando { $mid }…
winlink-from-2 = de { $msg } { $date }
winlink-attachment-bytes = anexo: { $att } ({ $data } bytes)

## aprsmap: mensagens com valores (src/aprs_map.rs)

aprsmap-object =
    {""}
    objeto de { $reported_by }
aprsmap-heard-ago =
    {""}
    ouvido há { $age }

## download: mensagens com valores (src/download.rs)

download-reading = lendo { $path }: { $error }

## help: mensagens com valores (src/help.rs)

help-missing-image = {"["}imagem ausente: { $path }]

## hfdlmap: mensagens com valores (src/hfdl_map.rs)

hfdlmap-mhz =
    {""}
    { $e3 } MHz
hfdlmap-fix =
    {""}
    { $fixes } posição{ $es }
hfdlmap-heard-ago =
    {""}
    ouvido há { $last_at }

## multi: mensagens com valores (src/multi.rs)

multi-radio = Rádio { $n }
multi-could-not-add-radio = Não foi possível adicionar um rádio: { $error }
multi-was-closed-station = { $value } foi fechado na estação.
multi-could-not-open = Não foi possível abrir { $peer }: { $error }
multi-has-tab-own-now = { $value } agora tem uma aba própria.

## remote: mensagens com valores (src/remote.rs)

remote-protocol-error = erro de protocolo: { $error }

## solarnet: mensagens com valores (src/solar3d/net.rs)

solarnet-bad-message = mensagem inválida: { $error }
solarnet-solar-protocol-mismatch-server-client = incompatibilidade de protocolo solar: servidor { $proto }, cliente { $value }

## smeter: mensagens com valores (src/widgets/smeter.rs)

smeter-forward-power-w-measured-radio =
    {""}

    Potência direta — { $w } W, medida pelo sensor próprio do rádio.
smeter-po-radio-s-own-power =
    {""}

    PO — o medidor de potência de saída próprio do rádio, a { $clamp } % da escala total. Uma posição de ponteiro como o rádio a informa, não uma potência calibrada em watts; o que observar é se ela cai enquanto a Excitação permanece a mesma.
smeter-swr-1-match-antenna-presenting =
    {""}

    ROE — { $swr }:1, o casamento que a antena apresenta, como o rádio o mede. Abaixo de 2:1 é normal; acima de 3:1 a escala fica vermelha.
smeter-received-signal-strength-s9-db = Intensidade do sinal recebido: S9 + { $over } dB ({ $s_dbm } dBm na banda de passagem).
smeter-received-signal-strength-s-dbm = Intensidade do sinal recebido: S{ $value } ({ $s_dbm } dBm na banda de passagem).
smeter-radio-reports-c =
    {""}

    O rádio informa { $t } °C.
smeter-ps-puresignal-correcting-db-compression =
    {""}

    PS — o PureSignal está corrigindo { $correction_db } dB de compressão{ $held } (a realimentação coincidiu com a transmissão em { $score }).
smeter-ps-puresignal-switched-but-has =
    {""}

    PS −− — o PureSignal está ligado, mas não encontrou a transmissão no caminho de realimentação (melhor coincidência { $score }), então o transmissor sai sem correção. Confira se uma amostra da saída do amplificador realmente chega a uma entrada de recepção que a chave T/R não retira na transmissão, e se o atenuador dela não a está engolindo. O laço escuta somente este receptor — um acoplador ligado a um segundo receptor (RX2, ADC2) nunca é lido como realimentação.

## specview: mensagens com valores (src/widgets/spectrum_view.rs)

specview-rit-hz = RIT { $hz } Hz
specview-xit-hz = XIT { $hz } Hz

## decodes: mensagens aninhadas

decodes-you-transmit-s-everyone-else = Você transmite { $off } s { $after } os demais — o motivo habitual de chamadas sem resposta. Sincronize o relógio do seu computador.

## publicsdr: mensagens aninhadas

publicsdr-noise-floor-score = pontuação do piso de ruído { $snr }
publicsdr-up-khz-i-q = até { $khz } kHz de I/Q

## txaudio: mais mensagens

txaudio-gr-gate-de-ess-comp = GR gate { $gate_gr_db } de-esser { $deess_gr_db } comp { $comp_gr_db } lim { $limiter_gr_db }

## amp: mais mensagens

amp-h-step = ~{ $l_step } µH · passo { $l_step2 }
amp-pf-step = ~{ $c_step } pF · passo { $c_step2 }

## sstv: mais mensagens

sstv-bit-octets-chunks-first-pass = { $from } · { $filename } · { $width }×{ $height } { $bits_per_pixel }-bit · { $media_type } / { $content_encoding } · { $encoded_size } octetos em { $chunk_count } blocos ({ $chunks_first_pass } na primeira passagem) · sessão { $session }
sstv-baud-cpfsk-hz-khz-occupied = { $value } — { $symbol_rate } baud CPFSK, ±{ $deviation_hz } Hz, { $bandwidth_hz } kHz de largura ocupada

## Relógio, e alguns rótulos curtos com valores.

time-utc-and-local = { $utc } UTC · { $local } local
sat-in-seconds = em { $s } s
winlink-to-list = para { $to }
winlink-cc-list = cc { $cc }
spots-n-of-total = { $n } de { $total }
wefax-n-of-total = { $n } de { $total }
adsb-range-bearing = { $km } km a { $bearing }°
ais-range-bearing = { $km } km a { $bearing }°

## Mais valores de sdroxide-types, como os seletores os nomeiam (src/i18n/labels.rs).
rtl-sdr-hf-mode-off = Desligado (somente sintonizador, 24 MHz acima)
rtl-sdr-hf-mode-auto = Automático
rtl-sdr-hf-mode-direct-q = Amostragem direta (ramo Q)
rtl-sdr-agc-manual = Manual (sem AGC)
rtl-sdr-agc-tuner = AGC do sintonizador
rtl-sdr-agc-rtl = AGC digital do RTL
rtl-sdr-agc-both = AGC do sintonizador + RTL
mode-control-cat = CAT
mode-control-radio = Controlado pelo rádio
line-state-none = Nenhum
line-state-high = Alto
line-state-low = Baixo
elad-tx-input-usb-audio = Áudio USB
elad-tx-input-mic = Microfone
elad-tx-input-auto = Automático
elad-tx-input-leave = Deixar como ajustado no rádio
tr-usdx-audio-one-cable = Um cabo (áudio no fluxo CAT)
tr-usdx-audio-sound-card = Placa de som USB (conector de 3,5 mm)
spy-server-format-uint8 = 8 bits (menor largura de banda)
spy-server-format-int16 = 16 bits
spy-server-format-float32 = Ponto flutuante de 32 bits
sound-format-iq = IQ (estéreo)
sound-format-demod-audio = Áudio demodulado
sdr-play-duo-tuner-tuner1 = Sintonizador 1 (50 Ohm / Hi-Z)
sdr-play-duo-tuner-tuner2 = Sintonizador 2 (50 Ohm)
sdr-play-duo-role-diversity = Uma segunda antena (diversidade / supressão de QRM)
sdr-play-duo-role-second-radio = Um segundo rádio, na própria frequência
sdr-play-agc-off = Desligado (ganho de F.I. manual)
sdr-play-agc-hz5 = 5 Hz
sdr-play-agc-hz50 = 50 Hz
sdr-play-agc-hz100 = 100 Hz
rotator-protocol-rotctld = Hamlib rotctld (TCP)
rotator-protocol-pst-rotator = PSTRotator (UDP)
pluto-ptt-off = Desligado
pluto-ptt-gpo01 = GPO0 = RX, GPO1 = TX
pluto-ptt-gpo23 = GPO2 = RX, GPO3 = TX
pluto-duplex-fdd = FDD (os dois ao mesmo tempo)
pluto-duplex-tdd = TDD (um de cada vez)
pluto-agc-manual = Manual
pluto-agc-slow-attack = Ataque lento
pluto-agc-fast-attack = Ataque rápido
pluto-agc-hybrid = Híbrido
parity-none = Nenhuma
parity-even = Par
parity-odd = Ímpar
panadapter-tap-antenna = Antena (compartilhada com o rádio)
panadapter-tap-if-output = A saída de F.I. do rádio
panadapter-audio-attached = O receptor conectado
panadapter-audio-transceiver = O transceptor
kenwood-send-ts2000 = Estilo TS-2000 (TX;)
kenwood-send-ts590 = Estilo TS-590 (TX1;)
digi-mode-usb = USB
digi-mode-data = DIGI
digi-mode-radio = Controlado pelo rádio
cw-keying-cat = Manipulador do rádio (CAT)
cw-keying-audio = Placa de som (MCW)
audio-route-both = Os dois canais
audio-route-left = Somente esquerdo
audio-route-right = Somente direito
amp-model-hr500 = Hardrock-500
amp-model-hr50 = Hardrock-50 (não testado)
alert-sound-ding = Ding
alert-sound-two-tone = Dois tons
alert-sound-triplet = Tríplice
alert-sound-warble = Trinado
alert-sound-digital = Digital
alert-reply-tone = Tom
alert-reply-voice = Voz
alert-reply-both = Tom + voz
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
icom-model-other = Outro (definir o endereço manualmente)
backend-auto = Detecção automática (SoapySDR / CAT)
backend-soapy = SoapySDR
backend-cat = CAT / Áudio
backend-hpsdr = HPSDR (rede)
backend-tci = TCI (rede)
backend-icom-net = Icom LAN (rede)
backend-smart-sdr = SmartSDR / FlexRadio (rede)
backend-pluto = PlutoSDR (rede)
backend-rtl-sdr = RTL-SDR (USB)
backend-rtl-tcp = RTL-SDR por rtl_tcp (rede)
backend-spy-server = SpyServer (rede)
backend-spy-server-vfo = SpyServer VFO+FFT, baixa largura de banda (rede)
backend-kiwi-sdr = KiwiSDR / Web-888 (rede)
backend-rx888 = RX-888 (USB)
backend-airspy-hf = Airspy HF+ (USB)
backend-airspy = Airspy R2 / Mini (USB)
backend-hack-rf = HackRF One / Pro (USB)
backend-sdr-play = SDRplay RSP (USB)
backend-elad = ELAD FDM-DUO / FDM-S (USB)
backend-lime = LimeSDR + LimeRFE (LimeSuite)
backend-hydra-sdr = HydraSDR RFOne (USB)
backend-fobos = RigExpert Fobos SDR (USB)
backend-usb-audio = Rádio USB com áudio (placa de som)
backend-none = Não configurado
cat-family-xiegu = Xiegu
cat-family-icom = Icom
cat-family-yaesu = Yaesu
cat-family-kenwood = Kenwood
cat-family-elecraft = Elecraft
cat-family-elad = ELAD
cat-family-qrp-labs = QRP Labs
cat-family-rs-hfiq = RS-HFIQ
cat-family-tr-usdx = (tr)uSDX
cat-family-rigctld = Hamlib rigctld (rede)
cat-family-flrig = flrig (rede)

## Valores de sdroxide-types exibidos em chips, menus e listas (src/i18n/labels.rs).
agc-mode-off = Desligado
agc-mode-slow = Lento
agc-mode-med = Médio
agc-mode-fast = Rápido
airspy-gain-linearity = Linearidade (melhor para sinais fortes)
airspy-gain-sensitivity = Sensibilidade (melhor para sinais fracos)
ais-kind-class-a = Classe A
ais-kind-class-b = Classe B
ais-kind-base-station = estação-base
ais-kind-sar-aircraft = aeronave SAR
ais-kind-aid-to-navigation = auxílio à navegação
ais-kind-sart = SART / MOB / EPIRB
ais-kind-craft = embarcação associada
alert-event-called = Uma estação está me chamando
alert-event-cq = Um CQ a que eu poderia responder
alert-event-new-dxcc = Nova entidade DXCC
alert-event-new-dxcc-band = Nova entidade nesta faixa
alert-event-new-grid = Novo quadrado locator
audio-routing-both = Esquerdo + direito
audio-routing-left = Somente esquerdo
audio-routing-right = Somente direito
button-mode-momentary = Segurar
button-mode-toggle = Alternar
contest-mode-none = Nenhum
contest-mode-eu-vhf = EU VHF Contest
converter-tx-off = Desligado enquanto converte
converter-tx-transverter = Pelo mesmo conversor
converter-tx-own = Deslocamento próprio
cw-engine-neural = NEURAL
cw-engine-timing = TEMPO
decode-sort-none = Nenhum
decode-sort-signal = SNR
decode-sort-distance = Dist
decode-sort-country = País
diversity-mode-cancel = Cancelar — anula uma fonte de ruído
diversity-mode-combine = Combinar — recepção por diversidade
dxped-mode-normal = Normal
dxped-mode-hound = Hound
dxped-mode-fox = Fox
fail-safe-refuse-tx = Recusar transmitir
fail-safe-warn-only = Transmitir mesmo assim, avisar
ftp-security-plain = FTP (sem criptografia)
ftp-security-explicit-tls = FTPS explícito (AUTH TLS)
ftp-security-implicit-tls = FTPS implícito
hpsdr-filter-board-none = Nenhuma — as saídas ficam desligadas
hpsdr-filter-board-n2adr = Placa de filtro N2ADR
hpsdr-filter-board-alex = Código de banda Alex / Hermes (Zeus, HiQSDR, Quisk)
hpsdr-filter-board-custom = Personalizada — minha própria tabela abaixo
hpsdr-io-rx-input-radio = Entrada própria do rádio
hpsdr-io-rx-input-io-board = Placa IO J9
hpsdr-io-rx-input-io-board-pure-signal = Placa IO J9, PureSignal na transmissão
hydra-sdr-gain-linearity = Linearidade (melhor para sinais fortes)
hydra-sdr-gain-sensitivity = Sensibilidade (melhor para sinais fracos)
icom-rx-source-af = AF — o rádio demodula
icom-rx-source-if12k = F.I. de 12 kHz — o sdroxide demodula
icom-scope-span-radio = Como ajustado no rádio
icom-scope-span-khz2-5 = 5 kHz (±2,5k)
icom-scope-span-khz5 = 10 kHz (±5k)
icom-scope-span-khz10 = 20 kHz (±10k)
icom-scope-span-khz25 = 50 kHz (±25k)
icom-scope-span-khz50 = 100 kHz (±50k)
icom-scope-span-khz100 = 200 kHz (±100k)
icom-scope-span-khz250 = 500 kHz (±250k)
icom-scope-span-khz500 = 1 MHz (±500k)
ism-family-weather = CLIMA
ism-family-meter = MEDIDORES
ism-family-home-auto = CASA
ism-family-lo-ra = LORA
ism-family-unidentified = DESCONHECIDO
ism-protocol-la-crosse-it = LaCrosse IT+
ism-protocol-fine-offset = Fine Offset
ism-protocol-bresser = Bresser
ism-protocol-z-wave = Z-Wave
ism-protocol-homematic = Homematic
ism-protocol-rtl433 = rtl_433
ism-protocol-unidentified = não identificado
js8-speed-normal = NORMAL
js8-speed-fast = RÁPIDO
js8-speed-turbo = TURBO
js8-speed-slow = LENTO
lime-aux-role-off = Não usada
lime-aux-role-diversity = Uma segunda antena (diversidade / supressão de QRM)
lime-aux-role-pure-signal = Realimentação de transmissão (pré-distorção PureSignal)
lookup-provider-none = Desligado
lookup-provider-qrz = QRZ.com
lookup-provider-ham-qth = HamQTH
mail-folder-inbox = CAIXA DE ENTRADA
mail-folder-outbox = CAIXA DE SAÍDA
mail-folder-sent = ENVIADAS
mail-folder-archive = ARQUIVO
memory-sort-name = Nome
memory-sort-freq = Freq
memory-sort-band = Faixa
memory-sort-stored = Guardada
mouse-button-middle = Meio
mouse-button-extra1 = Voltar (extra 1)
mouse-button-extra2 = Avançar (extra 2)
nr-level-off = Desligado
nr-level-low = Baixo
nr-level-medium = Médio
nr-level-high = Alto
nr-level-rnn-low = RNN Baixo
nr-level-rnn-med = RNN Médio
nr-level-rnn-high = RNN Alto
nr-level-spec-low = SPEC Baixo
nr-level-spec-med = SPEC Médio
nr-level-spec-high = SPEC Alto
nr-level-nr2-low = NR2 Baixo
nr-level-nr2-med = NR2 Médio
nr-level-nr2-high = NR2 Alto
nr-level-df-low = DFNR Baixo
nr-level-df-med = DFNR Médio
nr-level-df-high = DFNR Alto
nr-strength-low = Baixa
nr-strength-med = Média
nr-strength-high = Alta
orbit-rings-none = nenhum
orbit-rings-curated = curados
orbit-rings-all = todos
qso-step-idle = Ocioso
qso-step-wait-cq = Aguardar CQ
qso-step-calling-cq = Chamando CQ
qso-step-tx-grid = TX Locator
qso-step-tx-report = TX Relatório
qso-step-tx-r-report = TX R+Relatório
qso-step-tx-rr73 = TX RR73
qso-step-tx73 = TX 73
qso-step-confirming = Confirmando
rds-standard-auto = Automático
rds-standard-rds = RDS
rds-standard-rbds = RBDS
region-r1 = Região 1 — Europa, África, Oriente Médio, Ásia do Norte
region-r2 = Região 2 — Américas
region-r3 = Região 3 — Sul e Leste da Ásia, Austrália e Oceania, Pacífico
relative-mode-absolute = Absoluto (fader)
relative-mode-sign-magnitude = Relativo: sinal/magnitude
relative-mode-twos-complement = Relativo: complemento de dois
relative-mode-binary-offset = Relativo: centrado em 64
relay-link-off = Nenhum
relay-link-serial = Placa de relé USB (serial)
relay-link-serial-lines = Linha serial RTS/DTR
relay-link-hid = Placa de relé USB (HID)
relay-link-cm108 = GPIO de placa de som CM108/CM119
relay-link-gpio = Linha GPIO do Linux
relay-link-command = Comando externo
relay-role-unused = Não usado
relay-role-sdr-antenna = Aterrar a antena do SDR
relay-role-amplifier = Acionar um amplificador / relé T-R
relay-role-aux = Auxiliar
relay-role-band-decoder = Decodificador de faixa (filtro / transverter)
rfe-channel-wb1000 = Banda larga 1 – 1000 MHz
rfe-channel-wb4000 = Banda larga 1 – 4 GHz
rfe-channel-ham0030 = HF, até 30 MHz
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
rfe-channel-not-selected = Nenhum selecionado
rfe-link-off = Não conectado
rfe-link-serial = Cabo USB próprio (serial)
rfe-link-board = Pelo LimeSDR (GPIO / I²C)
rfe-mode-control-auto = Automático (segue a fiação)
rfe-mode-control-rx = Sempre recebendo
rfe-mode-control-tx = Sempre transmitindo
rfe-mode-control-tx-rx = Sempre os dois (somente conectores separados)
rifp-encoding-auto = Automático
rifp-encoding-png = PNG
rifp-encoding-jpeg = JPEG
rifp-encoding-raw = Bruto
rifp-encoding-rle8 = RLE8
rifp-encoding-zlib = Zlib
rifp-encoding-group4 = G4
rifp-encoding-group3 = G3
scan-kind-memories = MEM
scan-kind-range = FAIXA
scan-kind-panorama = PANO
scan-resume-carrier = PORTADORA
scan-resume-timed = TEMPORIZADO
scan-resume-manual = MANUAL
sense-line-off = Não ligada
sense-line-cts = CTS
sense-line-dsr = DSR
sense-line-dcd = DCD (detecção de portadora)
shift-simplex = SIMPLEX
shift-minus = −
shift-plus = +
speed-slow = Lento
speed-medium = Médio
speed-fast = Rápido
speed-faster = Mais rápido
speed-fastest = Mais rápido ainda
tls-verify-system = Repositório de confiança do sistema
tls-verify-pinned = Impressão digital fixada
tls-verify-off = Não verificar
tone-mode-off = DESLIGADO
tone-mode-ctcss = CTCSS
tone-mode-dcs = DCS
tx-voice-stage-gate = Gate
tx-voice-stage-eq = EQ
tx-voice-stage-de-ess = De-esser
tx-voice-stage-comp = Compressor
upload-naming-sequence = image1 … imageN (rotativo)
upload-naming-original = Nome original do arquivo
wheel-action-zoom = Zoom
wheel-action-tune = Sintonizar
wheel-action-none = Nada

# Undocked windows (from sdroxide-brown)
frame-detached-title = { $flavor } — { $name }
frame-dock-band-menu-hint = ACOPLAR — devolve o seletor de banda e modo à janela principal, o mesmo que fechar esta sem procurar o botão de fechar.
frame-dock-panel-hint = ACOPLAR — coloca este painel de volta sob o waterfall
frame-dock-spectrum-hint = ACOPLAR — devolve este espectro à janela principal, o mesmo que fechar esta sem procurar o botão de fechar.
frame-every-module-own-window = cada módulo está em sua própria janela
frame-module-aux-panadapter = panadapter aux
frame-module-band-mode = banda e modo
frame-module-controls = controles
frame-module-panadapter = panadapter
frame-module-panel = painel
multi-dock-all-windows = ACOPLAR TODAS AS JANELAS
multi-module-panel = Painel de operação
multi-undock-all-windows = DESACOPLAR TODAS AS JANELAS
settings-ui-dock-all-hint = Traz todas as janelas desacopladas de volta para esta.
settings-ui-docked = Acoplado nesta janela
settings-ui-module-aux = AUX SP (segundo espectro)
settings-ui-module-band-menu = Seletor de banda e modo
settings-ui-module-controls = Controles
settings-ui-module-panadapter = Panadapter
settings-ui-undock-all-hint = Coloca cada módulo em uma janela própria — a janela principal fica com o espectro e os decodificadores. Um módulo sem nada a mostrar, como o painel de operação em um modo de voz, não abre janela.
settings-ui-undock-hint =
    Desacoplado, este módulo vai para uma janela própria e a janela principal devolve o espaço ao que sobra — ponha o waterfall em um segundo monitor, ou os controles em um terceiro. Fechar a janela o acopla de novo.

    No Wayland o tamanho volta, mas o compositor decide onde uma janela fica: deixe-a flutuante e fixe-a em um monitor com uma regra de janela que corresponda ao app-id "{ $app_id }".
settings-ui-undocked = Desacoplado — janela própria
settings-ui-window-label = Janela { $module }
frame-window-chip-hint = JANELA — move esta janela para uma janela própria do sistema, para colocá-la em outro monitor
frame-dock-panel-pane-hint = ACOPLAR — devolve este painel ao painel abaixo do waterfall
frame-panel-window-title = { $mode } — painel
frame-roll-up-hint = Recolher — sai da barra de título e devolve a janela ao waterfall
frame-unroll-hint = Expandir — mostra o painel de novo
frame-window-panel-hint = JANELA — dá a este painel uma janela própria, para colocar em outro monitor
topbar-dock-all = ACOPLAR TUDO
topbar-dock-all-hint = Traz todas as janelas de módulos de volta a esta janela
topbar-module-windows-title = Janelas de módulos
topbar-undock-all = DESACOPLAR TUDO
topbar-undock-all-hint = Coloca cada módulo em uma janela própria — esta janela fica com o espectro e os decodificadores
topbar-windows-chip-hint =
    ▣  Esquerdo: traz todas as janelas de módulos para casa
    Direito: a lista de janelas de módulos
topbar-wname-aux = SP2 (AUX) — um segundo espectro
topbar-wname-band-menu = Banda e modo
topbar-wname-controls = Controle RX
topbar-wname-panadapter = SP1 — o espectro
topbar-wname-panel = Painel de operação
topbar-wstate-float = Tirar { $name } do painel e deixá-lo flutuando sobre esta janela
topbar-wstate-floating-panel-only = Flutuar é um estado próprio do painel de operação — um espectro ou um console tem um só lugar nesta janela
topbar-wstate-here-now = { $name } está aqui agora
topbar-wstate-no-panel = Sem painel de operação em { $mode } — isto vale em um modo que tenha um
topbar-wstate-own-window = Dar a { $name } uma janela própria, em qualquer monitor
topbar-wstate-put-back = Colocar { $name } de volta nesta janela
frame-workspace-missing = Nenhuma área de trabalho { $n } salva — salve uma em Configurações › UI › Áreas de trabalho e esta tecla a chamará de volta.
settings-ui-workspaces = Áreas de trabalho
settings-ui-workspaces-note = Uma disposição de janelas com nome — quais módulos estão em janelas próprias e onde essas janelas ficam. Salve a que você tem, ou restaure uma salva. Até dez.
settings-ui-workspaces-none = Nenhuma área de trabalho salva ainda.
settings-ui-workspace-restore-home = Restaurar esta disposição — todos os módulos na janela principal
settings-ui-workspace-restore-out = Restaurar esta disposição — { $n } módulo(s) em janela(s) própria(s)
settings-ui-workspace-forget = Esquecer esta área de trabalho
settings-ui-workspace-name = dê um nome a esta disposição
settings-ui-workspace-full = Dez áreas de trabalho é o limite que o SDRuno guarda; apague uma primeiro
settings-ui-workspace-save = SALVAR ATUAL
settings-ui-workspace-save-hint = Salva a disposição atual com este nome
settings-ui-workspace-name-first = Digite um nome primeiro
topbar-band-window2-hint = JANELA — abre o seletor em uma janela própria, tão larga quanto você quiser, em qualquer monitor
tnc-link = ENLACE
tnc-connected = CONECTADO
tnc-calling = CHAMANDO
tnc-idle = ESCUTANDO
tnc-tx = TX
tnc-busy = OCUPADO
tnc-busy-hint = Há outra estação no canal. Uma chamada agora cairia por cima dela.
tnc-setup = ⚙ CONFIG
tnc-peer = Correspondente
tnc-width = Largura
tnc-speed = Velocidade
tnc-snr = S/R
tnc-bytes = Bytes
tnc-queued = Na fila
tnc-host = Host
tnc-ports = Portas
tnc-ports-off = desligadas
tnc-nobody = ninguém
tnc-holder-panel = este painel
tnc-holder-winlink = a janela MAIL
tnc-holder-tcp = um programa em { $addr }
tnc-tune = SINTONIA
tnc-abort = ABORTAR
tnc-abort-hint = Derrubar o enlace agora, quem quer que o esteja conduzindo. A outra ponta vai esgotar o tempo em vez de ser avisada.
tnc-terminal = TERMINAL
tnc-callsign = indicativo
tnc-connect = CONECTAR
tnc-disconnect = DESCONECTAR
tnc-disconnect-hint = Desligar direito: o que está na fila sai primeiro, depois a outra ponta concorda.
tnc-held-by = O modem está em uso por { $holder }.
tnc-no-callsign = Defina um indicativo primeiro (⚙ CONFIG)
tnc-clear = LIMPAR
tnc-clear-hint = Esvaziar a transcrição. O enlace não é afetado.
tnc-nothing-yet = Nada ainda. Digite um indicativo acima e CONECTAR, ou ligue Escutar em ⚙ CONFIG para atender chamadas.
tnc-type-here = digite aqui, Enter envia
tnc-not-connected = não conectado
tnc-send = ENVIAR
setup-tnc-bandwidth = Largura de banda
setup-tnc-listen = Atender chamadas
setup-tnc-listen-check = Escutar pelo painel
setup-tnc-listen-hint = Atender chamadas recebidas no terminal do painel. Sem efeito enquanto as portas TCP do host estiverem ligadas, pois um programa conectado ali quer as chamadas.
setup-tnc-host = Host TCP
playback-radio-n = { " " }· rádio { $n }
playback-no-stamp = sem carimbo no nome — toca no modo em que você está
playback-title = REPRODUÇÃO — áudio gravado
playback-find-hint = modo, MHz, data, nome…
playback-refresh-hint = Ler a pasta de gravações de novo
playback-asking = Pedindo à estação suas gravações…
playback-folder = Pasta: { $dir }
playback-none = Nenhuma gravação ainda. Pressione REC → MP3 enquanto algo estiver no ar; o arquivo aparece aqui, nomeado com hora, frequência e modo.
playback-on = ▶ LIGADO
playback-play = ▶ TOCAR
playback-iq-hint = Uma captura I/Q, não áudio. A reprodução usa o áudio que o gravador escreve; uma captura I/Q é tocada com --file.
playback-not-while-tx = Não durante a transmissão
playback-play-hint = Tocar esta gravação nos decodificadores desde o início
playback-busy-playing = Reproduzindo — pare antes de excluir
playback-busy-recording = Ainda sendo gravada — pare o REC antes de excluir
playback-delete-ask = EXCLUIR?
playback-delete-hint-armed = Pressione de novo para excluir este arquivo definitivamente. Não pode ser desfeito.
playback-delete-hint = Excluir esta gravação da pasta (pergunta mais uma vez antes)
playback-strip-label = ▶ REPRODUÇÃO
playback-not-live = fora do ar — o rádio está em espera
playback-carry-on = Continuar reproduzindo
playback-pause = ⏸ PAUSAR
playback-pause-hint = Pausar — os decodificadores ouvem silêncio até você continuar
playback-again = ↺ REPETIR
playback-again-hint = Volta ao início — outra tentativa em uma imagem ou decodificação
playback-loop = LOOP
playback-loop-hint = Recomeçar do início quando o arquivo terminar
playback-stop = ■ PARAR
playback-stop-hint = Encerrar a reprodução e devolver o rádio ao que estava
playback-files = ARQUIVOS
playback-end-of-file = fim do arquivo
playback-paused = pausado
playback-recorded-on-mode = Gravado em { $mhz } MHz em { $mode } — o dial e o modo foram ajustados a partir do nome do arquivo.
playback-recorded-on = Gravado em { $mhz } MHz — o dial e o modo foram ajustados a partir do nome do arquivo.
playback-no-dial-mode = O nome não registra dial nem modo, então toca no modo em que você está — escolha o decodificador no menu de modo.
playback-format-stereo = { $rate } Hz, estéreo — o canal esquerdo (recepção) é reproduzido.
playback-format-mono = { $rate } Hz, mono — o único canal é reproduzido.
playback-stopped = Parado: { $fault }
playback-intro = Reproduz uma gravação nos decodificadores — SSTV, CW, RTTY, PSK, FT8 e todos os outros modos que decodificam a partir de áudio. O rádio fica em espera enquanto toca, o dial e o modo vão para o que a gravação usou, e PARAR devolve o rádio. Mude o modo durante a reprodução para tentar outro decodificador.
topbar-stereo-yields-hint = O estéreo WFM fica desligado enquanto a redução de ruído ou o auto-notch estão ativos — eles borrariam os dois canais. Desligue-os para ouvir estéreo
topbar-stereo-playback-hint = Estéreo: a gravação está sendo reproduzida nos dois ouvidos, como gravada. Clique para forçar mono
topbar-stereo-mono-recording-hint = Esta gravação é mono — não há segundo canal para reproduzir
topbar-caption-play-back = Reproduzir
topbar-recordings-chip = ▶ GRAVAÇÕES…
topbar-recordings-hint = Reproduz uma gravação de volta nos decodificadores — uma imagem SSTV perdida, um sinal RTTY ou CW fraco, um slot de FT8 — com o waterfall e o log de decodificação, como se estivesse no ar. O rádio fica em espera enquanto ela toca.
playback-find = Buscar
playback-refresh = ATUALIZAR
wxsat-title = SATÉLITES METEOROLÓGICOS
sat-wxsat-hint = SATÉLITES METEOROLÓGICOS — a imagem que um satélite NOAA (APT) ou Meteor (LRPT) envia em 137 MHz, desenhada à medida que chega, em uma janela que pode ser destacada
sat-wxsat = SATÉLITES METEOROLÓGICOS
wxsat-decode = Decodificar
wxsat-channel = Satélite
wxsat-tune = Sintonizar aqui
wxsat-tune-hint = Liga o decodificador e leva o dial para este canal, para que o sinal fique dentro da janela de amostragem.
wxsat-clear = Limpar
wxsat-save = Salvar PNG
wxsat-flip = Girar
wxsat-flip-hint = Vira a imagem: um satélite voando para o sul é desenhado de cabeça para baixo.
wxsat-state-off = Decodificador desligado
wxsat-level = Nível { $db } dBFS
wxsat-locked = Sincronismo de linha travado
wxsat-searching = Procurando o sincronismo de linha
wxsat-lines = { $n } linhas
wxsat-outside-window = O canal está fora da janela de amostragem, ou perto demais da borda para o sinal inteiro. Use Sintonizar aqui, ou uma janela mais larga.
wxsat-waiting = Ainda sem imagem. Ligue o decodificador quando um satélite estiver acima do horizonte; uma passagem dura cerca de dez minutos.
wxsat-signal = Sinal
wxsat-kind-apt = APT
wxsat-kind-lrpt = LRPT
wxsat-image-channel = Canal { $n }
wxsat-rows = { $n } fileiras
wxsat-frame-locked = Sincronismo de quadro travado
wxsat-frames = { $n } quadros bons
wxsat-waiting-lrpt = Ainda sem imagem. O LRPT do Meteor é digital: a imagem começa alguns segundos depois de ouvir o satélite, fileira por fileira, e cada canal é uma imagem separada.
wxsat-searching-frame = Procurando o sincronismo de quadro
sondemap-no-sonde-yet = Nenhuma radiossonda recebida ainda
sonde-decoding = DECODIFICANDO
sonde-decoding-off = DECODIFICADOR DESLIGADO
sonde-count = Sondas: { $n }
sonde-frames = Quadros: { $n }
sonde-channel = Canal
sonde-channel-hover = A frequência da sonda a escutar (400–406 MHz). O rádio também é sintonizado ali, para que o canal fique dentro da janela.
sonde-col-serial = Sonda
sonde-col-alt = Altitude
sonde-col-climb = Subida
sonde-col-speed = Velocidade
sonde-col-battery = Bateria
sonde-col-heard = Recebida
sonde-scan = Varredura
sonde-scan-hover = Procura sondas em toda a janela de amostragem e decodifica até quatro ao mesmo tempo, em vez de escutar um só canal. A janela precisa cobri-las: o campo Canal centraliza o rádio.
sonde-online = Online (SondeHub)
sonde-online-hover = Mostra também as sondas ouvidas por outras estações, da lista pública do SondeHub. Apenas leitura: seu locator arredondado a 0,1° e um raio de 400 km são enviados; nada é publicado. Desligado até você ativar.
sonde-online-count = { $n } online
sonde-online-no-grid = Defina seu locator na configuração dos modos digitais para ver as sondas online.
sonde-online-failed = O SondeHub não respondeu.
sonde-export = Exportar o voo
sonde-export-kml-hover = Salvar o voo desta sonda como trilha KML (Google Earth): hora, longitude, latitude, altitude em metros
sonde-export-csv-hover = Salvar o voo desta sonda como tabela CSV para uma planilha: hora, latitude, longitude, altitude em metros
sonde-export-none = Nenhuma posição guardada ainda para esta sonda.
sonde-col-temp = Temp.
sonde-col-humidity = Umidade
sonde-col-pressure = Pressão
sonde-online-why-signin = A estação exige login primeiro.
sonde-online-why-busy = A estação já consulta o SondeHub tão seguido quanto pode. Tente de novo em um minuto.
sonde-online-why-upstream = A estação não conseguiu alcançar o SondeHub.
sonde-online-why-unreadable = A resposta não era uma lista de sondas.
sonde-online-why-no-station = Nenhuma estação para consultar, e esta página não alcança o SondeHub sozinha.
sonde-online-why-http = Resposta: HTTP { $n }.
sonde-card-source = Origem
sonde-card-source-radio = Este rádio
sonde-card-source-hub = SondeHub (ouvida por outra estação)
sonde-card-model = Modelo
sonde-card-maker = Fabricante
sonde-card-variant = Variante
sonde-card-position = Posição
sonde-card-from-you = De você
sonde-card-heading = Rumo
sonde-card-satellites = Satélites
sonde-card-snr = Relação sinal/ruído
sonde-card-station = Ouvida por
sonde-card-station-position = Posição da estação
sonde-card-antenna = Antena
sonde-card-software = Software
sonde-card-frames = Quadros recebidos
sonde-card-open = Abrir no { $site }
sonde-card-close = Fechar
sonde-card-source-both = Rádio + SondeHub (ambos a recebem)
sonde-card-stale = Estado
sonde-card-stale-value = Fora da lista: última posição conhecida
sonde-card-detach = Destacar
sonde-card-attach = Fixar
sonde-card-listen-hover = Clique para sintonizar o rádio neste canal: o dial vai para lá e o decodificador escuta só ele.
sonde-card-decoding = Decodificada por este rádio
sonde-card-waiting = Escutando neste canal, nada decodificado ainda
sonde-card-click-to-listen = Clique na frequência para ouvir
sonde-type-all = Todos os tipos
sonde-type-hover = Procurar um tipo de sonda: escolher um inicia a varredura, que move a janela pela faixa até ouvir esse tipo. Só esse tipo é listado e desenhado.
sonde-export-saved = { $name } salvo.

## Keyboard modes: MT63 (src/app/panels/text_modem.rs)
textmodem-mt63-long-hint = MT63-{ $label }, { $bw } Hz de largura — o entrelaçamento longo espalha uma rajada por { $chars } caracteres
textmodem-mt63-short-hint = MT63-{ $label }, { $bw } Hz de largura, entrelaçamento curto — o entrelaçamento curto espalha uma rajada por { $chars } caracteres
textmodem-mt63-not-offered = 1K e 500L ainda não disponíveis
textmodem-mt63-not-offered-hint = O MT63-1K e o entrelaçamento longo de 500 Hz são lidos pelo decodificador em lote, mas ainda não pelo receptor ao vivo: o travamento procura a fase do símbolo e a polaridade, mas não o centro da banda, e uma busca que o faça trocava uma variante por outra.

## Keyboard modes: MFSK (src/app/panels/text_modem.rs)
textmodem-mfsk-hint = { $label } — { $baud } baud, { $tones } tons, { $bw } Hz de largura{ $caveat }
textmodem-mfsk-reverse-hint = Inverte a ordem dos tons. Uma estação recebida na banda lateral oposta àquela em que foi transmitida chega espelhada em torno do seu centro e soa como ruído até isto ser ativado.

## Keyboard modes: DominoEX (src/app/panels/text_modem.rs)
textmodem-domino-hint = { $name } — { $baud } baud, 18 tons, modulação incremental de frequência sem correção de erros direta
textmodem-ifk-reverse-hint = Inverte os passos de tom. O IFK+ leva seus dados no passo entre tons sucessivos, então uma estação recebida na banda lateral oposta tem todos os passos negativos e soa como ruído até isto ser ativado.

## Keyboard modes: PSK families (src/app/panels/text_modem.rs)
textmodem-psk-family = Família
textmodem-psk-family-plain-hint = A família própria do PSK31: uma portadora diferencial, sem correção de erros direta
textmodem-psk-family-bpsk63f-hint = O código robusto a 62,5 baud, sem entrelaçador — o único modo codificado que o fldigi envia sem um
textmodem-psk-family-robust-hint = Uma portadora com o código robusto e um entrelaçador 2x2xN
textmodem-psk-family-multi-hint = Várias portadoras, varicode do PSK31 e **sem** código — mais largo e mais rápido, e exige um sinal limpo
textmodem-psk-family-robust-multi-hint = Várias portadoras **e** o código robusto: o mais largo e o mais robusto dos modos PSK
textmodem-psk-baud-fast-hint = { $baud } baud — lê a partir de cerca de um segundo: nesta taxa, o tempo de travamento do receptor é um terço de uma mensagem
textmodem-psk-baud-hint = { $baud } baud
textmodem-psk-bpsk-hint = Um bit por símbolo, detectado diferencialmente
textmodem-psk-coded = codificado
textmodem-psk-no-code = sem código
textmodem-psk-coded-hint =
    { $baud } baud, { $n } { $n ->
            [one] portadora
           *[other] portadoras
        }, { $code }, { $bw } Hz de largura
