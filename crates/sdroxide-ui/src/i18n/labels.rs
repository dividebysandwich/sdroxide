//! The screen's words for values defined in `sdroxide-types`.
//!
//! Those types carry an English `label()` of their own, and it stays: it is
//! what a log, a config file or the engine reads, and it must not change with
//! the operator's language. What the interface draws comes from here instead.
//! Each is an exhaustive `match`, so a variant added to a type is a build
//! error here until it has a message.

use sdroxide_types::{
    AlertReply, AlertSound, AmpModel, AudioRoute, Backend, BandplanKind, CallsignStyle, CatFamily,
    ChromeStyle, CwKeying, DigiMode, EladTxInput, FontSize, FreqStyle, IcomModel, KenwoodSend,
    LayoutMode, LineState, ModeControl, PanadapterAudio, PanadapterTap, Parity, PlutoAgc,
    PlutoDuplex, PlutoPtt, RotatorProtocol, RtlSdrAgc, RtlSdrHfMode, SdrPlayAgc, SdrPlayDuoRole,
    SdrPlayDuoTuner, SoundFormat, SpyServerFormat, TrUsdxAudio, UiTheme, Verbosity,
};

use sdroxide_types::{
    AgcMode, AirspyGain, AisKind, AlertEvent, AudioRouting, ButtonMode, ContestMode, ConverterTx,
    CwEngine, DecodeSort, DiversityMode, DxpedMode, FailSafe, FtpSecurity, HpsdrFilterBoard,
    HpsdrIoRxInput, HydraSdrGain, IcomRxSource, IcomScopeSpan, IsmFamily, IsmProtocol, Js8Speed,
    LimeAuxRole, LookupProvider, MailFolder, MemorySort, MouseButton, NrLevel, NrStrength,
    OrbitRings, QsoStep, RdsStandard, Region, RelativeMode, RelayLink, RelayRole, RfeChannel,
    RfeLink, RfeModeControl, RifpEncoding, ScanKind, ScanResume, SenseLine, Shift, Speed,
    TlsVerify, ToneMode, TxVoiceStage, UploadNaming, WheelAction,
};

use super::tr;

/// A value's name, in the interface's language.
pub(crate) trait Tr: Copy {
    fn tr(self) -> String;
}

impl Tr for LayoutMode {
    fn tr(self) -> String {
        match self {
            LayoutMode::Auto => tr!("layout-mode-auto"),
            LayoutMode::Desktop => tr!("layout-mode-desktop"),
            LayoutMode::Tablet => tr!("layout-mode-tablet"),
            LayoutMode::Small => tr!("layout-mode-small"),
            LayoutMode::Phone => tr!("layout-mode-phone"),
        }
    }
}

impl Tr for UiTheme {
    fn tr(self) -> String {
        match self {
            UiTheme::Default => tr!("ui-theme-default"),
            UiTheme::Light => tr!("ui-theme-light"),
            UiTheme::HighContrast => tr!("ui-theme-high-contrast"),
            UiTheme::GreenPhosphor => tr!("ui-theme-green-phosphor"),
            UiTheme::AmberPhosphor => tr!("ui-theme-amber-phosphor"),
            UiTheme::TealOrange => tr!("ui-theme-teal-orange"),
            UiTheme::Rainbow => tr!("ui-theme-rainbow"),
            UiTheme::Nord => tr!("ui-theme-nord"),
            UiTheme::NordDark => tr!("ui-theme-nord-dark"),
            UiTheme::Gruvbox => tr!("ui-theme-gruvbox"),
            UiTheme::Everforest => tr!("ui-theme-everforest"),
            UiTheme::SolarizedDark => tr!("ui-theme-solarized-dark"),
            UiTheme::Solarized => tr!("ui-theme-solarized"),
            UiTheme::Dracula => tr!("ui-theme-dracula"),
            UiTheme::CatppuccinMocha => tr!("ui-theme-catppuccin-mocha"),
            UiTheme::CatppuccinLatte => tr!("ui-theme-catppuccin-latte"),
            UiTheme::ModernMinimalist => tr!("ui-theme-modern-minimalist"),
        }
    }
}

impl Tr for ChromeStyle {
    fn tr(self) -> String {
        match self {
            ChromeStyle::Angled => tr!("chrome-style-angled"),
            ChromeStyle::Rectangular => tr!("chrome-style-rectangular"),
            ChromeStyle::Rounded => tr!("chrome-style-rounded"),
            ChromeStyle::Gradient => tr!("chrome-style-gradient"),
            ChromeStyle::Bevel => tr!("chrome-style-bevel"),
            ChromeStyle::Terminal => tr!("chrome-style-terminal"),
        }
    }
}

impl Tr for FontSize {
    fn tr(self) -> String {
        match self {
            FontSize::Small => tr!("font-size-small"),
            FontSize::Medium => tr!("font-size-medium"),
            FontSize::Large => tr!("font-size-large"),
        }
    }
}

impl Tr for BandplanKind {
    fn tr(self) -> String {
        match self {
            BandplanKind::Ham => tr!("bandplan-kind-ham"),
            BandplanKind::Cw => tr!("bandplan-kind-cw"),
            BandplanKind::Digi => tr!("bandplan-kind-digi"),
            BandplanKind::Phone => tr!("bandplan-kind-phone"),
            BandplanKind::Beacon => tr!("bandplan-kind-beacon"),
            BandplanKind::Broadcast => tr!("bandplan-kind-broadcast"),
            BandplanKind::Am => tr!("bandplan-kind-am"),
            BandplanKind::Cb => tr!("bandplan-kind-cb"),
        }
    }
}

impl Tr for Verbosity {
    fn tr(self) -> String {
        match self {
            Verbosity::Terse => tr!("speech-verbosity-terse"),
            Verbosity::Normal => tr!("speech-verbosity-normal"),
            Verbosity::Full => tr!("speech-verbosity-full"),
        }
    }
}

impl Tr for FreqStyle {
    fn tr(self) -> String {
        match self {
            FreqStyle::Dial => tr!("speech-freq-style-dial"),
            FreqStyle::Digits => tr!("speech-freq-style-digits"),
            FreqStyle::Full => tr!("speech-freq-style-full"),
        }
    }
}

impl Tr for CallsignStyle {
    fn tr(self) -> String {
        match self {
            CallsignStyle::Phonetic => tr!("speech-callsign-style-phonetic"),
            CallsignStyle::Letters => tr!("speech-callsign-style-letters"),
            CallsignStyle::AsIs => tr!("speech-callsign-style-as-is"),
        }
    }
}

impl Tr for RtlSdrHfMode {
    fn tr(self) -> String {
        match self {
            RtlSdrHfMode::Off => tr!("rtl-sdr-hf-mode-off"),
            RtlSdrHfMode::Auto => tr!("rtl-sdr-hf-mode-auto"),
            RtlSdrHfMode::DirectQ => tr!("rtl-sdr-hf-mode-direct-q"),
        }
    }
}

impl Tr for RtlSdrAgc {
    fn tr(self) -> String {
        match self {
            RtlSdrAgc::Manual => tr!("rtl-sdr-agc-manual"),
            RtlSdrAgc::Tuner => tr!("rtl-sdr-agc-tuner"),
            RtlSdrAgc::Rtl => tr!("rtl-sdr-agc-rtl"),
            RtlSdrAgc::Both => tr!("rtl-sdr-agc-both"),
        }
    }
}

impl Tr for ModeControl {
    fn tr(self) -> String {
        match self {
            ModeControl::Cat => tr!("mode-control-cat"),
            ModeControl::Radio => tr!("mode-control-radio"),
        }
    }
}

impl Tr for LineState {
    fn tr(self) -> String {
        match self {
            LineState::None => tr!("line-state-none"),
            LineState::High => tr!("line-state-high"),
            LineState::Low => tr!("line-state-low"),
        }
    }
}

impl Tr for EladTxInput {
    fn tr(self) -> String {
        match self {
            EladTxInput::UsbAudio => tr!("elad-tx-input-usb-audio"),
            EladTxInput::Mic => tr!("elad-tx-input-mic"),
            EladTxInput::Auto => tr!("elad-tx-input-auto"),
            EladTxInput::Leave => tr!("elad-tx-input-leave"),
        }
    }
}

impl Tr for TrUsdxAudio {
    fn tr(self) -> String {
        match self {
            TrUsdxAudio::OneCable => tr!("tr-usdx-audio-one-cable"),
            TrUsdxAudio::SoundCard => tr!("tr-usdx-audio-sound-card"),
        }
    }
}

impl Tr for SpyServerFormat {
    fn tr(self) -> String {
        match self {
            SpyServerFormat::Uint8 => tr!("spy-server-format-uint8"),
            SpyServerFormat::Int16 => tr!("spy-server-format-int16"),
            SpyServerFormat::Float32 => tr!("spy-server-format-float32"),
        }
    }
}

impl Tr for SoundFormat {
    fn tr(self) -> String {
        match self {
            SoundFormat::Iq => tr!("sound-format-iq"),
            SoundFormat::DemodAudio => tr!("sound-format-demod-audio"),
        }
    }
}

impl Tr for SdrPlayDuoTuner {
    fn tr(self) -> String {
        match self {
            SdrPlayDuoTuner::Tuner1 => tr!("sdr-play-duo-tuner-tuner1"),
            SdrPlayDuoTuner::Tuner2 => tr!("sdr-play-duo-tuner-tuner2"),
        }
    }
}

impl Tr for SdrPlayDuoRole {
    fn tr(self) -> String {
        match self {
            SdrPlayDuoRole::Diversity => tr!("sdr-play-duo-role-diversity"),
            SdrPlayDuoRole::SecondRadio => tr!("sdr-play-duo-role-second-radio"),
        }
    }
}

impl Tr for SdrPlayAgc {
    fn tr(self) -> String {
        match self {
            SdrPlayAgc::Off => tr!("sdr-play-agc-off"),
            SdrPlayAgc::Hz5 => tr!("sdr-play-agc-hz5"),
            SdrPlayAgc::Hz50 => tr!("sdr-play-agc-hz50"),
            SdrPlayAgc::Hz100 => tr!("sdr-play-agc-hz100"),
        }
    }
}

impl Tr for RotatorProtocol {
    fn tr(self) -> String {
        match self {
            RotatorProtocol::Rotctld => tr!("rotator-protocol-rotctld"),
            RotatorProtocol::PstRotator => tr!("rotator-protocol-pst-rotator"),
        }
    }
}

impl Tr for PlutoPtt {
    fn tr(self) -> String {
        match self {
            PlutoPtt::Off => tr!("pluto-ptt-off"),
            PlutoPtt::Gpo01 => tr!("pluto-ptt-gpo01"),
            PlutoPtt::Gpo23 => tr!("pluto-ptt-gpo23"),
        }
    }
}

impl Tr for PlutoDuplex {
    fn tr(self) -> String {
        match self {
            PlutoDuplex::Fdd => tr!("pluto-duplex-fdd"),
            PlutoDuplex::Tdd => tr!("pluto-duplex-tdd"),
        }
    }
}

impl Tr for PlutoAgc {
    fn tr(self) -> String {
        match self {
            PlutoAgc::Manual => tr!("pluto-agc-manual"),
            PlutoAgc::SlowAttack => tr!("pluto-agc-slow-attack"),
            PlutoAgc::FastAttack => tr!("pluto-agc-fast-attack"),
            PlutoAgc::Hybrid => tr!("pluto-agc-hybrid"),
        }
    }
}

impl Tr for Parity {
    fn tr(self) -> String {
        match self {
            Parity::None => tr!("parity-none"),
            Parity::Even => tr!("parity-even"),
            Parity::Odd => tr!("parity-odd"),
        }
    }
}

impl Tr for PanadapterTap {
    fn tr(self) -> String {
        match self {
            PanadapterTap::Antenna => tr!("panadapter-tap-antenna"),
            PanadapterTap::IfOutput => tr!("panadapter-tap-if-output"),
        }
    }
}

impl Tr for PanadapterAudio {
    fn tr(self) -> String {
        match self {
            PanadapterAudio::Attached => tr!("panadapter-audio-attached"),
            PanadapterAudio::Transceiver => tr!("panadapter-audio-transceiver"),
        }
    }
}

impl Tr for KenwoodSend {
    fn tr(self) -> String {
        match self {
            KenwoodSend::Ts2000 => tr!("kenwood-send-ts2000"),
            KenwoodSend::Ts590 => tr!("kenwood-send-ts590"),
        }
    }
}

impl Tr for DigiMode {
    fn tr(self) -> String {
        match self {
            DigiMode::Usb => tr!("digi-mode-usb"),
            DigiMode::Data => tr!("digi-mode-data"),
            DigiMode::Radio => tr!("digi-mode-radio"),
        }
    }
}

impl Tr for CwKeying {
    fn tr(self) -> String {
        match self {
            CwKeying::Cat => tr!("cw-keying-cat"),
            CwKeying::Audio => tr!("cw-keying-audio"),
        }
    }
}

impl Tr for AudioRoute {
    fn tr(self) -> String {
        match self {
            AudioRoute::Both => tr!("audio-route-both"),
            AudioRoute::Left => tr!("audio-route-left"),
            AudioRoute::Right => tr!("audio-route-right"),
        }
    }
}

impl Tr for AmpModel {
    fn tr(self) -> String {
        match self {
            AmpModel::Hr500 => tr!("amp-model-hr500"),
            AmpModel::Hr50 => tr!("amp-model-hr50"),
        }
    }
}

impl Tr for AlertSound {
    fn tr(self) -> String {
        match self {
            AlertSound::Ding => tr!("alert-sound-ding"),
            AlertSound::TwoTone => tr!("alert-sound-two-tone"),
            AlertSound::Triplet => tr!("alert-sound-triplet"),
            AlertSound::Warble => tr!("alert-sound-warble"),
            AlertSound::Digital => tr!("alert-sound-digital"),
        }
    }
}

impl Tr for AlertReply {
    fn tr(self) -> String {
        match self {
            AlertReply::Tone => tr!("alert-reply-tone"),
            AlertReply::Voice => tr!("alert-reply-voice"),
            AlertReply::Both => tr!("alert-reply-both"),
        }
    }
}

impl Tr for IcomModel {
    fn tr(self) -> String {
        match self {
            IcomModel::Ic7300 => tr!("icom-model-ic7300"),
            IcomModel::Ic7300Mk2 => tr!("icom-model-ic7300-mk2"),
            IcomModel::Ic705 => tr!("icom-model-ic705"),
            IcomModel::Ic7760 => tr!("icom-model-ic7760"),
            IcomModel::Ic905 => tr!("icom-model-ic905"),
            IcomModel::Ic9700 => tr!("icom-model-ic9700"),
            IcomModel::Ic7610 => tr!("icom-model-ic7610"),
            IcomModel::Ic7100 => tr!("icom-model-ic7100"),
            IcomModel::Ic7410 => tr!("icom-model-ic7410"),
            IcomModel::Ic7600 => tr!("icom-model-ic7600"),
            IcomModel::Ic7700 => tr!("icom-model-ic7700"),
            IcomModel::Ic7800 => tr!("icom-model-ic7800"),
            IcomModel::Ic9100 => tr!("icom-model-ic9100"),
            IcomModel::Ic7200 => tr!("icom-model-ic7200"),
            IcomModel::Ic7000 => tr!("icom-model-ic7000"),
            IcomModel::Other => tr!("icom-model-other"),
        }
    }
}

impl Tr for Backend {
    fn tr(self) -> String {
        match self {
            Backend::Auto => tr!("backend-auto"),
            Backend::Soapy => tr!("backend-soapy"),
            Backend::Cat => tr!("backend-cat"),
            Backend::Hpsdr => tr!("backend-hpsdr"),
            Backend::Tci => tr!("backend-tci"),
            Backend::IcomNet => tr!("backend-icom-net"),
            Backend::SmartSdr => tr!("backend-smart-sdr"),
            Backend::Pluto => tr!("backend-pluto"),
            Backend::RtlSdr => tr!("backend-rtl-sdr"),
            Backend::RtlTcp => tr!("backend-rtl-tcp"),
            Backend::SpyServer => tr!("backend-spy-server"),
            Backend::SpyServerVfo => tr!("backend-spy-server-vfo"),
            Backend::KiwiSdr => tr!("backend-kiwi-sdr"),
            Backend::Rx888 => tr!("backend-rx888"),
            Backend::AirspyHf => tr!("backend-airspy-hf"),
            Backend::Airspy => tr!("backend-airspy"),
            Backend::HackRf => tr!("backend-hack-rf"),
            Backend::SdrPlay => tr!("backend-sdr-play"),
            Backend::Elad => tr!("backend-elad"),
            Backend::Lime => tr!("backend-lime"),
            Backend::HydraSdr => tr!("backend-hydra-sdr"),
            Backend::Fobos => tr!("backend-fobos"),
            Backend::UsbAudio => tr!("backend-usb-audio"),
            Backend::None => tr!("backend-none"),
        }
    }
}

impl Tr for CatFamily {
    fn tr(self) -> String {
        match self {
            CatFamily::Xiegu => tr!("cat-family-xiegu"),
            CatFamily::Icom => tr!("cat-family-icom"),
            CatFamily::Yaesu => tr!("cat-family-yaesu"),
            CatFamily::Kenwood => tr!("cat-family-kenwood"),
            CatFamily::Elecraft => tr!("cat-family-elecraft"),
            CatFamily::Elad => tr!("cat-family-elad"),
            CatFamily::QrpLabs => tr!("cat-family-qrp-labs"),
            CatFamily::RsHfiq => tr!("cat-family-rs-hfiq"),
            CatFamily::TrUsdx => tr!("cat-family-tr-usdx"),
            CatFamily::Rigctld => tr!("cat-family-rigctld"),
            CatFamily::Flrig => tr!("cat-family-flrig"),
        }
    }
}

impl Tr for AgcMode {
    fn tr(self) -> String {
        match self {
            AgcMode::Off => tr!("agc-mode-off"),
            AgcMode::Slow => tr!("agc-mode-slow"),
            AgcMode::Med => tr!("agc-mode-med"),
            AgcMode::Fast => tr!("agc-mode-fast"),
        }
    }
}

impl Tr for AirspyGain {
    fn tr(self) -> String {
        match self {
            AirspyGain::Linearity => tr!("airspy-gain-linearity"),
            AirspyGain::Sensitivity => tr!("airspy-gain-sensitivity"),
        }
    }
}

impl Tr for AisKind {
    fn tr(self) -> String {
        match self {
            AisKind::ClassA => tr!("ais-kind-class-a"),
            AisKind::ClassB => tr!("ais-kind-class-b"),
            AisKind::BaseStation => tr!("ais-kind-base-station"),
            AisKind::SarAircraft => tr!("ais-kind-sar-aircraft"),
            AisKind::AidToNavigation => tr!("ais-kind-aid-to-navigation"),
            AisKind::Sart => tr!("ais-kind-sart"),
            AisKind::Craft => tr!("ais-kind-craft"),
        }
    }
}

impl Tr for AlertEvent {
    fn tr(self) -> String {
        match self {
            AlertEvent::Called => tr!("alert-event-called"),
            AlertEvent::Cq => tr!("alert-event-cq"),
            AlertEvent::NewDxcc => tr!("alert-event-new-dxcc"),
            AlertEvent::NewDxccBand => tr!("alert-event-new-dxcc-band"),
            AlertEvent::NewGrid => tr!("alert-event-new-grid"),
        }
    }
}

impl Tr for AudioRouting {
    fn tr(self) -> String {
        match self {
            AudioRouting::Both => tr!("audio-routing-both"),
            AudioRouting::Left => tr!("audio-routing-left"),
            AudioRouting::Right => tr!("audio-routing-right"),
        }
    }
}

impl Tr for ButtonMode {
    fn tr(self) -> String {
        match self {
            ButtonMode::Momentary => tr!("button-mode-momentary"),
            ButtonMode::Toggle => tr!("button-mode-toggle"),
        }
    }
}

impl Tr for ContestMode {
    fn tr(self) -> String {
        match self {
            ContestMode::None => tr!("contest-mode-none"),
            ContestMode::EuVhf => tr!("contest-mode-eu-vhf"),
        }
    }
}

impl Tr for ConverterTx {
    fn tr(self) -> String {
        match self {
            ConverterTx::Off => tr!("converter-tx-off"),
            ConverterTx::Transverter => tr!("converter-tx-transverter"),
            ConverterTx::Own(..) => tr!("converter-tx-own"),
        }
    }
}

impl Tr for CwEngine {
    fn tr(self) -> String {
        match self {
            CwEngine::Neural => tr!("cw-engine-neural"),
            CwEngine::Timing => tr!("cw-engine-timing"),
        }
    }
}

impl Tr for DecodeSort {
    fn tr(self) -> String {
        match self {
            DecodeSort::None => tr!("decode-sort-none"),
            DecodeSort::Signal => tr!("decode-sort-signal"),
            DecodeSort::Distance => tr!("decode-sort-distance"),
            DecodeSort::Country => tr!("decode-sort-country"),
        }
    }
}

impl Tr for DiversityMode {
    fn tr(self) -> String {
        match self {
            DiversityMode::Cancel => tr!("diversity-mode-cancel"),
            DiversityMode::Combine => tr!("diversity-mode-combine"),
        }
    }
}

impl Tr for DxpedMode {
    fn tr(self) -> String {
        match self {
            DxpedMode::Normal => tr!("dxped-mode-normal"),
            DxpedMode::Hound => tr!("dxped-mode-hound"),
            DxpedMode::Fox => tr!("dxped-mode-fox"),
        }
    }
}

impl Tr for FailSafe {
    fn tr(self) -> String {
        match self {
            FailSafe::RefuseTx => tr!("fail-safe-refuse-tx"),
            FailSafe::WarnOnly => tr!("fail-safe-warn-only"),
        }
    }
}

impl Tr for FtpSecurity {
    fn tr(self) -> String {
        match self {
            FtpSecurity::Plain => tr!("ftp-security-plain"),
            FtpSecurity::ExplicitTls => tr!("ftp-security-explicit-tls"),
            FtpSecurity::ImplicitTls => tr!("ftp-security-implicit-tls"),
        }
    }
}

impl Tr for HpsdrFilterBoard {
    fn tr(self) -> String {
        match self {
            HpsdrFilterBoard::None => tr!("hpsdr-filter-board-none"),
            HpsdrFilterBoard::N2adr => tr!("hpsdr-filter-board-n2adr"),
            HpsdrFilterBoard::Alex => tr!("hpsdr-filter-board-alex"),
            HpsdrFilterBoard::Custom => tr!("hpsdr-filter-board-custom"),
        }
    }
}

impl Tr for HpsdrIoRxInput {
    fn tr(self) -> String {
        match self {
            HpsdrIoRxInput::Radio => tr!("hpsdr-io-rx-input-radio"),
            HpsdrIoRxInput::IoBoard => tr!("hpsdr-io-rx-input-io-board"),
            HpsdrIoRxInput::IoBoardPureSignal => tr!("hpsdr-io-rx-input-io-board-pure-signal"),
        }
    }
}

impl Tr for HydraSdrGain {
    fn tr(self) -> String {
        match self {
            HydraSdrGain::Linearity => tr!("hydra-sdr-gain-linearity"),
            HydraSdrGain::Sensitivity => tr!("hydra-sdr-gain-sensitivity"),
        }
    }
}

impl Tr for IcomRxSource {
    fn tr(self) -> String {
        match self {
            IcomRxSource::Af => tr!("icom-rx-source-af"),
            IcomRxSource::If12k => tr!("icom-rx-source-if12k"),
        }
    }
}

impl Tr for IcomScopeSpan {
    fn tr(self) -> String {
        match self {
            IcomScopeSpan::Radio => tr!("icom-scope-span-radio"),
            IcomScopeSpan::Khz2_5 => tr!("icom-scope-span-khz2-5"),
            IcomScopeSpan::Khz5 => tr!("icom-scope-span-khz5"),
            IcomScopeSpan::Khz10 => tr!("icom-scope-span-khz10"),
            IcomScopeSpan::Khz25 => tr!("icom-scope-span-khz25"),
            IcomScopeSpan::Khz50 => tr!("icom-scope-span-khz50"),
            IcomScopeSpan::Khz100 => tr!("icom-scope-span-khz100"),
            IcomScopeSpan::Khz250 => tr!("icom-scope-span-khz250"),
            IcomScopeSpan::Khz500 => tr!("icom-scope-span-khz500"),
        }
    }
}

impl Tr for IsmFamily {
    fn tr(self) -> String {
        match self {
            IsmFamily::Weather => tr!("ism-family-weather"),
            IsmFamily::Meter => tr!("ism-family-meter"),
            IsmFamily::HomeAuto => tr!("ism-family-home-auto"),
            IsmFamily::LoRa => tr!("ism-family-lo-ra"),
            IsmFamily::Unidentified => tr!("ism-family-unidentified"),
        }
    }
}

impl Tr for IsmProtocol {
    fn tr(self) -> String {
        match self {
            IsmProtocol::LaCrosseIt => tr!("ism-protocol-la-crosse-it"),
            IsmProtocol::FineOffset => tr!("ism-protocol-fine-offset"),
            IsmProtocol::Bresser => tr!("ism-protocol-bresser"),
            IsmProtocol::ZWave => tr!("ism-protocol-z-wave"),
            IsmProtocol::Homematic => tr!("ism-protocol-homematic"),
            IsmProtocol::Rtl433 => tr!("ism-protocol-rtl433"),
            IsmProtocol::Unidentified => tr!("ism-protocol-unidentified"),
        }
    }
}

impl Tr for Js8Speed {
    fn tr(self) -> String {
        match self {
            Js8Speed::Normal => tr!("js8-speed-normal"),
            Js8Speed::Fast => tr!("js8-speed-fast"),
            Js8Speed::Turbo => tr!("js8-speed-turbo"),
            Js8Speed::Slow => tr!("js8-speed-slow"),
        }
    }
}

impl Tr for LimeAuxRole {
    fn tr(self) -> String {
        match self {
            LimeAuxRole::Off => tr!("lime-aux-role-off"),
            LimeAuxRole::Diversity => tr!("lime-aux-role-diversity"),
            LimeAuxRole::PureSignal => tr!("lime-aux-role-pure-signal"),
        }
    }
}

impl Tr for LookupProvider {
    fn tr(self) -> String {
        match self {
            LookupProvider::None => tr!("lookup-provider-none"),
            LookupProvider::Qrz => tr!("lookup-provider-qrz"),
            LookupProvider::HamQth => tr!("lookup-provider-ham-qth"),
        }
    }
}

impl Tr for MailFolder {
    fn tr(self) -> String {
        match self {
            MailFolder::Inbox => tr!("mail-folder-inbox"),
            MailFolder::Outbox => tr!("mail-folder-outbox"),
            MailFolder::Sent => tr!("mail-folder-sent"),
            MailFolder::Archive => tr!("mail-folder-archive"),
        }
    }
}

impl Tr for MemorySort {
    fn tr(self) -> String {
        match self {
            MemorySort::Name => tr!("memory-sort-name"),
            MemorySort::Freq => tr!("memory-sort-freq"),
            MemorySort::Band => tr!("memory-sort-band"),
            MemorySort::Stored => tr!("memory-sort-stored"),
        }
    }
}

impl Tr for MouseButton {
    fn tr(self) -> String {
        match self {
            MouseButton::Middle => tr!("mouse-button-middle"),
            MouseButton::Extra1 => tr!("mouse-button-extra1"),
            MouseButton::Extra2 => tr!("mouse-button-extra2"),
        }
    }
}

impl Tr for NrLevel {
    fn tr(self) -> String {
        match self {
            NrLevel::Off => tr!("nr-level-off"),
            NrLevel::Low => tr!("nr-level-low"),
            NrLevel::Medium => tr!("nr-level-medium"),
            NrLevel::High => tr!("nr-level-high"),
            NrLevel::RnnLow => tr!("nr-level-rnn-low"),
            NrLevel::RnnMed => tr!("nr-level-rnn-med"),
            NrLevel::RnnHigh => tr!("nr-level-rnn-high"),
            NrLevel::SpecLow => tr!("nr-level-spec-low"),
            NrLevel::SpecMed => tr!("nr-level-spec-med"),
            NrLevel::SpecHigh => tr!("nr-level-spec-high"),
            NrLevel::Nr2Low => tr!("nr-level-nr2-low"),
            NrLevel::Nr2Med => tr!("nr-level-nr2-med"),
            NrLevel::Nr2High => tr!("nr-level-nr2-high"),
            NrLevel::DfLow => tr!("nr-level-df-low"),
            NrLevel::DfMed => tr!("nr-level-df-med"),
            NrLevel::DfHigh => tr!("nr-level-df-high"),
        }
    }
}

impl Tr for NrStrength {
    fn tr(self) -> String {
        match self {
            NrStrength::Low => tr!("nr-strength-low"),
            NrStrength::Med => tr!("nr-strength-med"),
            NrStrength::High => tr!("nr-strength-high"),
        }
    }
}

impl Tr for OrbitRings {
    fn tr(self) -> String {
        match self {
            OrbitRings::None => tr!("orbit-rings-none"),
            OrbitRings::Curated => tr!("orbit-rings-curated"),
            OrbitRings::All => tr!("orbit-rings-all"),
        }
    }
}

impl Tr for QsoStep {
    fn tr(self) -> String {
        match self {
            QsoStep::Idle => tr!("qso-step-idle"),
            QsoStep::WaitCq => tr!("qso-step-wait-cq"),
            QsoStep::CallingCq => tr!("qso-step-calling-cq"),
            QsoStep::TxGrid => tr!("qso-step-tx-grid"),
            QsoStep::TxReport => tr!("qso-step-tx-report"),
            QsoStep::TxRReport => tr!("qso-step-tx-r-report"),
            QsoStep::TxRr73 => tr!("qso-step-tx-rr73"),
            QsoStep::Tx73 => tr!("qso-step-tx73"),
            QsoStep::Confirming => tr!("qso-step-confirming"),
        }
    }
}

impl Tr for RdsStandard {
    fn tr(self) -> String {
        match self {
            RdsStandard::Auto => tr!("rds-standard-auto"),
            RdsStandard::Rds => tr!("rds-standard-rds"),
            RdsStandard::Rbds => tr!("rds-standard-rbds"),
        }
    }
}

impl Tr for Region {
    fn tr(self) -> String {
        match self {
            Region::R1 => tr!("region-r1"),
            Region::R2 => tr!("region-r2"),
            Region::R3 => tr!("region-r3"),
        }
    }
}

impl Tr for RelativeMode {
    fn tr(self) -> String {
        match self {
            RelativeMode::Absolute => tr!("relative-mode-absolute"),
            RelativeMode::SignMagnitude => tr!("relative-mode-sign-magnitude"),
            RelativeMode::TwosComplement => tr!("relative-mode-twos-complement"),
            RelativeMode::BinaryOffset => tr!("relative-mode-binary-offset"),
        }
    }
}

impl Tr for RelayLink {
    fn tr(self) -> String {
        match self {
            RelayLink::Off => tr!("relay-link-off"),
            RelayLink::Serial => tr!("relay-link-serial"),
            RelayLink::SerialLines => tr!("relay-link-serial-lines"),
            RelayLink::Hid => tr!("relay-link-hid"),
            RelayLink::Cm108 => tr!("relay-link-cm108"),
            RelayLink::Gpio => tr!("relay-link-gpio"),
            RelayLink::Command => tr!("relay-link-command"),
        }
    }
}

impl Tr for RelayRole {
    fn tr(self) -> String {
        match self {
            RelayRole::Unused => tr!("relay-role-unused"),
            RelayRole::SdrAntenna => tr!("relay-role-sdr-antenna"),
            RelayRole::Amplifier => tr!("relay-role-amplifier"),
            RelayRole::Aux => tr!("relay-role-aux"),
            RelayRole::BandDecoder => tr!("relay-role-band-decoder"),
        }
    }
}

impl Tr for RfeChannel {
    fn tr(self) -> String {
        match self {
            RfeChannel::Wb1000 => tr!("rfe-channel-wb1000"),
            RfeChannel::Wb4000 => tr!("rfe-channel-wb4000"),
            RfeChannel::Ham0030 => tr!("rfe-channel-ham0030"),
            RfeChannel::Ham0070 => tr!("rfe-channel-ham0070"),
            RfeChannel::Ham0145 => tr!("rfe-channel-ham0145"),
            RfeChannel::Ham0220 => tr!("rfe-channel-ham0220"),
            RfeChannel::Ham0435 => tr!("rfe-channel-ham0435"),
            RfeChannel::Ham0920 => tr!("rfe-channel-ham0920"),
            RfeChannel::Ham1280 => tr!("rfe-channel-ham1280"),
            RfeChannel::Ham2400 => tr!("rfe-channel-ham2400"),
            RfeChannel::Ham3500 => tr!("rfe-channel-ham3500"),
            RfeChannel::CellBand01 => tr!("rfe-channel-cell-band01"),
            RfeChannel::CellBand02 => tr!("rfe-channel-cell-band02"),
            RfeChannel::CellBand03 => tr!("rfe-channel-cell-band03"),
            RfeChannel::CellBand07 => tr!("rfe-channel-cell-band07"),
            RfeChannel::CellBand38 => tr!("rfe-channel-cell-band38"),
            RfeChannel::NotSelected => tr!("rfe-channel-not-selected"),
        }
    }
}

impl Tr for RfeLink {
    fn tr(self) -> String {
        match self {
            RfeLink::Off => tr!("rfe-link-off"),
            RfeLink::Serial => tr!("rfe-link-serial"),
            RfeLink::Board => tr!("rfe-link-board"),
        }
    }
}

impl Tr for RfeModeControl {
    fn tr(self) -> String {
        match self {
            RfeModeControl::Auto => tr!("rfe-mode-control-auto"),
            RfeModeControl::Rx => tr!("rfe-mode-control-rx"),
            RfeModeControl::Tx => tr!("rfe-mode-control-tx"),
            RfeModeControl::TxRx => tr!("rfe-mode-control-tx-rx"),
        }
    }
}

impl Tr for RifpEncoding {
    fn tr(self) -> String {
        match self {
            RifpEncoding::Auto => tr!("rifp-encoding-auto"),
            RifpEncoding::Png => tr!("rifp-encoding-png"),
            RifpEncoding::Jpeg => tr!("rifp-encoding-jpeg"),
            RifpEncoding::Raw => tr!("rifp-encoding-raw"),
            RifpEncoding::Rle8 => tr!("rifp-encoding-rle8"),
            RifpEncoding::Zlib => tr!("rifp-encoding-zlib"),
            RifpEncoding::Group4 => tr!("rifp-encoding-group4"),
            RifpEncoding::Group3 => tr!("rifp-encoding-group3"),
        }
    }
}

impl Tr for ScanKind {
    fn tr(self) -> String {
        match self {
            ScanKind::Memories => tr!("scan-kind-memories"),
            ScanKind::Range => tr!("scan-kind-range"),
        }
    }
}

impl Tr for ScanResume {
    fn tr(self) -> String {
        match self {
            ScanResume::Carrier => tr!("scan-resume-carrier"),
            ScanResume::Timed => tr!("scan-resume-timed"),
            ScanResume::Manual => tr!("scan-resume-manual"),
        }
    }
}

impl Tr for SenseLine {
    fn tr(self) -> String {
        match self {
            SenseLine::Off => tr!("sense-line-off"),
            SenseLine::Cts => tr!("sense-line-cts"),
            SenseLine::Dsr => tr!("sense-line-dsr"),
            SenseLine::Dcd => tr!("sense-line-dcd"),
        }
    }
}

impl Tr for Shift {
    fn tr(self) -> String {
        match self {
            Shift::Simplex => tr!("shift-simplex"),
            Shift::Minus => tr!("shift-minus"),
            Shift::Plus => tr!("shift-plus"),
        }
    }
}

impl Tr for Speed {
    fn tr(self) -> String {
        match self {
            Speed::Slow => tr!("speed-slow"),
            Speed::Medium => tr!("speed-medium"),
            Speed::Fast => tr!("speed-fast"),
            Speed::Faster => tr!("speed-faster"),
            Speed::Fastest => tr!("speed-fastest"),
        }
    }
}

impl Tr for TlsVerify {
    fn tr(self) -> String {
        match self {
            TlsVerify::System => tr!("tls-verify-system"),
            TlsVerify::Pinned => tr!("tls-verify-pinned"),
            TlsVerify::Off => tr!("tls-verify-off"),
        }
    }
}

impl Tr for ToneMode {
    fn tr(self) -> String {
        match self {
            ToneMode::Off => tr!("tone-mode-off"),
            ToneMode::Ctcss => tr!("tone-mode-ctcss"),
            ToneMode::Dcs => tr!("tone-mode-dcs"),
        }
    }
}

impl Tr for TxVoiceStage {
    fn tr(self) -> String {
        match self {
            TxVoiceStage::Gate => tr!("tx-voice-stage-gate"),
            TxVoiceStage::Eq => tr!("tx-voice-stage-eq"),
            TxVoiceStage::DeEss => tr!("tx-voice-stage-de-ess"),
            TxVoiceStage::Comp => tr!("tx-voice-stage-comp"),
        }
    }
}

impl Tr for UploadNaming {
    fn tr(self) -> String {
        match self {
            UploadNaming::Sequence => tr!("upload-naming-sequence"),
            UploadNaming::Original => tr!("upload-naming-original"),
        }
    }
}

impl Tr for WheelAction {
    fn tr(self) -> String {
        match self {
            WheelAction::Zoom => tr!("wheel-action-zoom"),
            WheelAction::Tune => tr!("wheel-action-tune"),
            WheelAction::None => tr!("wheel-action-none"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In English every value reads exactly as its `label()` does, so moving a
    /// picker onto these changes nothing for an English-speaking operator.
    /// Holds only while the loader is in English, which no test changes.
    #[test]
    fn english_matches_the_types_own_labels() {
        fn check<T: Tr + std::fmt::Debug>(all: &[T], label: fn(T) -> &'static str) {
            for &v in all {
                assert_eq!(v.tr(), label(v), "{v:?}");
            }
        }
        check(&LayoutMode::ALL, LayoutMode::label);
        check(&UiTheme::ALL, UiTheme::label);
        check(&ChromeStyle::ALL, ChromeStyle::label);
        check(&FontSize::ALL, FontSize::label);
        check(&BandplanKind::ALL, BandplanKind::label);
        check(&Verbosity::ALL, Verbosity::label);
        check(&FreqStyle::ALL, FreqStyle::label);
        check(&CallsignStyle::ALL, CallsignStyle::label);
        check(&AgcMode::ALL, AgcMode::label);
        check(&AirspyGain::ALL, AirspyGain::label);
        check(&AlertEvent::ALL, AlertEvent::label);
        check(&AudioRouting::ALL, AudioRouting::label);
        check(&ButtonMode::ALL, ButtonMode::label);
        check(&ContestMode::ALL, ContestMode::label);
        check(&CwEngine::ALL, CwEngine::label);
        check(&DecodeSort::ALL, DecodeSort::label);
        check(&DiversityMode::ALL, DiversityMode::label);
        check(&DxpedMode::ALL, DxpedMode::label);
        check(&FtpSecurity::ALL, FtpSecurity::label);
        check(&HpsdrFilterBoard::ALL, HpsdrFilterBoard::label);
        check(&HpsdrIoRxInput::ALL, HpsdrIoRxInput::label);
        check(&HydraSdrGain::ALL, HydraSdrGain::label);
        check(&IcomRxSource::ALL, IcomRxSource::label);
        check(&IcomScopeSpan::ALL, IcomScopeSpan::label);
        check(&IsmFamily::ALL, IsmFamily::label);
        check(&IsmProtocol::ALL, IsmProtocol::label);
        check(&Js8Speed::ALL, Js8Speed::label);
        check(&LimeAuxRole::ALL, LimeAuxRole::label);
        check(&LookupProvider::ALL, LookupProvider::label);
        check(&MailFolder::ALL, MailFolder::label);
        check(&MemorySort::ALL, MemorySort::label);
        check(&MouseButton::ALL, MouseButton::label);
        check(&NrLevel::ALL, NrLevel::label);
        check(&NrStrength::ALL, NrStrength::label);
        check(&OrbitRings::ALL, OrbitRings::label);
        check(&RdsStandard::ALL, RdsStandard::label);
        check(&Region::ALL, Region::label);
        check(&RelativeMode::ALL, RelativeMode::label);
        check(&RfeChannel::ALL, RfeChannel::label);
        check(&RfeLink::ALL, RfeLink::label);
        check(&RfeModeControl::ALL, RfeModeControl::label);
        check(&RifpEncoding::ALL, RifpEncoding::label);
        check(&ScanKind::ALL, ScanKind::label);
        check(&ScanResume::ALL, ScanResume::label);
        check(&Shift::ALL, Shift::label);
        check(&Speed::ALL, Speed::label);
        check(&TlsVerify::ALL, TlsVerify::label);
        check(&ToneMode::ALL, ToneMode::label);
        check(&TxVoiceStage::ALL, TxVoiceStage::label);
        check(&UploadNaming::ALL, UploadNaming::label);
        check(&WheelAction::ALL, WheelAction::label);
        check(&RtlSdrHfMode::ALL, RtlSdrHfMode::label);
        check(&RtlSdrAgc::ALL, RtlSdrAgc::label);
        check(&ModeControl::ALL, ModeControl::label);
        check(&LineState::ALL, LineState::label);
        check(&EladTxInput::ALL, EladTxInput::label);
        check(&TrUsdxAudio::ALL, TrUsdxAudio::label);
        check(&SpyServerFormat::ALL, SpyServerFormat::label);
        check(&SoundFormat::ALL, SoundFormat::label);
        check(&SdrPlayDuoTuner::ALL, SdrPlayDuoTuner::label);
        check(&SdrPlayDuoRole::ALL, SdrPlayDuoRole::label);
        check(&SdrPlayAgc::ALL, SdrPlayAgc::label);
        check(&RotatorProtocol::ALL, RotatorProtocol::label);
        check(&PlutoPtt::ALL, PlutoPtt::label);
        check(&PlutoDuplex::ALL, PlutoDuplex::label);
        check(&PlutoAgc::ALL, PlutoAgc::label);
        check(&Parity::ALL, Parity::label);
        check(&PanadapterTap::ALL, PanadapterTap::label);
        check(&PanadapterAudio::ALL, PanadapterAudio::label);
        check(&KenwoodSend::ALL, KenwoodSend::label);
        check(&DigiMode::ALL, DigiMode::label);
        check(&CwKeying::ALL, CwKeying::label);
        check(&AudioRoute::ALL, AudioRoute::label);
        check(&AmpModel::ALL, AmpModel::label);
        check(&AlertSound::ALL, AlertSound::label);
        check(&AlertReply::ALL, AlertReply::label);
        check(&IcomModel::ALL, IcomModel::label);
        check(&Backend::ALL, Backend::label);
        check(&CatFamily::ALL, CatFamily::label);
    }
}
