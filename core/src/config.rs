//! The subset of the player's settings that the input mapping actually depends on.
//!
//! Deliberately a plain value type rather than a call into `gfn::stream_prefs`. Reading a
//! preference used to mean cloning the entire `AppSettings` struct - a `BTreeMap` plus eight
//! `String`s - and that happened once per SDL event inside the event loop. Passing a `Copy`
//! snapshot down instead means the mapping allocates nothing at all, and it is what lets the
//! mapper be a pure function.

/// Which control profile the stream is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlProfile {
    /// Every stick, button, trigger zone and stick-click reaches the title untouched.
    #[default]
    Game,
    /// The pad and the rear panel become a mouse and keyboard for the Windows desktop.
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

    /// Unknown text falls back to `Game`: a corrupt or future settings file must not strand the
    /// player in a profile where the pad does not reach the title.
    pub fn from_key(key: &str) -> Self {
        match key {
            "desktop" => Self::Desktop,
            _ => Self::Game,
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Game => Self::Desktop,
            Self::Desktop => Self::Game,
        }
    }
}

/// How the rear panel is split in the game profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RearTouchMode {
    /// Left half is L2, right half is R2. The only layout with zones big enough to find by feel.
    #[default]
    Halves,
    /// Top halves are L2/R2, bottom halves L3/R3. Kept for players who had it set; each zone is
    /// about 1.2 cm tall with no landmark, so reaching for L2 can give L3.
    Quadrant,
}

/// Everything the input mapping needs to know about the player's settings, as one `Copy` value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputConfig {
    pub profile: ControlProfile,
    /// Master switch for the whole on-screen overlay.
    pub overlay_enabled: bool,
    /// Whether the overlay is expanded or collapsed to just the eye.
    pub overlay_revealed: bool,
    /// Pointer speed, 100 = 1x.
    pub sensitivity_percent: u16,
    /// Whether the front screen's bottom corners stand in for L3/R3.
    pub stick_zones_active: bool,
    /// How hard a held rear-panel trigger zone presses, 0-255.
    pub trigger_pressure: u8,
    pub rear_touch_mode: RearTouchMode,
    /// Swaps L1/L2 with R1/R2, for titles that assume a pad with real triggers.
    pub trigger_swap: bool,
    /// Front-screen trackpad. Only ever consulted in the game profile - see `router`.
    pub front_trackpad: bool,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            profile: ControlProfile::Game,
            overlay_enabled: true,
            overlay_revealed: true,
            sensitivity_percent: 100,
            stick_zones_active: true,
            trigger_pressure: 255,
            rear_touch_mode: RearTouchMode::Halves,
            trigger_swap: false,
            front_trackpad: true,
        }
    }
}

impl InputConfig {
    /// True when the desktop profile's zones and pad remapping are live.
    pub fn desktop_active(self) -> bool {
        self.overlay_enabled && self.profile == ControlProfile::Desktop
    }

    /// True when the overlay's key zones should respond to touch.
    pub fn zones_live(self) -> bool {
        self.overlay_enabled && self.overlay_revealed
    }
}
