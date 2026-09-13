use serde::{Deserialize, Serialize};
use std::sync::Mutex;

const STORE_DIR: &str = "ux0:data/opennow-vita";
const SETTINGS_JSON_PATH: &str = "ux0:data/opennow-vita/settings.json";

// Old paths for one-time legacy migration
const FPS_STORE_PATH: &str = "ux0:data/opennow-vita/stream-fps.txt";
const TRIGGER_STORE_PATH: &str = "ux0:data/opennow-vita/trigger-intensity.txt";
const AUDIO_BOOST_STORE_PATH: &str = "ux0:data/opennow-vita/audio-boost.txt";
const CONTROLS_HINT_STORE_PATH: &str = "ux0:data/opennow-vita/controls-hint-seen.txt";
const STICK_ZONES_STORE_PATH: &str = "ux0:data/opennow-vita/stick-zones.txt";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub fps: u32,
    pub trigger_intensity: u8,
    pub audio_boost_percent: u16,
    pub controls_hint_seen: bool,
    pub stick_zones: String,
    #[serde(default = "default_catalog_sort")]
    pub catalog_sort: String,
    #[serde(default = "default_rear_touch_mode")]
    pub rear_touch_mode: String,
    #[serde(default = "default_catalog_filter")]
    pub catalog_filter: String,
    #[serde(default = "default_true")]
    pub session_timer_enabled: bool,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub game_language: String,
    #[serde(default = "default_color_depth")]
    pub color_depth: String,
    #[serde(default)]
    pub game_profiles: std::collections::BTreeMap<String, GameProfile>,
    #[serde(default)]
    pub trigger_swap_enabled: bool,
    #[serde(default)]
    pub ui_locale: String,
    /// Master switch for the PC-touch overlay mod: rear-panel mouse plus the front-panel key
    /// strips and sliders. See `src/input.rs`.
    #[serde(default = "default_true")]
    pub pc_overlay_enabled: bool,
    /// Which control profile the stream is in. `game` hands the pad to the title untouched and
    /// keeps the rear panel as L2/R2; `desktop` repurposes the pad and rear panel as a
    /// mouse/keyboard. See `ControlProfile`.
    #[serde(default = "default_control_profile")]
    pub control_profile: String,
    /// Whether the overlay is expanded (zones + control panel drawn) or collapsed to just the
    /// eye toggle. The eye itself is always drawn while `pc_overlay_enabled` is set.
    #[serde(default = "default_true")]
    pub overlay_revealed: bool,
    #[serde(default = "default_overlay_alpha")]
    pub pc_overlay_alpha: u8,
    #[serde(default = "default_overlay_sensitivity_percent")]
    pub overlay_sensitivity_percent: u16,
    /// When true (the default), login always uses NVIDIA's own idp and skips the
    /// GeoIP/ISP-based `serviceUrls` provider discovery entirely. Some ISPs (e.g. Peruvian
    /// carriers reselling GFN through "powered by Digevo") get silently redirected to that
    /// regional partner's login/idp even over a VPN, because the request that decides the
    /// preferred provider is a plain HTTPS GET to `pcs.geforcenow.com` that some networks
    /// transparently proxy or that NVIDIA's GeoIP database still maps to the ISP's home
    /// country. Forcing the direct NVIDIA idp avoids that endpoint altogether, which is what a
    /// user with a US-registered Ultimate + persistent-storage account wants.
    #[serde(default = "default_true")]
    pub force_direct_nvidia_login: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rear_touch_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stick_zones: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_intensity: Option<u8>,
}

static ACTIVE_GAME: Mutex<Option<String>> = Mutex::new(None);

pub fn set_active_game(app_id: Option<&str>) {
    if let Ok(mut guard) = ACTIVE_GAME.lock() {
        let next = app_id.map(str::to_owned);
        if *guard != next {
            *guard = next;
        }
    }
}

pub fn active_game() -> Option<String> {
    ACTIVE_GAME.lock().ok().and_then(|guard| guard.clone())
}

fn active_profile() -> Option<GameProfile> {
    let app_id = active_game()?;
    with_cached_settings(|s| s.game_profiles.get(&app_id).cloned())
}

pub fn active_game_has_profile() -> bool {
    let Some(app_id) = active_game() else {
        return false;
    };
    with_cached_settings(|s| s.game_profiles.contains_key(&app_id))
}

pub fn set_active_game_profile(enabled: bool) {
    let Some(app_id) = active_game() else {
        return;
    };
    update_settings(|s| {
        if enabled {
            let seed = GameProfile {
                rear_touch_mode: Some(s.rear_touch_mode.clone()),
                stick_zones: Some(s.stick_zones.clone()),
                trigger_intensity: Some(s.trigger_intensity),
            };
            s.game_profiles.entry(app_id).or_insert(seed);
        } else {
            s.game_profiles.remove(&app_id);
        }
    });
}

fn update_control_setting<F>(apply: F)
where
    F: FnOnce(&mut AppSettings, Option<&str>),
{
    let app_id = active_game().filter(|_| active_game_has_profile());
    update_settings(|s| apply(s, app_id.as_deref()));
}

fn default_true() -> bool {
    true
}

fn default_catalog_sort() -> String {
    "last_played".to_owned()
}

fn default_catalog_filter() -> String {
    "my_games".to_owned()
}

fn default_rear_touch_mode() -> String {
    "quadrant".to_owned()
}

fn default_color_depth() -> String {
    ColorDepth::default().key().to_owned()
}

fn default_overlay_alpha() -> u8 {
    OverlayOpacity::default().alpha()
}

fn default_control_profile() -> String {
    ControlProfile::default().key().to_owned()
}

fn default_overlay_sensitivity_percent() -> u16 {
    OverlaySensitivity::default().percent()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            fps: 60,
            trigger_intensity: 255,
            audio_boost_percent: 1200,
            controls_hint_seen: false,
            stick_zones: "hidden".to_owned(),
            catalog_sort: "last_played".to_owned(),
            rear_touch_mode: "quadrant".to_owned(),
            catalog_filter: "my_games".to_owned(),
            session_timer_enabled: true,
            region: String::new(),
            game_language: GameLanguage::default().code().to_owned(),
            color_depth: ColorDepth::default().key().to_owned(),
            game_profiles: std::collections::BTreeMap::new(),
            trigger_swap_enabled: false,
            ui_locale: String::new(),
            pc_overlay_enabled: true,
            control_profile: default_control_profile(),
            overlay_revealed: true,
            pc_overlay_alpha: default_overlay_alpha(),
            overlay_sensitivity_percent: default_overlay_sensitivity_percent(),
            force_direct_nvidia_login: default_true(),
        }
    }
}

static CACHED_SETTINGS: Mutex<Option<AppSettings>> = Mutex::new(None);

fn with_cached_settings<R>(f: impl FnOnce(&AppSettings) -> R) -> R {
    {
        let guard = CACHED_SETTINGS.lock().unwrap();
        if let Some(ref settings) = *guard {
            return f(settings);
        }
    }
    let _ = load_or_init_settings();
    let guard = CACHED_SETTINGS.lock().unwrap();
    f(guard.as_ref().expect("settings cache populated"))
}

/// Disk load / legacy migration. Must not touch `CACHED_SETTINGS` — callers that already hold
/// that lock (notably `update_settings`) would otherwise self-deadlock on `std::sync::Mutex`.
fn read_or_migrate_settings() -> AppSettings {
    if let Ok(content) = std::fs::read_to_string(SETTINGS_JSON_PATH) {
        match serde_json::from_str::<AppSettings>(&content) {
            Ok(settings) => return settings,
            Err(_) => {
                eprintln!("settings.json corrupt; recreating with stable defaults");
                let settings = AppSettings::default();
                save_settings_disk(&settings);
                return settings;
            }
        }
    }

    let mut settings = AppSettings::default();

    if let Ok(text) = std::fs::read_to_string(FPS_STORE_PATH) {
        if let Ok(val) = text.trim().parse::<u32>() {
            settings.fps = val;
        }
        let _ = std::fs::remove_file(FPS_STORE_PATH);
    }
    if let Ok(text) = std::fs::read_to_string(TRIGGER_STORE_PATH) {
        if let Ok(val) = text.trim().parse::<u8>() {
            settings.trigger_intensity = val;
        }
        let _ = std::fs::remove_file(TRIGGER_STORE_PATH);
    }
    if let Ok(text) = std::fs::read_to_string(AUDIO_BOOST_STORE_PATH) {
        if let Ok(val) = text.trim().parse::<u16>() {
            settings.audio_boost_percent = val;
        }
        let _ = std::fs::remove_file(AUDIO_BOOST_STORE_PATH);
    }
    if std::fs::metadata(CONTROLS_HINT_STORE_PATH).is_ok() {
        settings.controls_hint_seen = true;
        let _ = std::fs::remove_file(CONTROLS_HINT_STORE_PATH);
    }
    if let Ok(text) = std::fs::read_to_string(STICK_ZONES_STORE_PATH) {
        settings.stick_zones = text.trim().to_owned();
        let _ = std::fs::remove_file(STICK_ZONES_STORE_PATH);
    }

    save_settings_disk(&settings);
    settings
}

fn load_or_init_settings() -> AppSettings {
    {
        let guard = CACHED_SETTINGS.lock().unwrap();
        if let Some(ref settings) = *guard {
            return settings.clone();
        }
    }

    let settings = read_or_migrate_settings();
    let mut guard = CACHED_SETTINGS.lock().unwrap();
    if let Some(ref cached) = *guard {
        return cached.clone();
    }
    *guard = Some(settings.clone());
    settings
}

fn save_settings_disk(settings: &AppSettings) {
    if std::fs::create_dir_all(STORE_DIR).is_err() {
        return;
    }
    let Ok(json) = serde_json::to_string_pretty(settings) else {
        return;
    };
    let tmp_path = format!("{SETTINGS_JSON_PATH}.tmp");
    if std::fs::write(&tmp_path, &json).is_ok() {
        if std::fs::rename(&tmp_path, SETTINGS_JSON_PATH).is_ok() {
            return;
        }
        let _ = std::fs::remove_file(&tmp_path);
    }
    let _ = std::fs::write(SETTINGS_JSON_PATH, json);
}

fn update_settings<F: FnOnce(&mut AppSettings)>(f: F) {
    let mut settings = {
        let guard = CACHED_SETTINGS.lock().unwrap();
        guard.clone()
    }
    .unwrap_or_else(read_or_migrate_settings);
    f(&mut settings);
    save_settings_disk(&settings);
    *CACHED_SETTINGS.lock().unwrap() = Some(settings);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorDepth {
    ThirtyTwoBit,
    #[default]
    SixteenBit,
}

impl ColorDepth {
    pub const ALL: [ColorDepth; 2] = [Self::ThirtyTwoBit, Self::SixteenBit];

    fn key(self) -> &'static str {
        match self {
            Self::ThirtyTwoBit => "32",
            Self::SixteenBit => "16",
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::ThirtyTwoBit => "settings-color-depth-32",
            Self::SixteenBit => "settings-color-depth-16",
        }
    }

    fn from_key(key: &str) -> Self {
        match key {
            "16" => Self::SixteenBit,
            _ => Self::ThirtyTwoBit,
        }
    }

    pub fn stream_bit_depth(self) -> u32 {
        8
    }
}

pub fn color_depth() -> ColorDepth {
    with_cached_settings(|s| ColorDepth::from_key(&s.color_depth))
}

pub fn set_color_depth(depth: ColorDepth) {
    update_settings(|s| s.color_depth = depth.key().to_owned());
}

/// Frame rate to request from GFN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamFps {
    #[default]
    Sixty,
    Thirty,
}

impl StreamFps {
    pub const ALL: [StreamFps; 2] = [Self::Sixty, Self::Thirty];

    pub fn value(self) -> u32 {
        match self {
            Self::Sixty => 60,
            Self::Thirty => 30,
        }
    }

    fn from_value(fps: u32) -> Self {
        match fps {
            30 => Self::Thirty,
            _ => Self::Sixty,
        }
    }
}

pub fn fps() -> StreamFps {
    with_cached_settings(|s| StreamFps::from_value(s.fps))
}

pub fn fps_value() -> u32 {
    fps().value()
}

pub fn set_fps(fps: StreamFps) {
    update_settings(|s| s.fps = fps.value());
}

/// How hard a rear-panel touch presses L2/R2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TriggerIntensity {
    #[default]
    Full,
    High,
    Half,
}

impl TriggerIntensity {
    pub const ALL: [TriggerIntensity; 3] = [Self::Full, Self::High, Self::Half];

    pub fn value(self) -> u8 {
        match self {
            Self::Full => 255,
            Self::High => 192,
            Self::Half => 128,
        }
    }

    fn from_value(value: u8) -> Self {
        match value {
            v if v == 255 => Self::Full,
            v if v >= 192 => Self::High,
            _ => Self::Half,
        }
    }
}

pub fn trigger_intensity() -> TriggerIntensity {
    if let Some(value) = active_profile().and_then(|profile| profile.trigger_intensity) {
        return TriggerIntensity::from_value(value);
    }
    with_cached_settings(|s| TriggerIntensity::from_value(s.trigger_intensity))
}

pub fn set_trigger_intensity(intensity: TriggerIntensity) {
    update_control_setting(
        |s, app_id| match app_id.and_then(|id| s.game_profiles.get_mut(id)) {
            Some(profile) => profile.trigger_intensity = Some(intensity.value()),
            None => s.trigger_intensity = intensity.value(),
        },
    );
}

pub fn stick_zones() -> StickZones {
    if let Some(text) = active_profile().and_then(|profile| profile.stick_zones) {
        return StickZones::from_text(&text);
    }
    with_cached_settings(|s| StickZones::from_text(&s.stick_zones))
}

pub fn set_stick_zones(zones: StickZones) {
    update_control_setting(
        |s, app_id| match app_id.and_then(|id| s.game_profiles.get_mut(id)) {
            Some(profile) => profile.stick_zones = Some(zones.as_text().to_owned()),
            None => s.stick_zones = zones.as_text().to_owned(),
        },
    );
}

/// How much the decoded stream is amplified, in percent of unity gain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioBoost {
    Off,
    Low,
    #[default]
    Normal,
    High,
    Max,
}

impl AudioBoost {
    pub const ALL: [AudioBoost; 5] = [Self::Off, Self::Low, Self::Normal, Self::High, Self::Max];

    pub fn percent(self) -> u16 {
        match self {
            Self::Off => 100,
            Self::Low => 800,
            Self::Normal => 1200,
            Self::High => 1400,
            Self::Max => 1600,
        }
    }

    fn from_percent(percent: u16) -> Self {
        match percent {
            p if p >= 1600 => Self::Max,
            p if p >= 1400 => Self::High,
            p if p >= 1200 => Self::Normal,
            p if p >= 800 => Self::Low,
            _ => Self::Off,
        }
    }
}

pub fn audio_boost() -> AudioBoost {
    let s = load_or_init_settings();
    AudioBoost::from_percent(s.audio_boost_percent)
}

pub fn set_audio_boost(boost: AudioBoost) {
    update_settings(|s| s.audio_boost_percent = boost.percent());
}

pub fn controls_hint_seen() -> bool {
    let s = load_or_init_settings();
    s.controls_hint_seen
}

pub fn mark_controls_hint_seen() {
    update_settings(|s| s.controls_hint_seen = true);
}

/// Whether the front screen's bottom corners act as L3/R3, and whether they are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StickZones {
    Off,
    #[default]
    Hidden,
    Visible,
}

impl StickZones {
    pub const ALL: [StickZones; 3] = [Self::Off, Self::Hidden, Self::Visible];

    pub fn is_active(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Off => "settings-stick-zones-off",
            Self::Hidden => "settings-stick-zones-hidden",
            Self::Visible => "settings-stick-zones-visible",
        }
    }

    fn from_text(text: &str) -> Self {
        match text.trim() {
            "off" => Self::Off,
            "visible" => Self::Visible,
            _ => Self::Hidden,
        }
    }

    pub fn debug_label(self) -> &'static str {
        self.as_text()
    }

    fn as_text(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Hidden => "hidden",
            Self::Visible => "visible",
        }
    }
}

pub fn saved_catalog_sort() -> String {
    let s = load_or_init_settings();
    s.catalog_sort
}

pub fn set_saved_catalog_sort(sort_text: &str) {
    update_settings(|s| s.catalog_sort = sort_text.to_owned());
}

pub fn saved_catalog_filter() -> String {
    let s = load_or_init_settings();
    s.catalog_filter
}

pub fn set_saved_catalog_filter(filter_text: &str) {
    update_settings(|s| s.catalog_filter = filter_text.to_owned());
}

// rear panel layout mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RearTouchMode {
    // left half L2, right half R2
    Halves,
    // 4 corners: TL L2, TR R2, BL L3, BR R3
    #[default]
    Quadrant,
}

impl RearTouchMode {
    pub const ALL: [RearTouchMode; 2] = [Self::Quadrant, Self::Halves];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Quadrant => "settings-rear-touch-quadrant",
            Self::Halves => "settings-rear-touch-halves",
        }
    }

    fn from_text(text: &str) -> Self {
        match text.trim() {
            "halves" => Self::Halves,
            _ => Self::Quadrant,
        }
    }

    pub fn as_text(self) -> &'static str {
        match self {
            Self::Halves => "halves",
            Self::Quadrant => "quadrant",
        }
    }
}

pub fn rear_touch_mode() -> RearTouchMode {
    if let Some(text) = active_profile().and_then(|profile| profile.rear_touch_mode) {
        return RearTouchMode::from_text(&text);
    }
    let s = load_or_init_settings();
    RearTouchMode::from_text(&s.rear_touch_mode)
}

pub fn set_rear_touch_mode(mode: RearTouchMode) {
    update_control_setting(
        |s, app_id| match app_id.and_then(|id| s.game_profiles.get_mut(id)) {
            Some(profile) => profile.rear_touch_mode = Some(mode.as_text().to_owned()),
            None => s.rear_touch_mode = mode.as_text().to_owned(),
        },
    );
}

pub fn region() -> String {
    let s = load_or_init_settings();
    crate::gfn::regions::normalize_base_url(&s.region).unwrap_or_default()
}

pub fn set_region(base_url: &str) {
    let normalized = crate::gfn::regions::normalize_base_url(base_url).unwrap_or_default();
    update_settings(|s| s.region = normalized);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameLanguage {
    #[default]
    EnUs,
    EnGb,
    EsEs,
    EsMx,
    PtBr,
    FrFr,
    DeDe,
    ItIt,
    NlNl,
    PlPl,
    CsCz,
    HuHu,
    RuRu,
    UkUa,
    TrTr,
    SvSe,
    NbNo,
    DaDk,
    FiFi,
    JaJp,
    KoKr,
    ZhCn,
    ZhTw,
    ThTh,
}

impl GameLanguage {
    pub const ALL: [GameLanguage; 24] = [
        Self::EnUs,
        Self::EnGb,
        Self::EsEs,
        Self::EsMx,
        Self::PtBr,
        Self::FrFr,
        Self::DeDe,
        Self::ItIt,
        Self::NlNl,
        Self::PlPl,
        Self::CsCz,
        Self::HuHu,
        Self::RuRu,
        Self::UkUa,
        Self::TrTr,
        Self::SvSe,
        Self::NbNo,
        Self::DaDk,
        Self::FiFi,
        Self::JaJp,
        Self::KoKr,
        Self::ZhCn,
        Self::ZhTw,
        Self::ThTh,
    ];

    fn info(self) -> (&'static str, &'static str) {
        match self {
            Self::EnUs => ("en_US", "English (US)"),
            Self::EnGb => ("en_GB", "English (UK)"),
            Self::EsEs => ("es_ES", "Español (España)"),
            Self::EsMx => ("es_MX", "Español (México)"),
            Self::PtBr => ("pt_BR", "Português (Brasil)"),
            Self::FrFr => ("fr_FR", "Français"),
            Self::DeDe => ("de_DE", "Deutsch"),
            Self::ItIt => ("it_IT", "Italiano"),
            Self::NlNl => ("nl_NL", "Nederlands"),
            Self::PlPl => ("pl_PL", "Polski"),
            Self::CsCz => ("cs_CZ", "Čeština"),
            Self::HuHu => ("hu_HU", "Magyar"),
            Self::RuRu => ("ru_RU", "Russian (RU)"),
            Self::UkUa => ("uk_UA", "Ukrainian (UA)"),
            Self::TrTr => ("tr_TR", "Türkçe"),
            Self::SvSe => ("sv_SE", "Svenska"),
            Self::NbNo => ("nb_NO", "Norsk"),
            Self::DaDk => ("da_DK", "Dansk"),
            Self::FiFi => ("fi_FI", "Suomi"),
            Self::JaJp => ("ja_JP", "日本語"),
            Self::KoKr => ("ko_KR", "한국어"),
            Self::ZhCn => ("zh_CN", "简体中文"),
            Self::ZhTw => ("zh_TW", "繁體中文"),
            Self::ThTh => ("th_TH", "Thai (TH)"),
        }
    }

    pub fn code(self) -> &'static str {
        self.info().0
    }

    pub fn label(self) -> &'static str {
        self.info().1
    }

    fn from_code(code: &str) -> Self {
        let code = code.trim();
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.code() == code)
            .unwrap_or_default()
    }
}

pub fn game_language() -> GameLanguage {
    let s = load_or_init_settings();
    GameLanguage::from_code(&s.game_language)
}

pub fn set_game_language(language: GameLanguage) {
    update_settings(|s| s.game_language = language.code().to_owned());
}

pub fn session_timer_enabled() -> bool {
    let s = load_or_init_settings();
    s.session_timer_enabled
}

pub fn set_session_timer_enabled(enabled: bool) {
    update_settings(|s| s.session_timer_enabled = enabled);
}

pub fn trigger_swap_enabled() -> bool {
    let s = load_or_init_settings();
    s.trigger_swap_enabled
}

pub fn force_direct_nvidia_login() -> bool {
    let s = load_or_init_settings();
    s.force_direct_nvidia_login
}

pub fn set_force_direct_nvidia_login(enabled: bool) {
    update_settings(|s| s.force_direct_nvidia_login = enabled);
}

pub fn set_trigger_swap_enabled(enabled: bool) {
    update_settings(|s| s.trigger_swap_enabled = enabled);
}

pub fn ui_locale() -> crate::locale::Locale {
    crate::locale::Locale::from_str(&load_or_init_settings().ui_locale)
}

pub fn set_ui_locale(locale: crate::locale::Locale) {
    update_settings(|s| s.ui_locale = locale.as_str().to_owned());
}

/// Master switch for the PC-touch overlay mod (rear-panel mouse + front-panel key strips).
/// See `src/input.rs`.
pub fn pc_overlay_enabled() -> bool {
    load_or_init_settings().pc_overlay_enabled
}

pub fn set_pc_overlay_enabled(enabled: bool) {
    update_settings(|s| s.pc_overlay_enabled = enabled);
}

/// Which of the two control schemes the stream is currently using.
///
/// The Vita has no physical L2/R2, so those triggers only exist as the rear touch panel. That
/// makes "rear panel drives the mouse" and "rear panel drives L2/R2" mutually exclusive, which
/// is exactly why this is a profile switch rather than a pile of independent toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlProfile {
    /// Pad is handed to the title untouched: rear panel = L2/R2, front bottom corners = L3/R3,
    /// full d-pad. The overlay draws only the eye toggle and never eats a button.
    #[default]
    Game,
    /// Pad and rear panel are repurposed as a mouse/keyboard for the Windows desktop.
    Desktop,
}

impl ControlProfile {
    pub const ALL: [ControlProfile; 2] = [Self::Game, Self::Desktop];

    pub fn key(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::Desktop => "desktop",
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Game => "settings-control-profile-game",
            Self::Desktop => "settings-control-profile-desktop",
        }
    }

    fn from_key(key: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.key() == key.trim())
            .unwrap_or_default()
    }
}

pub fn control_profile() -> ControlProfile {
    ControlProfile::from_key(&load_or_init_settings().control_profile)
}

pub fn set_control_profile(profile: ControlProfile) {
    update_settings(|s| s.control_profile = profile.key().to_owned());
}

/// True when the overlay's zones and control panel are expanded. The eye toggle itself stays
/// drawn either way so the player can always get back.
pub fn overlay_revealed() -> bool {
    load_or_init_settings().overlay_revealed
}

pub fn set_overlay_revealed(revealed: bool) {
    update_settings(|s| s.overlay_revealed = revealed);
}

/// How opaque the overlay's on-screen buttons/sliders are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayOpacity {
    Ghost,
    Faint,
    #[default]
    Normal,
    Strong,
    Bold,
}

impl OverlayOpacity {
    pub const ALL: [OverlayOpacity; 5] = [
        Self::Ghost,
        Self::Faint,
        Self::Normal,
        Self::Strong,
        Self::Bold,
    ];

    /// egui alpha byte (0..255) applied to the overlay's fills/strokes/text. The overlay is now
    /// visible by default, so the scale starts far lower than it used to: the point is to keep
    /// the buttons readable without washing out the 960x544 picture behind them.
    pub fn alpha(self) -> u8 {
        match self {
            Self::Ghost => 40,
            Self::Faint => 70,
            Self::Normal => 110,
            Self::Strong => 160,
            Self::Bold => 210,
        }
    }

    /// 0.0..1.0 multiplier for egui widgets that take a gamma multiply rather than a raw alpha,
    /// such as the on-screen keyboard's panel and key fills.
    pub fn multiplier(self) -> f32 {
        f32::from(self.alpha()) / 255.0
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Ghost => "settings-overlay-opacity-ghost",
            Self::Faint => "settings-overlay-opacity-faint",
            Self::Normal => "settings-overlay-opacity-normal",
            Self::Strong => "settings-overlay-opacity-strong",
            Self::Bold => "settings-overlay-opacity-bold",
        }
    }

    /// Maps a stored raw alpha back onto the nearest preset, so settings written by an older
    /// build (which only had 60/120/200) still land somewhere sensible.
    fn from_value(value: u8) -> Self {
        Self::ALL
            .into_iter()
            .min_by_key(|candidate| candidate.alpha().abs_diff(value))
            .unwrap_or_default()
    }
}

pub fn overlay_opacity() -> OverlayOpacity {
    OverlayOpacity::from_value(load_or_init_settings().pc_overlay_alpha)
}

pub fn set_overlay_opacity(opacity: OverlayOpacity) {
    update_settings(|s| s.pc_overlay_alpha = opacity.alpha());
}

/// Rear-panel trackpad sensitivity. The NVST protocol has no absolute-position packet (see
/// `INPUT_MOUSE_MOVE_REL`'s doc comment in `gfn::input_protocol`), so the rear panel drives the
/// cursor as relative deltas; this multiplier is the only "DPI" knob available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlaySensitivity {
    Low,
    #[default]
    Normal,
    High,
    Max,
}

impl OverlaySensitivity {
    pub const ALL: [OverlaySensitivity; 4] = [Self::Low, Self::Normal, Self::High, Self::Max];

    /// Multiplier applied to the raw rear-touch delta, in percent (100 = 1x).
    pub fn percent(self) -> u16 {
        match self {
            Self::Low => 50,
            Self::Normal => 100,
            Self::High => 175,
            Self::Max => 250,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Low => "settings-overlay-sensitivity-low",
            Self::Normal => "settings-overlay-sensitivity-normal",
            Self::High => "settings-overlay-sensitivity-high",
            Self::Max => "settings-overlay-sensitivity-max",
        }
    }

    fn from_percent(percent: u16) -> Self {
        match percent {
            p if p <= 60 => Self::Low,
            p if p <= 120 => Self::Normal,
            p if p <= 200 => Self::High,
            _ => Self::Max,
        }
    }
}

pub fn overlay_sensitivity() -> OverlaySensitivity {
    OverlaySensitivity::from_percent(load_or_init_settings().overlay_sensitivity_percent)
}

pub fn set_overlay_sensitivity(sensitivity: OverlaySensitivity) {
    update_settings(|s| s.overlay_sensitivity_percent = sensitivity.percent());
}

/// Raw sensitivity percent (100 = 1x), for the rear-panel trackpad's per-motion-event scaling -
/// see `scale_rear_delta` in `crate::input`. Distinct from `overlay_sensitivity()`'s coarse
/// enum, which only exists to give the settings menu's chip picker four presets to choose from.
pub fn overlay_sensitivity_percent() -> u16 {
    load_or_init_settings().overlay_sensitivity_percent
}

/// Nudges the raw sensitivity percent by `delta_percent`, clamped to a sane range. Used by the
/// front-screen DPI slider zone, which drags continuously rather than picking one of the four
/// enum presets.
pub fn adjust_overlay_sensitivity_percent(delta_percent: i32) -> u16 {
    const MIN_PERCENT: i32 = 10;
    const MAX_PERCENT: i32 = 400;
    let current = i32::from(load_or_init_settings().overlay_sensitivity_percent);
    let next = (current + delta_percent).clamp(MIN_PERCENT, MAX_PERCENT) as u16;
    update_settings(|s| s.overlay_sensitivity_percent = next);
    next
}

#[cfg(test)]
mod tests {
    use super::AppSettings;

    #[test]
    fn settings_json_without_ui_locale_still_loads() {
        let json = r#"{
            "fps": 60,
            "trigger_intensity": 255,
            "audio_boost_percent": 1200,
            "controls_hint_seen": false,
            "stick_zones": "hidden"
        }"#;
        let settings: AppSettings = serde_json::from_str(json).expect("0.3.1 settings should load");
        assert!(settings.ui_locale.is_empty());
        assert_eq!(
            crate::locale::Locale::from_str(&settings.ui_locale),
            crate::locale::Locale::EnUs
        );
    }

    #[test]
    fn color_depth_stream_bit_depth_is_h264_8bit() {
        assert_eq!(super::ColorDepth::SixteenBit.stream_bit_depth(), 8);
        assert_eq!(super::ColorDepth::ThirtyTwoBit.stream_bit_depth(), 8);
    }

    #[test]
    fn settings_json_without_overlay_fields_still_loads_with_sane_defaults() {
        let json = r#"{
            "fps": 60,
            "trigger_intensity": 255,
            "audio_boost_percent": 1200,
            "controls_hint_seen": false,
            "stick_zones": "hidden"
        }"#;
        let settings: AppSettings =
            serde_json::from_str(json).expect("pre-overlay settings should load");
        // v0.5.0 flipped this default on: the overlay shipping off is precisely why players
        // never saw it and kept using the legacy front-screen trackpad instead.
        assert!(settings.pc_overlay_enabled);
        assert!(settings.overlay_revealed);
        assert_eq!(
            super::ControlProfile::from_key(&settings.control_profile),
            super::ControlProfile::Game
        );
        assert_eq!(
            settings.pc_overlay_alpha,
            super::OverlayOpacity::Normal.alpha()
        );
        assert_eq!(
            settings.overlay_sensitivity_percent,
            super::OverlaySensitivity::Normal.percent()
        );
    }

    #[test]
    fn control_profile_round_trips_through_its_key() {
        for profile in super::ControlProfile::ALL {
            assert_eq!(super::ControlProfile::from_key(profile.key()), profile);
        }
        assert_eq!(
            super::ControlProfile::from_key("nonsense"),
            super::ControlProfile::Game
        );
    }

    #[test]
    fn legacy_overlay_alpha_values_snap_to_the_nearest_new_preset() {
        // The 0.4.x scale only had 60/120/200; those stored bytes must still map somewhere sane
        // rather than silently resetting to the default.
        assert_eq!(
            super::OverlayOpacity::from_value(60),
            super::OverlayOpacity::Faint
        );
        assert_eq!(
            super::OverlayOpacity::from_value(120),
            super::OverlayOpacity::Normal
        );
        assert_eq!(
            super::OverlayOpacity::from_value(200),
            super::OverlayOpacity::Bold
        );
    }

    #[test]
    fn overlay_opacity_round_trips_through_its_alpha_byte() {
        for opacity in super::OverlayOpacity::ALL {
            assert_eq!(super::OverlayOpacity::from_value(opacity.alpha()), opacity);
        }
    }

    #[test]
    fn overlay_sensitivity_round_trips_through_its_percent() {
        for sensitivity in super::OverlaySensitivity::ALL {
            assert_eq!(
                super::OverlaySensitivity::from_percent(sensitivity.percent()),
                sensitivity
            );
        }
    }

    #[test]
    fn settings_json_without_login_provider_field_defaults_to_forcing_direct_nvidia_login() {
        // Old settings.json files predate this field entirely; a user upgrading must not be
        // silently switched to their ISP's regional GFN partner (e.g. Digevo) just because the
        // key was absent on disk.
        let json = r#"{
            "fps": 60,
            "trigger_intensity": 255,
            "audio_boost_percent": 1200,
            "controls_hint_seen": false,
            "stick_zones": "hidden"
        }"#;
        let settings: AppSettings =
            serde_json::from_str(json).expect("pre-login-provider settings should load");
        assert!(settings.force_direct_nvidia_login);
    }
}
