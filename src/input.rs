//! Input mapping: SDL2 events (keyboard, Vita controller, touch) -> app-level commands.
//!
//! The Vita controller mapping GUID string and the front/rear touch device ids below are
//! adapted from `green-vita` (MPL-2.0, https://github.com/Day-OS/green-vita) - see
//! `THIRD_PARTY_NOTICES.md`. They encode non-obvious platform quirks (SDL's built-in Vita
//! controller driver does not ship a default `SDL_GameControllerDB` mapping, and VitaSDK's
//! `SDL_vitatouch.c` registers the front/back touch panels as SDL touch devices 1 and 2, in
//! that order) that are not documented anywhere else and are cheaper to keep than to
//! rediscover.

use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;

/// Directional/confirm navigation, independent of what screen is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputCommand {
    Back,
    Confirm,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    PrevTab,
    NextTab,
}

/// Top-level command enum the shell feeds into `App::handle_command`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    Input(InputCommand),
    SetSearchQuery(String),
    /// Ask the shell to open the platform text-input method (SDL IME / on-screen keyboard).
    RequestSearch,
    /// Close/submit search input and stop the platform text-input method.
    CloseSearch,
    ToggleConfirmExit,
    CancelConfirmExit,
    ConfirmExitSession,
    /// Emitted by the catalog screen's language picker.
    SetLocale(crate::locale::Locale),
    /// Emitted by the catalog screen's sort picker.
    SetSort(crate::app::CatalogSort),
    SetFilter(crate::app::CatalogFilter),
    /// Emitted by the stream-quality section of the account popup.
    SetStreamFps(crate::gfn::stream_prefs::StreamFps),
    ToggleSessionTimer,
    /// Emitted by the rear-trigger section of the account popup.
    SetTriggerIntensity(crate::gfn::stream_prefs::TriggerIntensity),
    /// Closes the first-run controls explainer for good.
    DismissControlsHint,
    /// Stars or unstars a game from its row in the library.
    ToggleFavorite(String),
    /// Shows or hides the streaming diagnostics panel.
    ToggleStreamStats,
    /// Shows or hides the in-game keyboard.
    ToggleKeyboard,
    SendKey(crate::gfn::input_protocol::KeyStroke),
    SendChord {
        ctrl: bool,
        alt: bool,
        /// Holds the Left Windows key for the duration of the chord, e.g. for Win+D. Added
        /// alongside ctrl/alt rather than as a fourth boolean-per-modifier field elsewhere,
        /// since every existing chord (Ctrl+Alt+Del, Alt+Tab, ...) only ever needed those two.
        win: bool,
        key: crate::gfn::input_protocol::KeyStroke,
    },
    ToggleKeyShift,
    ToggleKeyCtrl,
    ToggleKeyAlt,
    /// Emitted by the stick-zone section of the settings modal.
    SetStickZones(crate::gfn::stream_prefs::StickZones),
    /// from the rear-touch layout picker in settings
    SetRearTouchMode(crate::gfn::stream_prefs::RearTouchMode),
    /// Emitted by the volume-boost section of the account popup.
    SetAudioBoost(crate::gfn::stream_prefs::AudioBoost),
    SetColorDepth(crate::gfn::stream_prefs::ColorDepth),
    /// Emitted when a row in the catalog list is tapped/clicked.
    SelectGame(usize),
    /// Toggles the streaming toolbar between expanded and collapsed.
    ToggleToolbar,
    /// Emits a momentary right-click to the host.
    RightClick,
    /// Toggles the in-stream controls configuration modal (L2/R2 and L3/R3).
    ToggleControlsModal,
    /// Toggles front touch trackpad host mouse input on and off.
    ToggleMouseTrackpad,
    /// Master switch for the PC-touch overlay mod (rear-panel mouse + front-panel
    /// ESC/settings/DPI/scroll/enter/click zones). See `overlay_zone_at` below.
    TogglePcOverlay,
    SetOverlayOpacity(crate::gfn::stream_prefs::OverlayOpacity),
    SetOverlaySensitivity(crate::gfn::stream_prefs::OverlaySensitivity),
    /// Switches between the game and desktop control profiles - see
    /// `stream_prefs::ControlProfile`.
    SetControlProfile(crate::gfn::stream_prefs::ControlProfile),
    /// bumps the bitrate mid session, kbps
    SetMaxBitrate(u32),
    SetRegion(String),
    LoadRegions,
    TestRegionLatency,
    OpenSettings,
    CloseSettings,
    SetSettingsTab(crate::app::settings_menu::SettingsTab),
    ExpandSettingsRow(Option<usize>),
    ChooseSettingsOption(usize, usize),
    ToggleGameProfile,
    ToggleTriggerSwap,
    SetGameLanguage(crate::gfn::stream_prefs::GameLanguage),
    /// Emitted by the Account tab; toggles bypassing regional-partner idp discovery (e.g. a
    /// Peruvian ISP's "GeForce NOW powered by Digevo" reseller) and forcing NVIDIA's own login.
    ToggleForceDirectNvidiaLogin,
    CloseServerPicker,
    FocusServerPicker(usize),
    LaunchOnServer(String),
    LoadQueueStats,
}

impl From<InputCommand> for AppCommand {
    fn from(command: InputCommand) -> Self {
        AppCommand::Input(command)
    }
}

/// The front touchscreen's own SDL touch device id (registered second on the Vita's touch
/// backend, `SDL_vitatouch.c`: `SDL_AddTouch(1, ..., "Front")`, `SDL_AddTouch(2, ..., "Back")`).
pub const FRONT_TOUCH_DEVICE_ID: i64 = 1;

/// The rear touch panel, registered right after the front one by `SDL_vitatouch.c`.
const REAR_TOUCH_DEVICE_ID: i64 = 2;

pub fn map_keyboard_event(event: &Event) -> Option<AppCommand> {
    let Event::KeyDown {
        keycode: Some(key),
        repeat: false,
        ..
    } = event
    else {
        return None;
    };
    let command = match *key {
        Keycode::Escape => InputCommand::Back,
        Keycode::Return => InputCommand::Confirm,
        Keycode::Up => InputCommand::MoveUp,
        Keycode::Down => InputCommand::MoveDown,
        Keycode::Left => InputCommand::MoveLeft,
        Keycode::Right => InputCommand::MoveRight,
        _ => return None,
    };
    Some(command.into())
}

pub fn map_controller_button_event(event: &Event) -> Option<AppCommand> {
    match event {
        Event::ControllerButtonDown {
            button: Button::B, ..
        } => Some(InputCommand::Back.into()),
        Event::ControllerButtonDown {
            button: Button::A, ..
        } => Some(InputCommand::Confirm.into()),
        Event::ControllerButtonDown {
            button: Button::LeftShoulder,
            ..
        } => Some(InputCommand::PrevTab.into()),
        Event::ControllerButtonDown {
            button: Button::RightShoulder,
            ..
        } => Some(InputCommand::NextTab.into()),
        _ => None,
    }
}

const MENU_STICK_DEADZONE: f32 = 0.5;

/// Polled every frame (rather than event-driven) so holding a direction auto-repeats; see the
/// repeat-delay/interval constants in `shell::run`.
pub fn held_menu_direction(controller: Option<&GameController>) -> Option<InputCommand> {
    let controller = controller?;
    if controller.button(Button::DPadUp) {
        return Some(InputCommand::MoveUp);
    }
    if controller.button(Button::DPadDown) {
        return Some(InputCommand::MoveDown);
    }
    if controller.button(Button::DPadLeft) {
        return Some(InputCommand::MoveLeft);
    }
    if controller.button(Button::DPadRight) {
        return Some(InputCommand::MoveRight);
    }
    let x = axis_to_f32(controller.axis(Axis::LeftX));
    let y = axis_to_f32(controller.axis(Axis::LeftY));
    if y.abs() >= x.abs() {
        match y {
            y if y <= -MENU_STICK_DEADZONE => Some(InputCommand::MoveUp),
            y if y >= MENU_STICK_DEADZONE => Some(InputCommand::MoveDown),
            _ => None,
        }
    } else {
        match x {
            x if x <= -MENU_STICK_DEADZONE => Some(InputCommand::MoveLeft),
            x if x >= MENU_STICK_DEADZONE => Some(InputCommand::MoveRight),
            _ => None,
        }
    }
}

fn axis_to_f32(raw: i16) -> f32 {
    raw as f32 / i16::MAX as f32
}

/// SDL's built-in Vita controller driver reports raw joystick buttons/axes but does not ship a
/// `SDL_GameControllerDB` mapping for the Vita's own front controls, so `GameControllerSubsystem`
/// would otherwise never recognize it as a "game controller".
pub fn register_vita_controller_mapping(sdl: &sdl2::Sdl) -> Result<(), String> {
    let joystick_subsystem = sdl.joystick()?;
    if joystick_subsystem
        .num_joysticks()
        .map_err(|e| e.to_string())?
        == 0
    {
        return Ok(());
    }
    let guid = joystick_subsystem
        .device_guid(0)
        .map_err(|e| e.to_string())?;
    let mapping = format!(
        "{guid},PSVita Controller,\
         a:b2,b:b1,x:b3,y:b0,\
         back:b10,start:b11,\
         leftshoulder:b4,rightshoulder:b5,\
         leftstick:b14,rightstick:b15,\
         dpup:b8,dpdown:b6,dpleft:b7,dpright:b9,\
         leftx:a0,lefty:a1,rightx:a2,righty:a3,\
         lefttrigger:b12,righttrigger:b13,platform:PS Vita,"
    );
    sdl.game_controller()?
        .add_mapping(&mapping)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn open_first_controller(subsystem: &sdl2::GameControllerSubsystem) -> Option<GameController> {
    let available = subsystem.num_joysticks().ok()?;
    (0..available).find_map(|id| {
        if !subsystem.is_game_controller(id) {
            return None;
        }
        subsystem.open(id).ok()
    })
}

/// Forwards mouse/front-touch input to egui.
pub fn map_pointer_event(
    event: &Event,
    screen_size: (f32, f32),
    pixels_per_point: f32,
    pointer_pos: &mut egui::Pos2,
) -> Option<egui::Event> {
    match *event {
        Event::MouseMotion { x, y, .. } => {
            *pointer_pos = mouse_to_screen_pos(x, y, pixels_per_point);
            Some(egui::Event::PointerMoved(*pointer_pos))
        }
        Event::MouseButtonDown {
            mouse_btn, x, y, ..
        } => Some(pointer_button_at(
            pointer_pos,
            mouse_to_screen_pos(x, y, pixels_per_point),
            map_mouse_button(mouse_btn),
            true,
        )),
        Event::MouseButtonUp {
            mouse_btn, x, y, ..
        } => Some(pointer_button_at(
            pointer_pos,
            mouse_to_screen_pos(x, y, pixels_per_point),
            map_mouse_button(mouse_btn),
            false,
        )),
        Event::FingerDown { touch_id, x, y, .. } if touch_id == FRONT_TOUCH_DEVICE_ID => {
            Some(pointer_button_at(
                pointer_pos,
                touch_to_screen_pos(x, y, screen_size),
                egui::PointerButton::Primary,
                true,
            ))
        }
        Event::FingerMotion { touch_id, x, y, .. } if touch_id == FRONT_TOUCH_DEVICE_ID => {
            *pointer_pos = touch_to_screen_pos(x, y, screen_size);
            Some(egui::Event::PointerMoved(*pointer_pos))
        }
        Event::FingerUp { touch_id, x, y, .. } if touch_id == FRONT_TOUCH_DEVICE_ID => {
            Some(pointer_button_at(
                pointer_pos,
                touch_to_screen_pos(x, y, screen_size),
                egui::PointerButton::Primary,
                false,
            ))
        }
        _ => None,
    }
}

/// Front-touch -> host mouse, for the streaming screen.
///
/// The NVST input protocol has no absolute-position packet, only relative deltas, so the
/// touchscreen is driven as a trackpad rather than as a direct-touch pointer: dragging a finger
/// moves the cursor by the distance dragged, and a short tap that barely moves is a left click.
/// Rear touch is deliberately left alone - it sits under the player's fingers during normal play.
#[derive(Default)]
pub struct StreamTouchState {
    /// Where the finger was on the previous event, in normalized 0..1 touch coordinates.
    last: Option<(f32, f32)>,
    /// Accumulated travel since finger-down, to tell a tap from a drag.
    travel: f32,
    down_at: Option<std::time::Instant>,
}

/// How far (in normalized touch units) a finger may travel and still count as a tap.
const TAP_MAX_TRAVEL: f32 = 0.03;
/// How long a contact may last and still count as a tap.
const TAP_MAX_DURATION: std::time::Duration = std::time::Duration::from_millis(300);

impl StreamTouchState {
    /// Translates one SDL event into the host-mouse events it implies, if any.
    pub fn map(
        &mut self,
        event: &Event,
        stream_size: (f32, f32),
    ) -> Vec<crate::gfn::input_protocol::MouseEvent> {
        use crate::gfn::input_protocol::{MouseButton as StreamMouseButton, MouseEvent};

        let mut out = Vec::new();
        match *event {
            Event::FingerDown { touch_id, x, y, .. } if touch_id == FRONT_TOUCH_DEVICE_ID => {
                self.last = Some((x, y));
                self.travel = 0.0;
                self.down_at = Some(std::time::Instant::now());
            }
            Event::FingerMotion { touch_id, x, y, .. } if touch_id == FRONT_TOUCH_DEVICE_ID => {
                if let Some((prev_x, prev_y)) = self.last {
                    let (dx, dy) = (x - prev_x, y - prev_y);
                    self.travel += (dx * dx + dy * dy).sqrt();
                    // Normalized touch delta scaled into host pixels.
                    let move_x = (dx * stream_size.0).round() as i32;
                    let move_y = (dy * stream_size.1).round() as i32;
                    if move_x != 0 || move_y != 0 {
                        out.push(MouseEvent::MoveBy {
                            dx: move_x.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                            dy: move_y.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                        });
                    }
                }
                self.last = Some((x, y));
            }
            Event::FingerUp { touch_id, .. } if touch_id == FRONT_TOUCH_DEVICE_ID => {
                let tapped = self.travel <= TAP_MAX_TRAVEL
                    && self
                        .down_at
                        .is_some_and(|at| at.elapsed() <= TAP_MAX_DURATION);
                if tapped {
                    out.push(MouseEvent::Button {
                        button: StreamMouseButton::Left,
                        pressed: true,
                    });
                    out.push(MouseEvent::Button {
                        button: StreamMouseButton::Left,
                        pressed: false,
                    });
                }
                self.last = None;
                self.travel = 0.0;
                self.down_at = None;
            }
            _ => {}
        }
        out
    }
}

/// Where a front-touch event landed, in egui points - used to decide whether a touch belongs to
/// the client's own on-screen controls or to the game behind them.
pub fn front_touch_position(event: &Event, screen_size: (f32, f32)) -> Option<egui::Pos2> {
    match *event {
        Event::FingerDown { touch_id, x, y, .. }
        | Event::FingerMotion { touch_id, x, y, .. }
        | Event::FingerUp { touch_id, x, y, .. }
            if touch_id == FRONT_TOUCH_DEVICE_ID =>
        {
            Some(touch_to_screen_pos(x, y, screen_size))
        }
        _ => None,
    }
}

fn pointer_button_at(
    pointer_pos: &mut egui::Pos2,
    pos: egui::Pos2,
    button: egui::PointerButton,
    pressed: bool,
) -> egui::Event {
    *pointer_pos = pos;
    egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::default(),
    }
}

fn mouse_to_screen_pos(x: i32, y: i32, pixels_per_point: f32) -> egui::Pos2 {
    egui::pos2(x as f32 / pixels_per_point, y as f32 / pixels_per_point)
}

fn touch_to_screen_pos(x: f32, y: f32, (width, height): (f32, f32)) -> egui::Pos2 {
    egui::pos2(x * width, y * height)
}

fn map_mouse_button(button: MouseButton) -> egui::PointerButton {
    match button {
        MouseButton::Right => egui::PointerButton::Secondary,
        MouseButton::Middle => egui::PointerButton::Middle,
        _ => egui::PointerButton::Primary,
    }
}

/// The Vita has no L2/R2. Its shoulder buttons are L1/R1, and the controller mapping's
/// `lefttrigger:b12,righttrigger:b13` only resolves on a Vita TV with a DualShock attached - on a
/// handheld those buttons do not exist, so the triggers were simply unreachable and any game
/// bound to them was unplayable.
///

/// Live diagnostics for the front stick zones, so a "L3/R3 doesn't work" report can be checked
/// against what the touch router actually decided instead of guessed at from source.
pub mod stick_zone_stats {
    use std::sync::atomic::{AtomicBool, Ordering};

    static TOUCH_OWNED_BY_ZONE: AtomicBool = AtomicBool::new(false);
    static LEFT_ACTIVE: AtomicBool = AtomicBool::new(false);
    static RIGHT_ACTIVE: AtomicBool = AtomicBool::new(false);

    pub(crate) fn record_touch_owned(owned: bool) {
        TOUCH_OWNED_BY_ZONE.store(owned, Ordering::Relaxed);
    }

    pub(crate) fn record_clicks(left: bool, right: bool) {
        LEFT_ACTIVE.store(left, Ordering::Relaxed);
        RIGHT_ACTIVE.store(right, Ordering::Relaxed);
    }

    pub fn line() -> String {
        format!(
            "inp zones:{} touch_owned:{} l3:{} r3:{}",
            crate::gfn::stream_prefs::stick_zones().debug_label(),
            u8::from(TOUCH_OWNED_BY_ZONE.load(Ordering::Relaxed)),
            u8::from(LEFT_ACTIVE.load(Ordering::Relaxed)),
            u8::from(RIGHT_ACTIVE.load(Ordering::Relaxed)),
        )
    }
}

/// The front screen's bottom corners, standing in for the stick clicks the Vita has no buttons for.
///
/// The rear panel was tried first and gave the whole panel away to four ~1.2 cm zones with nothing
/// to tell them apart by feel. The front screen has room: two corners the thumb already rests near,
/// with the middle of the screen - where the game is - left alone.
#[derive(Default)]
pub struct FrontStickZones {
    /// Touch positions by SDL finger id, normalized 0..1. Multi-touch, so both clicks can be held
    /// at once and releasing one must not cancel the other.
    fingers: Vec<(i64, f32, f32)>,
    enabled: bool,
}

/// Where the zones sit, normalized 0..1. Bottom third, outer quarter of each side.
pub const STICK_ZONE_TOP: f32 = 0.66;
pub const STICK_ZONE_WIDTH: f32 = 0.25;

/// Whether a normalized position falls in one of the zones.
fn in_stick_zone(x: f32, y: f32, left: bool) -> bool {
    y >= STICK_ZONE_TOP
        && if left {
            x < STICK_ZONE_WIDTH
        } else {
            x >= 1.0 - STICK_ZONE_WIDTH
        }
}

/// Whether a normalized position falls in *either* zone - what the touch router asks before
/// handing a gesture to the host's mouse.
pub fn is_in_stick_zone(x: f32, y: f32) -> bool {
    in_stick_zone(x, y, true) || in_stick_zone(x, y, false)
}

impl FrontStickZones {
    pub fn handle(&mut self, event: &Event) {
        match *event {
            Event::FingerDown {
                touch_id,
                finger_id,
                x,
                y,
                ..
            }
            | Event::FingerMotion {
                touch_id,
                finger_id,
                x,
                y,
                ..
            } if touch_id == FRONT_TOUCH_DEVICE_ID => {
                match self.fingers.iter_mut().find(|(id, _, _)| *id == finger_id) {
                    Some(slot) => {
                        slot.1 = x;
                        slot.2 = y;
                    }
                    None => self.fingers.push((finger_id, x, y)),
                }
            }
            Event::FingerUp {
                touch_id,
                finger_id,
                ..
            } if touch_id == FRONT_TOUCH_DEVICE_ID => {
                self.fingers.retain(|(id, _, _)| *id != finger_id);
            }
            _ => {}
        }
    }

    /// Picks up a changed setting; call when a session starts, alongside `reload_intensity`.
    pub fn reload_enabled(&mut self) {
        self.enabled = crate::gfn::stream_prefs::stick_zones().is_active();
        if !self.enabled {
            self.fingers.clear();
        }
    }

    fn held(&self, left: bool) -> bool {
        self.enabled
            && self
                .fingers
                .iter()
                .any(|(_, x, y)| in_stick_zone(*x, *y, left))
    }

    /// L3, from the bottom-left corner of the screen.
    pub fn left_stick_click(&self) -> bool {
        self.held(true)
    }

    /// R3, from the bottom-right corner.
    pub fn right_stick_click(&self) -> bool {
        self.held(false)
    }
}

/// One fixed hit-zone of the PC-touch overlay's front screen.
///
/// Layout rationale (v0.5.0): the previous design put zones on all four edges *and* both bottom
/// corners, which meant the overlay bracketed the picture on every side and stole the corners
/// that `FrontStickZones` needs for L3/R3. The zones now live in two thin strips along the top
/// and bottom plus two narrow slider rails, leaving the middle of the 960x544 panel - where the
/// game actually is - completely clear.
///
/// Every zone except `Eye` is active only in [`ControlProfile::Desktop`] while the overlay is
/// revealed. `Eye` is always live whenever the overlay is enabled at all, so the player can
/// never lock themselves out of the toggle.
///
/// [`ControlProfile::Desktop`]: crate::gfn::stream_prefs::ControlProfile::Desktop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcOverlayZone {
    /// Top-right corner, drawn in both profiles: shows/hides everything else.
    Eye,
    /// Directly under the eye, live in both profiles while revealed: swaps game/desktop.
    ModeToggle,

    // --- top strip, left to right ---
    Esc,
    Tab,
    /// Taps the Left Windows key on its own, i.e. opens the Start menu.
    Win,
    AltTab,
    Copy,
    Paste,
    /// Shows/hides the on-screen keyboard (reuses `AppCommand::ToggleKeyboard`).
    Keyboard,
    /// Opens the native settings menu; while it is open the modal claims touch via
    /// `stream_ui_rects`, so input stops reaching the game.
    OpenSettings,

    // --- bottom strip, left to right ---
    /// Sticky modifiers, shared with the on-screen keyboard's own shift/ctrl/alt state.
    Shift,
    Ctrl,
    Alt,
    Enter,
    Backspace,
    CtrlAltDel,

    // --- slider rails ---
    /// Left rail: dragging up/down raises/lowers the rear-panel mouse's DPI.
    DpiSlider,
    /// Right rail: dragging up/down scrolls the mouse wheel.
    ScrollSlider,
}

/// Height of the top and bottom key strips, as a fraction of the 544 px panel (~71 px each).
/// Big enough to hit with a thumb without a stylus, small enough to leave ~400 px of clear
/// picture between them.
const OVERLAY_STRIP_HEIGHT: f32 = 0.13;
/// Left edge of the always-on eye toggle. The top strip stops here so the two can never overlap.
const OVERLAY_EYE_LEFT: f32 = 0.88;
/// How many cells the top and bottom strips are divided into.
const OVERLAY_TOP_CELLS: usize = 8;
const OVERLAY_BOTTOM_CELLS: usize = 6;
/// Width of the DPI / scroll slider rails, reaching in from the left and right edges.
const OVERLAY_RAIL_WIDTH: f32 = 0.07;
/// Vertical span of both slider rails. Kept well inside the strips so a thumb sliding off the
/// end of a rail cannot accidentally land on a key.
const OVERLAY_RAIL_TOP: f32 = 0.30;
const OVERLAY_RAIL_BOTTOM: f32 = 0.72;

/// Normalized `(x0, y0, x1, y1)` box of the always-visible eye toggle.
pub const OVERLAY_EYE_RECT: (f32, f32, f32, f32) =
    (OVERLAY_EYE_LEFT, 0.0, 1.0, OVERLAY_STRIP_HEIGHT);

/// Normalized box of the profile switch, sitting directly under the eye. Only live while the
/// overlay is revealed, but live in *both* profiles - it is the only touch route from the game
/// profile (where the key strips are dead) back into the desktop profile. It sits below the top
/// strip and above `OVERLAY_RAIL_TOP`, so it collides with nothing in either profile.
pub const OVERLAY_MODE_RECT: (f32, f32, f32, f32) = (
    OVERLAY_EYE_LEFT,
    OVERLAY_STRIP_HEIGHT,
    1.0,
    OVERLAY_STRIP_HEIGHT * 2.0,
);

/// True when a normalized (0..1) front-touch lands on the profile switch.
pub fn overlay_mode_at(x: f32, y: f32) -> bool {
    let (x0, y0, x1, y1) = OVERLAY_MODE_RECT;
    (x0..x1).contains(&x) && (y0..y1).contains(&y)
}

const OVERLAY_TOP_ZONES: [(PcOverlayZone, &str); OVERLAY_TOP_CELLS] = [
    (PcOverlayZone::Esc, "ESC"),
    (PcOverlayZone::Tab, "TAB"),
    (PcOverlayZone::Win, "\u{229e}"),
    (PcOverlayZone::AltTab, "ALT\u{21b9}"),
    (PcOverlayZone::Copy, "COPY"),
    (PcOverlayZone::Paste, "PASTE"),
    (PcOverlayZone::Keyboard, "\u{2328}"),
    (PcOverlayZone::OpenSettings, "\u{2699}"),
];

const OVERLAY_BOTTOM_ZONES: [(PcOverlayZone, &str); OVERLAY_BOTTOM_CELLS] = [
    (PcOverlayZone::Shift, "SHIFT"),
    (PcOverlayZone::Ctrl, "CTRL"),
    (PcOverlayZone::Alt, "ALT"),
    (PcOverlayZone::Enter, "\u{23ce}"),
    (PcOverlayZone::Backspace, "\u{232b}"),
    (PcOverlayZone::CtrlAltDel, "C-A-DEL"),
];

/// True when a normalized (0..1) front-touch position lands on the eye toggle. Checked before
/// [`overlay_zone_at`] and before the stick zones, in both control profiles.
pub fn overlay_eye_at(x: f32, y: f32) -> bool {
    let (x0, y0, x1, y1) = OVERLAY_EYE_RECT;
    (x0..x1).contains(&x) && (y0..y1).contains(&y)
}

/// Maps a normalized (0..1) front-touch position to the desktop-profile overlay zone it lands
/// in, or `None` if it is over the clear middle of the screen - i.e. still the game's or the
/// trackpad's touch, not the overlay's.
///
/// Deliberately excludes [`PcOverlayZone::Eye`]; callers must test that separately with
/// [`overlay_eye_at`] so the toggle keeps working while the rest of the overlay is hidden.
pub fn overlay_zone_at(x: f32, y: f32) -> Option<PcOverlayZone> {
    if !(0.0..1.0).contains(&x) || !(0.0..1.0).contains(&y) {
        return None;
    }
    if y < OVERLAY_STRIP_HEIGHT {
        if x >= OVERLAY_EYE_LEFT {
            // The eye's own box: not one of this function's zones.
            return None;
        }
        let cell = ((x / OVERLAY_EYE_LEFT) * OVERLAY_TOP_CELLS as f32) as usize;
        return OVERLAY_TOP_ZONES
            .get(cell.min(OVERLAY_TOP_CELLS - 1))
            .map(|&(zone, _)| zone);
    }
    if y >= 1.0 - OVERLAY_STRIP_HEIGHT {
        let cell = (x * OVERLAY_BOTTOM_CELLS as f32) as usize;
        return OVERLAY_BOTTOM_ZONES
            .get(cell.min(OVERLAY_BOTTOM_CELLS - 1))
            .map(|&(zone, _)| zone);
    }
    if (OVERLAY_RAIL_TOP..OVERLAY_RAIL_BOTTOM).contains(&y) {
        if x < OVERLAY_RAIL_WIDTH {
            return Some(PcOverlayZone::DpiSlider);
        }
        if x >= 1.0 - OVERLAY_RAIL_WIDTH {
            return Some(PcOverlayZone::ScrollSlider);
        }
    }
    None
}

/// One entry from `overlay_zone_rects`: which zone, its label, and its normalized
/// `(x0, y0, x1, y1)` bounding box.
pub type OverlayZoneRect = (PcOverlayZone, &'static str, (f32, f32, f32, f32));

/// Normalized (0..1) rectangles for every desktop-profile overlay zone, paired with a short
/// label, for the renderer in `app::ui` to draw. Kept in sync with `overlay_zone_at` by
/// construction: both derive from the same constants and the same two cell tables, so the drawn
/// boxes and the actual hit-test can never drift apart.
///
/// The eye is not included; it is drawn separately from [`OVERLAY_EYE_RECT`] because it stays
/// visible in both profiles and whether or not the rest is revealed.
pub fn overlay_zone_rects() -> Vec<OverlayZoneRect> {
    let mut rects = Vec::with_capacity(OVERLAY_TOP_CELLS + OVERLAY_BOTTOM_CELLS + 2);

    let top_cell = OVERLAY_EYE_LEFT / OVERLAY_TOP_CELLS as f32;
    for (index, &(zone, label)) in OVERLAY_TOP_ZONES.iter().enumerate() {
        let x0 = index as f32 * top_cell;
        rects.push((zone, label, (x0, 0.0, x0 + top_cell, OVERLAY_STRIP_HEIGHT)));
    }

    let bottom_cell = 1.0 / OVERLAY_BOTTOM_CELLS as f32;
    for (index, &(zone, label)) in OVERLAY_BOTTOM_ZONES.iter().enumerate() {
        let x0 = index as f32 * bottom_cell;
        rects.push((
            zone,
            label,
            (x0, 1.0 - OVERLAY_STRIP_HEIGHT, x0 + bottom_cell, 1.0),
        ));
    }

    rects.push((
        PcOverlayZone::DpiSlider,
        "DPI",
        (
            0.0,
            OVERLAY_RAIL_TOP,
            OVERLAY_RAIL_WIDTH,
            OVERLAY_RAIL_BOTTOM,
        ),
    ));
    rects.push((
        PcOverlayZone::ScrollSlider,
        "\u{2195}",
        (
            1.0 - OVERLAY_RAIL_WIDTH,
            OVERLAY_RAIL_TOP,
            1.0,
            OVERLAY_RAIL_BOTTOM,
        ),
    ));

    rects
}

/// One user-facing effect of an overlay touch gesture. `shell::run` turns these into
/// `AppCommand`s / `MouseEvent`s / key taps; kept separate from those types so the mapping logic
/// here stays pure and unit-testable without an `App` or a live peer connection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PcOverlayAction {
    /// Shows/hides every zone except the eye itself.
    ToggleReveal,
    /// Swaps between the game and desktop control profiles.
    ToggleProfile,
    /// A plain key tap: press-and-release paired with the finger, like a physical key.
    Key(crate::gfn::input_protocol::KeyStroke),
    /// A modified key tap, e.g. Alt+Tab or Ctrl+Alt+Del.
    Chord {
        ctrl: bool,
        alt: bool,
        win: bool,
        key: crate::gfn::input_protocol::KeyStroke,
    },
    OpenSettings,
    ToggleKeyboard,
    /// Sticky modifier, shared with the on-screen keyboard's shift/ctrl/alt state.
    ToggleShift,
    ToggleCtrl,
    ToggleAlt,
    /// Vertical slider drag, normalized screen units (positive = finger moved down).
    DpiDelta(f32),
    ScrollDelta(f32),
}

/// The one-shot action a zone fires on finger-down, or `None` for the slider rails, which only
/// act on motion. Split out from `PcOverlayTouch` so the whole table is unit-testable.
fn overlay_zone_action(zone: PcOverlayZone) -> Option<PcOverlayAction> {
    use crate::gfn::input_protocol as proto;

    // `key_for_char` is the same lookup the on-screen keyboard uses, so C/V/D here are
    // guaranteed to agree with what a typed letter sends.
    let letter = |ch: char| proto::key_for_char(ch).expect("ASCII letter has a keystroke");

    Some(match zone {
        PcOverlayZone::Eye => PcOverlayAction::ToggleReveal,
        PcOverlayZone::ModeToggle => PcOverlayAction::ToggleProfile,
        PcOverlayZone::Esc => PcOverlayAction::Key(proto::KEY_ESCAPE),
        PcOverlayZone::Tab => PcOverlayAction::Key(proto::KEY_TAB),
        PcOverlayZone::Win => PcOverlayAction::Key(proto::KEY_LEFT_WIN),
        PcOverlayZone::AltTab => PcOverlayAction::Chord {
            ctrl: false,
            alt: true,
            win: false,
            key: proto::KEY_TAB,
        },
        PcOverlayZone::Copy => PcOverlayAction::Chord {
            ctrl: true,
            alt: false,
            win: false,
            key: letter('c'),
        },
        PcOverlayZone::Paste => PcOverlayAction::Chord {
            ctrl: true,
            alt: false,
            win: false,
            key: letter('v'),
        },
        PcOverlayZone::Keyboard => PcOverlayAction::ToggleKeyboard,
        PcOverlayZone::OpenSettings => PcOverlayAction::OpenSettings,
        PcOverlayZone::Shift => PcOverlayAction::ToggleShift,
        PcOverlayZone::Ctrl => PcOverlayAction::ToggleCtrl,
        PcOverlayZone::Alt => PcOverlayAction::ToggleAlt,
        PcOverlayZone::Enter => PcOverlayAction::Key(proto::KEY_ENTER),
        PcOverlayZone::Backspace => PcOverlayAction::Key(proto::KEY_BACKSPACE),
        PcOverlayZone::CtrlAltDel => PcOverlayAction::Chord {
            ctrl: true,
            alt: true,
            win: false,
            key: proto::KEY_DELETE,
        },
        PcOverlayZone::DpiSlider | PcOverlayZone::ScrollSlider => return None,
    })
}

/// Front-screen half of the PC-touch overlay: the two key strips, the eye toggle and the two
/// slider rails. Tracks which zone each active finger landed in on finger-down, so a finger that
/// drifts out of its zone mid-drag (sliders in particular) keeps controlling the same thing
/// until it lifts.
#[derive(Default)]
pub struct PcOverlayTouch {
    // finger id -> (zone, last x, last y)
    fingers: Vec<(i64, PcOverlayZone, f32, f32)>,
}

impl PcOverlayTouch {
    /// `revealed` is the overlay's show/hide state: the profile switch under the eye is live
    /// whenever it is true, in either profile. `zones_live` is narrower - false unless the
    /// overlay is revealed *and* the profile is `Desktop` - so in game mode a thumb resting on
    /// the top strip still reaches the game.
    pub fn handle(
        &mut self,
        event: &Event,
        revealed: bool,
        zones_live: bool,
    ) -> Vec<PcOverlayAction> {
        let mut out = Vec::new();
        match *event {
            Event::FingerDown {
                touch_id,
                finger_id,
                x,
                y,
                ..
            } if touch_id == FRONT_TOUCH_DEVICE_ID => {
                let zone = if overlay_eye_at(x, y) {
                    PcOverlayZone::Eye
                } else if revealed && overlay_mode_at(x, y) {
                    PcOverlayZone::ModeToggle
                } else if zones_live {
                    match overlay_zone_at(x, y) {
                        Some(zone) => zone,
                        None => return out,
                    }
                } else {
                    return out;
                };
                self.fingers.push((finger_id, zone, x, y));
                out.extend(overlay_zone_action(zone));
            }
            Event::FingerMotion {
                touch_id,
                finger_id,
                x,
                y,
                ..
            } if touch_id == FRONT_TOUCH_DEVICE_ID => {
                if let Some(slot) = self.fingers.iter_mut().find(|(id, ..)| *id == finger_id) {
                    let (_, zone, _, last_y) = *slot;
                    let dy = y - last_y;
                    slot.2 = x;
                    slot.3 = y;
                    match zone {
                        PcOverlayZone::DpiSlider => out.push(PcOverlayAction::DpiDelta(dy)),
                        PcOverlayZone::ScrollSlider => out.push(PcOverlayAction::ScrollDelta(dy)),
                        _ => {}
                    }
                }
            }
            Event::FingerUp {
                touch_id,
                finger_id,
                ..
            } if touch_id == FRONT_TOUCH_DEVICE_ID => {
                self.fingers.retain(|(id, ..)| *id != finger_id);
            }
            _ => {}
        }
        out
    }

    /// True while at least one finger is inside an overlay zone, so `shell::run` can keep the
    /// touch away from the trackpad and the game for the whole gesture rather than just the
    /// frame it started on.
    pub fn is_active(&self) -> bool {
        !self.fingers.is_empty()
    }
}

/// Rear panel -> host mouse, active only in [`ControlProfile::Desktop`].
///
/// The NVST input protocol has no absolute-position mouse packet (see `INPUT_MOUSE_MOVE_REL`'s
/// doc comment in `gfn::input_protocol`) - only relative deltas - so, exactly like the front
/// screen's own `StreamTouchState` trackpad, this drives the cursor by the distance dragged
/// rather than by mapping touch position directly onto the host screen. `sensitivity_percent`
/// (100 = 1x) is the configurable "DPI": see `stream_prefs::OverlaySensitivity` and the
/// L-shoulder "sniper mode" halving in `shell::run`.
///
/// v0.5.0 added clicks. Earlier builds deliberately refused to click from the rear panel,
/// reasoning that the panel is out of sight so a stray tap-click would be hard to notice. In
/// practice the opposite was true: with no rear click there was no way to click at all without
/// covering the picture with a thumb, so the panel is now the primary mouse. The safeguard is
/// that a click only fires when the finger both lifts quickly and barely moved - anything
/// longer or further is treated as a cursor drag and clicks nothing.
///
/// [`ControlProfile::Desktop`]: crate::gfn::stream_prefs::ControlProfile::Desktop
#[derive(Default)]
pub struct RearOverlayMouse {
    // finger id -> (last x, last y, start x, start y, down_at, moved_too_far)
    fingers: Vec<RearFinger>,
}

struct RearFinger {
    id: i64,
    last: (f32, f32),
    start: (f32, f32),
    down_at: std::time::Instant,
    dragged: bool,
}

/// Longest press still counted as a tap rather than a drag.
const REAR_TAP_MAX_HOLD: std::time::Duration = std::time::Duration::from_millis(300);
/// Furthest a finger may wander (normalized panel units) and still count as a tap. The rear
/// panel reports a fairly noisy position, so this has to be loose enough that simply resting a
/// finger does not disqualify the tap.
const REAR_TAP_MAX_TRAVEL: f32 = 0.05;

/// Pure delta math for `RearOverlayMouse::map`, split out so it is unit-testable without
/// constructing an SDL touch event.
pub fn scale_rear_delta(
    dx: f32,
    dy: f32,
    stream_size: (f32, f32),
    sensitivity_percent: u16,
) -> (i16, i16) {
    let scale = f32::from(sensitivity_percent) / 100.0;
    let move_x = (dx * stream_size.0 * scale).round();
    let move_y = (dy * stream_size.1 * scale).round();
    (
        move_x.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
        move_y.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
    )
}

/// Which mouse button a rear-panel tap corresponds to, from where it started: left half of the
/// panel is the left button, right half the right button. Split by the *start* position rather
/// than the lift position so a tap that drifts a few millimetres cannot change buttons.
pub fn rear_tap_is_right_click(start_x: f32) -> bool {
    start_x >= 0.5
}

/// True when a finger's press qualifies as a tap (and therefore a click) rather than a drag.
pub fn rear_press_is_tap(held: std::time::Duration, travel: f32) -> bool {
    held <= REAR_TAP_MAX_HOLD && travel <= REAR_TAP_MAX_TRAVEL
}

impl RearOverlayMouse {
    /// Translates one SDL event into the host-mouse events it implies. A motion yields a
    /// `MoveBy`; a short, still press yields a paired press/release click on lift.
    pub fn map(
        &mut self,
        event: &Event,
        stream_size: (f32, f32),
        sensitivity_percent: u16,
    ) -> Vec<crate::gfn::input_protocol::MouseEvent> {
        use crate::gfn::input_protocol::MouseEvent;
        let mut out = Vec::new();
        match *event {
            Event::FingerDown {
                touch_id,
                finger_id,
                x,
                y,
                ..
            } if touch_id == REAR_TOUCH_DEVICE_ID => {
                self.fingers.retain(|f| f.id != finger_id);
                self.fingers.push(RearFinger {
                    id: finger_id,
                    last: (x, y),
                    start: (x, y),
                    down_at: std::time::Instant::now(),
                    dragged: false,
                });
            }
            Event::FingerMotion {
                touch_id,
                finger_id,
                x,
                y,
                ..
            } if touch_id == REAR_TOUCH_DEVICE_ID => {
                let Some(finger) = self.fingers.iter_mut().find(|f| f.id == finger_id) else {
                    return out;
                };
                let (prev_x, prev_y) = finger.last;
                finger.last = (x, y);
                if travel(finger.start, (x, y)) > REAR_TAP_MAX_TRAVEL {
                    finger.dragged = true;
                }
                let (dx, dy) =
                    scale_rear_delta(x - prev_x, y - prev_y, stream_size, sensitivity_percent);
                if dx != 0 || dy != 0 {
                    out.push(MouseEvent::MoveBy { dx, dy });
                }
            }
            Event::FingerUp {
                touch_id,
                finger_id,
                ..
            } if touch_id == REAR_TOUCH_DEVICE_ID => {
                let Some(pos) = self.fingers.iter().position(|f| f.id == finger_id) else {
                    return out;
                };
                let finger = self.fingers.remove(pos);
                let held = finger.down_at.elapsed();
                if !finger.dragged && rear_press_is_tap(held, travel(finger.start, finger.last)) {
                    let button = if rear_tap_is_right_click(finger.start.0) {
                        crate::gfn::input_protocol::MouseButton::Right
                    } else {
                        crate::gfn::input_protocol::MouseButton::Left
                    };
                    out.push(MouseEvent::Button {
                        button,
                        pressed: true,
                    });
                    out.push(MouseEvent::Button {
                        button,
                        pressed: false,
                    });
                }
            }
            _ => {}
        }
        out
    }
}

fn travel(from: (f32, f32), to: (f32, f32)) -> f32 {
    ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt()
}

/// A snapshot of the physical controls, in Vita terms, for [`DesktopPad`]. Kept as a plain data
/// struct rather than reading the `GameController` directly so the whole mapping is pure and
/// unit-testable without SDL.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DesktopPadState {
    /// -1.0..1.0 each, already normalized from the raw axes.
    pub left_stick: (f32, f32),
    pub right_stick: (f32, f32),
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
    /// Cross.
    pub cross: bool,
    /// Circle.
    pub circle: bool,
    /// Triangle.
    pub triangle: bool,
    /// Square.
    pub square: bool,
    pub l1: bool,
    pub r1: bool,
    pub select: bool,
    pub start: bool,
}

/// What one `DesktopPad` update wants done. `shell::run` turns these into wire packets and
/// `AppCommand`s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DesktopPadAction {
    Mouse(crate::gfn::input_protocol::MouseEvent),
    /// Paired press+release of a plain key.
    KeyTap(crate::gfn::input_protocol::KeyStroke),
    ToggleKeyboard,
}

/// Sticks below this fraction of full deflection are treated as centred. The Vita's sticks rest
/// noticeably off-centre when worn, so this is deliberately generous.
const PAD_STICK_DEADZONE: f32 = 0.22;
/// Cursor speed at full stick deflection, in host pixels per second at 100% sensitivity.
const PAD_CURSOR_PIXELS_PER_SEC: f32 = 620.0;
/// Wheel notches per second at full right-stick deflection.
const PAD_SCROLL_NOTCHES_PER_SEC: f32 = 9.0;
/// One Windows wheel notch.
const WHEEL_NOTCH: i16 = 120;
/// Hold a d-pad direction this long before it starts repeating.
const PAD_REPEAT_DELAY: std::time::Duration = std::time::Duration::from_millis(400);
/// ...then fire this often.
const PAD_REPEAT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(70);

/// Applies a radial deadzone and rescales what is left back over the full 0..1 range, so the
/// cursor still reaches full speed at the edge of the stick's travel.
fn apply_deadzone(value: f32) -> f32 {
    if value.abs() <= PAD_STICK_DEADZONE {
        return 0.0;
    }
    let sign = value.signum();
    ((value.abs() - PAD_STICK_DEADZONE) / (1.0 - PAD_STICK_DEADZONE)).min(1.0) * sign
}

/// Desktop-profile pad mapping: the sticks, d-pad and face buttons become a mouse and keyboard
/// instead of reaching the title.
///
/// This is what makes the desktop profile usable without a thumb permanently on the screen: the
/// rear panel is the fast pointer, the left stick is the precise one, and the right stick is the
/// scroll wheel. Fractional movement is accumulated between updates so slow stick deflections
/// still produce motion rather than rounding away to zero every frame.
#[derive(Default)]
pub struct DesktopPad {
    previous: DesktopPadState,
    cursor_remainder: (f32, f32),
    scroll_remainder: f32,
    /// Which direction is repeating, and when it next fires.
    repeat: Option<(crate::gfn::input_protocol::KeyStroke, std::time::Duration)>,
    /// How long the current d-pad direction has been held.
    held: std::time::Duration,
}

impl DesktopPad {
    /// Advances the mapping by `dt` and returns everything that should be sent. `sniper` is the
    /// L-shoulder "halve the sensitivity" modifier, applied on top of `sensitivity_percent`.
    pub fn update(
        &mut self,
        state: DesktopPadState,
        sensitivity_percent: u16,
        dt: std::time::Duration,
    ) -> Vec<DesktopPadAction> {
        use crate::gfn::input_protocol as proto;
        use proto::{MouseButton, MouseEvent};

        let mut out = Vec::new();
        let seconds = dt.as_secs_f32();
        let mut scale = f32::from(sensitivity_percent) / 100.0;
        if state.l1 {
            // Sniper mode: same halving the rear panel gets, so both pointers agree.
            scale *= 0.5;
        }

        // --- left stick: fine cursor ---
        let (sx, sy) = (
            apply_deadzone(state.left_stick.0),
            apply_deadzone(state.left_stick.1),
        );
        if sx != 0.0 || sy != 0.0 {
            let step = PAD_CURSOR_PIXELS_PER_SEC * scale * seconds;
            self.cursor_remainder.0 += sx * step;
            self.cursor_remainder.1 += sy * step;
        } else {
            self.cursor_remainder = (0.0, 0.0);
        }
        let dx = self.cursor_remainder.0.trunc();
        let dy = self.cursor_remainder.1.trunc();
        if dx != 0.0 || dy != 0.0 {
            self.cursor_remainder.0 -= dx;
            self.cursor_remainder.1 -= dy;
            out.push(DesktopPadAction::Mouse(MouseEvent::MoveBy {
                dx: dx.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
                dy: dy.clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            }));
        }

        // --- right stick: scroll wheel ---
        let scroll = apply_deadzone(state.right_stick.1);
        if scroll != 0.0 {
            self.scroll_remainder += scroll * PAD_SCROLL_NOTCHES_PER_SEC * seconds;
        } else {
            self.scroll_remainder = 0.0;
        }
        let notches = self.scroll_remainder.trunc();
        if notches != 0.0 {
            self.scroll_remainder -= notches;
            // Stick down (positive y) should scroll the page down, which is a negative wheel
            // delta under the Windows convention MouseEvent::WheelBy documents.
            let delta =
                (-notches * f32::from(WHEEL_NOTCH)).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            out.push(DesktopPadAction::Mouse(MouseEvent::WheelBy { delta }));
        }

        // --- d-pad: arrow keys, with hold-to-repeat ---
        let direction = if state.dpad_up {
            Some(proto::KEY_UP)
        } else if state.dpad_down {
            Some(proto::KEY_DOWN)
        } else if state.dpad_left {
            Some(proto::KEY_LEFT)
        } else if state.dpad_right {
            Some(proto::KEY_RIGHT)
        } else {
            None
        };
        match direction {
            Some(key) if self.repeat.map(|(k, _)| k) == Some(key) => {
                self.held += dt;
                if let Some((_, next_at)) = self.repeat
                    && self.held >= next_at
                {
                    out.push(DesktopPadAction::KeyTap(key));
                    self.repeat = Some((key, self.held + PAD_REPEAT_INTERVAL));
                }
            }
            Some(key) => {
                out.push(DesktopPadAction::KeyTap(key));
                self.held = std::time::Duration::ZERO;
                self.repeat = Some((key, PAD_REPEAT_DELAY));
            }
            None => {
                self.repeat = None;
                self.held = std::time::Duration::ZERO;
            }
        }

        // --- face buttons ---
        // Cross/Circle are held, not tapped, so click-and-drag works.
        for (now, before, button) in [
            (state.cross, self.previous.cross, MouseButton::Left),
            (state.circle, self.previous.circle, MouseButton::Right),
        ] {
            if now != before {
                out.push(DesktopPadAction::Mouse(MouseEvent::Button {
                    button,
                    pressed: now,
                }));
            }
        }
        let pressed = |now: bool, before: bool| now && !before;
        if pressed(state.triangle, self.previous.triangle) {
            out.push(DesktopPadAction::KeyTap(proto::KEY_ENTER));
        }
        if pressed(state.square, self.previous.square) {
            out.push(DesktopPadAction::KeyTap(proto::KEY_BACKSPACE));
        }
        if pressed(state.r1, self.previous.r1) {
            // Double click: two complete press/release pairs back to back.
            for _ in 0..2 {
                out.push(DesktopPadAction::Mouse(MouseEvent::Button {
                    button: MouseButton::Left,
                    pressed: true,
                }));
                out.push(DesktopPadAction::Mouse(MouseEvent::Button {
                    button: MouseButton::Left,
                    pressed: false,
                }));
            }
        }
        if pressed(state.select, self.previous.select) {
            out.push(DesktopPadAction::ToggleKeyboard);
        }
        if pressed(state.start, self.previous.start) {
            out.push(DesktopPadAction::KeyTap(proto::KEY_LEFT_WIN));
        }

        self.previous = state;
        out
    }

    /// Releases anything still held, for when the profile switches back to `Game` mid-press.
    pub fn release_all(&mut self) -> Vec<DesktopPadAction> {
        use crate::gfn::input_protocol::{MouseButton, MouseEvent};
        let mut out = Vec::new();
        for (held, button) in [
            (self.previous.cross, MouseButton::Left),
            (self.previous.circle, MouseButton::Right),
        ] {
            if held {
                out.push(DesktopPadAction::Mouse(MouseEvent::Button {
                    button,
                    pressed: false,
                }));
            }
        }
        *self = Self::default();
        out
    }
}

/// The rear touch panel stands in, split down the middle: left half is L2, right half is R2.
///
/// It was briefly split into quadrants to fit L3/R3 in as well, but the panel is only ~2.5 cm tall
/// and has no landmark to tell top from bottom by feel, so a quadrant was ~1.2 cm of guesswork and
/// reaching for L2 could give L3. The stick clicks moved to the front screen instead.
#[derive(Default)]
pub struct RearTouchTriggers {
    // finger id -> (x, y) normalized 0..1
    fingers: Vec<(i64, f32, f32)>,
    intensity: u8,
}

impl RearTouchTriggers {
    pub fn handle(&mut self, event: &Event) {
        match *event {
            Event::FingerDown {
                touch_id,
                finger_id,
                x,
                y,
                ..
            }
            | Event::FingerMotion {
                touch_id,
                finger_id,
                x,
                y,
                ..
            } if touch_id == REAR_TOUCH_DEVICE_ID => {
                match self.fingers.iter_mut().find(|(id, _, _)| *id == finger_id) {
                    Some(slot) => {
                        slot.1 = x;
                        slot.2 = y;
                    }
                    None => self.fingers.push((finger_id, x, y)),
                }
            }
            Event::FingerUp {
                touch_id,
                finger_id,
                ..
            } if touch_id == REAR_TOUCH_DEVICE_ID => {
                self.fingers.retain(|(id, _, _)| *id != finger_id);
            }
            _ => {}
        }
    }

    fn quadrant_held(&self, left_side: bool, top_side: bool) -> bool {
        self.fingers.iter().any(|(_, x, y)| {
            let matches_x = if left_side { *x < 0.5 } else { *x >= 0.5 };
            let matches_y = if top_side { *y < 0.5 } else { *y >= 0.5 };
            matches_x && matches_y
        })
    }

    fn half_held(&self, left_side: bool) -> bool {
        self.fingers
            .iter()
            .any(|(_, x, _)| if left_side { *x < 0.5 } else { *x >= 0.5 })
    }

    /// Picks up a changed intensity setting; call when a session starts.
    pub fn reload_intensity(&mut self) {
        self.intensity = crate::gfn::stream_prefs::trigger_intensity().value();
    }

    fn pressure(&self) -> u8 {
        if self.intensity == 0 {
            crate::gfn::stream_prefs::TriggerIntensity::default().value()
        } else {
            self.intensity
        }
    }

    fn left_trigger(&self) -> u8 {
        let is_quadrant = crate::gfn::stream_prefs::rear_touch_mode()
            == crate::gfn::stream_prefs::RearTouchMode::Quadrant;
        let held = if is_quadrant {
            self.quadrant_held(true, true)
        } else {
            self.half_held(true)
        };
        if held { self.pressure() } else { 0 }
    }

    fn right_trigger(&self) -> u8 {
        let is_quadrant = crate::gfn::stream_prefs::rear_touch_mode()
            == crate::gfn::stream_prefs::RearTouchMode::Quadrant;
        let held = if is_quadrant {
            self.quadrant_held(false, true)
        } else {
            self.half_held(false)
        };
        if held { self.pressure() } else { 0 }
    }

    pub fn left_stick_click(&self) -> bool {
        if crate::gfn::stream_prefs::rear_touch_mode()
            == crate::gfn::stream_prefs::RearTouchMode::Quadrant
        {
            self.quadrant_held(true, false)
        } else {
            false
        }
    }

    pub fn right_stick_click(&self) -> bool {
        if crate::gfn::stream_prefs::rear_touch_mode()
            == crate::gfn::stream_prefs::RearTouchMode::Quadrant
        {
            self.quadrant_held(false, false)
        } else {
            false
        }
    }
}

/// Reads the physical controls into a [`DesktopPadState`]. The Vita's SDL mapping (see
/// `register_vita_controller_mapping`) puts Cross on `A`, Circle on `B`, Square on `X` and
/// Triangle on `Y`, which is what the field names here refer to.
pub fn desktop_pad_state(controller: &GameController) -> DesktopPadState {
    DesktopPadState {
        left_stick: (
            axis_to_f32(controller.axis(Axis::LeftX)),
            axis_to_f32(controller.axis(Axis::LeftY)),
        ),
        right_stick: (
            axis_to_f32(controller.axis(Axis::RightX)),
            axis_to_f32(controller.axis(Axis::RightY)),
        ),
        dpad_up: controller.button(Button::DPadUp),
        dpad_down: controller.button(Button::DPadDown),
        dpad_left: controller.button(Button::DPadLeft),
        dpad_right: controller.button(Button::DPadRight),
        cross: controller.button(Button::A),
        circle: controller.button(Button::B),
        triangle: controller.button(Button::Y),
        square: controller.button(Button::X),
        l1: controller.button(Button::LeftShoulder),
        r1: controller.button(Button::RightShoulder),
        select: controller.button(Button::Back),
        start: controller.button(Button::Start),
    }
}

/// Full controller snapshot for the streaming session, in XInput conventions (the format the NVST
/// input channel speaks - see `gfn::input_protocol`).
pub fn gamepad_snapshot(
    controller: &GameController,
    rear_touch: &RearTouchTriggers,
    stick_zones: &FrontStickZones,
) -> crate::gfn::input_protocol::GamepadInput {
    const DPAD_UP: u16 = 0x0001;
    const DPAD_DOWN: u16 = 0x0002;
    const DPAD_LEFT: u16 = 0x0004;
    const DPAD_RIGHT: u16 = 0x0008;
    const START: u16 = 0x0010;
    const BACK: u16 = 0x0020;
    const LEFT_THUMB: u16 = 0x0040;
    const RIGHT_THUMB: u16 = 0x0080;
    const LEFT_SHOULDER: u16 = 0x0100;
    const RIGHT_SHOULDER: u16 = 0x0200;
    const A: u16 = 0x1000;
    const B: u16 = 0x2000;
    const X: u16 = 0x4000;
    const Y: u16 = 0x8000;

    let mut buttons = 0u16;
    let mut set = |button: Button, mask: u16| {
        if controller.button(button) {
            buttons |= mask;
        }
    };
    set(Button::DPadUp, DPAD_UP);
    set(Button::DPadDown, DPAD_DOWN);
    set(Button::DPadLeft, DPAD_LEFT);
    set(Button::DPadRight, DPAD_RIGHT);
    set(Button::Start, START);
    set(Button::Back, BACK);
    set(Button::LeftStick, LEFT_THUMB);
    set(Button::RightStick, RIGHT_THUMB);
    set(Button::A, A);
    set(Button::B, B);
    set(Button::X, X);
    set(Button::Y, Y);

    let axis = |axis: Axis| controller.axis(axis);
    let trigger = |value: i16| (value.max(0) / 129).min(255) as u8;

    let swap_triggers = crate::gfn::stream_prefs::trigger_swap_enabled();
    let phys_l1 = controller.button(Button::LeftShoulder);
    let phys_r1 = controller.button(Button::RightShoulder);
    let phys_l2 = trigger(axis(Axis::TriggerLeft)).max(rear_touch.left_trigger());
    let phys_r2 = trigger(axis(Axis::TriggerRight)).max(rear_touch.right_trigger());
    let (left_shoulder_down, right_shoulder_down, left_trigger_val, right_trigger_val) =
        if swap_triggers {
            (
                phys_l2 > 0,
                phys_r2 > 0,
                if phys_l1 { 255 } else { 0 },
                if phys_r1 { 255 } else { 0 },
            )
        } else {
            (phys_l1, phys_r1, phys_l2, phys_r2)
        };
    if left_shoulder_down {
        buttons |= LEFT_SHOULDER;
    }
    if right_shoulder_down {
        buttons |= RIGHT_SHOULDER;
    }

    if controller.button(Button::LeftStick)
        || stick_zones.left_stick_click()
        || rear_touch.left_stick_click()
    {
        buttons |= LEFT_THUMB;
    }
    if controller.button(Button::RightStick)
        || stick_zones.right_stick_click()
        || rear_touch.right_stick_click()
    {
        buttons |= RIGHT_THUMB;
    }

    crate::gfn::input_protocol::GamepadInput {
        controller_id: 0,
        buttons,
        // Whichever source is pressing harder wins, so an attached DualShock on a Vita TV still
        // works while the rear panel covers the handheld.
        left_trigger: left_trigger_val,
        right_trigger: right_trigger_val,
        left_stick_x: axis(Axis::LeftX),
        left_stick_y: axis(Axis::LeftY).saturating_neg(),
        right_stick_x: axis(Axis::RightX),
        right_stick_y: axis(Axis::RightY).saturating_neg(),
        timestamp_us: 0,
    }
}

#[cfg(test)]
mod stick_zone_tests {
    use super::*;

    #[test]
    fn the_bottom_corners_map_to_their_own_side() {
        // Bottom-left is L3 and nothing else; bottom-right is R3.
        assert!(in_stick_zone(0.05, 0.95, true));
        assert!(!in_stick_zone(0.05, 0.95, false));
        assert!(in_stick_zone(0.95, 0.95, false));
        assert!(!in_stick_zone(0.95, 0.95, true));
    }

    /// The middle of the screen is where the game is - it has to stay mouse.
    #[test]
    fn the_centre_of_the_screen_is_not_a_zone() {
        assert!(!is_in_stick_zone(0.5, 0.5));
        assert!(!is_in_stick_zone(0.5, 0.95), "bottom centre is still mouse");
        assert!(!is_in_stick_zone(0.05, 0.4), "left edge but too high");
    }

    #[test]
    fn the_zones_only_cover_the_bottom_third() {
        assert!(!is_in_stick_zone(0.05, STICK_ZONE_TOP - 0.01));
        assert!(is_in_stick_zone(0.05, STICK_ZONE_TOP + 0.01));
    }
}

#[cfg(test)]
mod pc_overlay_tests {
    use super::*;
    use crate::gfn::input_protocol as proto;
    use crate::gfn::input_protocol::{MouseButton, MouseEvent};
    use std::time::Duration;

    fn finger_down(id: i64, touch_id: i64, x: f32, y: f32) -> Event {
        Event::FingerDown {
            timestamp: 0,
            touch_id,
            finger_id: id,
            x,
            y,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
        }
    }

    fn finger_motion(id: i64, touch_id: i64, x: f32, y: f32) -> Event {
        Event::FingerMotion {
            timestamp: 0,
            touch_id,
            finger_id: id,
            x,
            y,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
        }
    }

    fn finger_up(id: i64, touch_id: i64, x: f32, y: f32) -> Event {
        Event::FingerUp {
            timestamp: 0,
            touch_id,
            finger_id: id,
            x,
            y,
            dx: 0.0,
            dy: 0.0,
            pressure: 0.0,
        }
    }

    /// The whole point of the v0.5.0 layout: the middle of the 960x544 panel - where the game
    /// actually is - must stay clear, so the overlay can never swallow the picture.
    #[test]
    fn the_centre_of_the_screen_is_still_mouse() {
        assert_eq!(overlay_zone_at(0.5, 0.5), None);
        assert_eq!(overlay_zone_at(0.5, 0.3), None);
        assert_eq!(overlay_zone_at(0.5, 0.7), None);
        assert_eq!(overlay_zone_at(0.2, 0.5), None, "inboard of the DPI rail");
        assert_eq!(
            overlay_zone_at(0.8, 0.5),
            None,
            "inboard of the scroll rail"
        );
        assert!(!overlay_eye_at(0.5, 0.5));
    }

    #[test]
    fn the_eye_owns_the_top_right_corner_and_nothing_else_does() {
        assert!(overlay_eye_at(0.99, 0.01));
        assert!(overlay_eye_at(0.9, 0.1));
        assert_eq!(
            overlay_zone_at(0.99, 0.01),
            None,
            "the eye is not one of overlay_zone_at's zones"
        );
        assert!(
            !overlay_eye_at(0.85, 0.05),
            "left of the eye is the top strip"
        );
        assert!(
            !overlay_eye_at(0.99, 0.5),
            "below the strip is the scroll rail"
        );
    }

    #[test]
    fn the_top_strip_runs_esc_through_settings_left_to_right() {
        let cell = OVERLAY_EYE_LEFT / OVERLAY_TOP_CELLS as f32;
        let expected = [
            PcOverlayZone::Esc,
            PcOverlayZone::Tab,
            PcOverlayZone::Win,
            PcOverlayZone::AltTab,
            PcOverlayZone::Copy,
            PcOverlayZone::Paste,
            PcOverlayZone::Keyboard,
            PcOverlayZone::OpenSettings,
        ];
        for (index, zone) in expected.into_iter().enumerate() {
            let centre_x = (index as f32 + 0.5) * cell;
            assert_eq!(
                overlay_zone_at(centre_x, OVERLAY_STRIP_HEIGHT / 2.0),
                Some(zone),
                "top cell {index}"
            );
        }
    }

    #[test]
    fn the_bottom_strip_runs_shift_through_ctrl_alt_del_left_to_right() {
        let cell = 1.0 / OVERLAY_BOTTOM_CELLS as f32;
        let expected = [
            PcOverlayZone::Shift,
            PcOverlayZone::Ctrl,
            PcOverlayZone::Alt,
            PcOverlayZone::Enter,
            PcOverlayZone::Backspace,
            PcOverlayZone::CtrlAltDel,
        ];
        for (index, zone) in expected.into_iter().enumerate() {
            let centre_x = (index as f32 + 0.5) * cell;
            assert_eq!(
                overlay_zone_at(centre_x, 1.0 - OVERLAY_STRIP_HEIGHT / 2.0),
                Some(zone),
                "bottom cell {index}"
            );
        }
    }

    #[test]
    fn the_side_rails_are_the_dpi_and_scroll_sliders() {
        assert_eq!(overlay_zone_at(0.01, 0.5), Some(PcOverlayZone::DpiSlider));
        assert_eq!(
            overlay_zone_at(0.99, 0.5),
            Some(PcOverlayZone::ScrollSlider)
        );
        assert_eq!(
            overlay_zone_at(0.01, OVERLAY_RAIL_TOP - 0.01),
            None,
            "above the rail is clear picture, not a slider"
        );
    }

    /// Every drawn rect must hit-test back to the zone it is labelled with, or the overlay would
    /// lie about what a button does.
    #[test]
    fn every_drawn_rect_hit_tests_back_to_its_own_zone() {
        for (zone, label, (x0, y0, x1, y1)) in overlay_zone_rects() {
            let centre = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            assert_eq!(
                overlay_zone_at(centre.0, centre.1),
                Some(zone),
                "{label} at {centre:?}"
            );
        }
    }

    #[test]
    fn no_two_drawn_rects_overlap() {
        let rects = overlay_zone_rects();
        for (i, (_, a_label, a)) in rects.iter().enumerate() {
            for (_, b_label, b) in rects.iter().skip(i + 1) {
                let disjoint = a.2 <= b.0 || b.2 <= a.0 || a.3 <= b.1 || b.3 <= a.1;
                assert!(disjoint, "{a_label} overlaps {b_label}");
            }
            let eye = OVERLAY_EYE_RECT;
            let disjoint = a.2 <= eye.0 || eye.2 <= a.0 || a.3 <= eye.1 || eye.3 <= a.1;
            assert!(disjoint, "{a_label} overlaps the eye toggle");
        }
    }

    /// The profile switch is the only touch route out of the game profile, where every key strip
    /// is dead, so it must fire with `zones_live` false as long as the overlay is revealed.
    #[test]
    fn the_mode_toggle_works_in_the_game_profile_but_only_while_revealed() {
        let (x0, y0, x1, y1) = OVERLAY_MODE_RECT;
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);

        let mut revealed = PcOverlayTouch::default();
        assert_eq!(
            revealed.handle(&finger_down(1, FRONT_TOUCH_DEVICE_ID, cx, cy), true, false),
            vec![PcOverlayAction::ToggleProfile]
        );

        let mut collapsed = PcOverlayTouch::default();
        assert!(
            collapsed
                .handle(&finger_down(2, FRONT_TOUCH_DEVICE_ID, cx, cy), false, false)
                .is_empty(),
            "a collapsed overlay must leave that spot to the game"
        );
    }

    /// The switch sits in the gap between the top strip and the slider rails, so it must not
    /// collide with the eye above it or with any drawn zone.
    #[test]
    fn the_mode_toggle_box_is_disjoint_from_the_eye_and_every_zone() {
        let m = OVERLAY_MODE_RECT;
        let eye = OVERLAY_EYE_RECT;
        assert!(m.3 <= eye.1 || eye.3 <= m.1, "mode toggle overlaps the eye");
        for (_, label, r) in overlay_zone_rects() {
            let disjoint = r.2 <= m.0 || m.2 <= r.0 || r.3 <= m.1 || m.3 <= r.1;
            assert!(disjoint, "{label} overlaps the mode toggle");
        }
        let (cx, cy) = ((m.0 + m.2) / 2.0, (m.1 + m.3) / 2.0);
        assert_eq!(overlay_zone_at(cx, cy), None);
        assert!(!overlay_eye_at(cx, cy));
        assert!(overlay_mode_at(cx, cy));
    }

    #[test]
    fn tapping_the_eye_toggles_the_reveal_even_when_the_zones_are_hidden() {
        let mut overlay = PcOverlayTouch::default();
        let actions = overlay.handle(
            &finger_down(1, FRONT_TOUCH_DEVICE_ID, 0.95, 0.05),
            false,
            false,
        );
        assert_eq!(actions, vec![PcOverlayAction::ToggleReveal]);
    }

    /// While collapsed (or in the game profile) a thumb on the top strip must fall through to the
    /// title rather than firing ESC.
    #[test]
    fn hidden_zones_do_not_fire() {
        let mut overlay = PcOverlayTouch::default();
        let actions = overlay.handle(
            &finger_down(1, FRONT_TOUCH_DEVICE_ID, 0.02, 0.05),
            false,
            false,
        );
        assert!(actions.is_empty());
        assert!(!overlay.is_active());
    }

    #[test]
    fn tapping_esc_emits_the_escape_keystroke() {
        let mut overlay = PcOverlayTouch::default();
        let actions = overlay.handle(
            &finger_down(1, FRONT_TOUCH_DEVICE_ID, 0.02, 0.05),
            true,
            true,
        );
        assert_eq!(actions, vec![PcOverlayAction::Key(proto::KEY_ESCAPE)]);
        assert!(
            overlay.is_active(),
            "the gesture owns the touch until it lifts"
        );
    }

    #[test]
    fn alt_tab_and_ctrl_alt_del_emit_chords_with_the_right_modifiers() {
        assert_eq!(
            overlay_zone_action(PcOverlayZone::AltTab),
            Some(PcOverlayAction::Chord {
                ctrl: false,
                alt: true,
                win: false,
                key: proto::KEY_TAB,
            })
        );
        assert_eq!(
            overlay_zone_action(PcOverlayZone::CtrlAltDel),
            Some(PcOverlayAction::Chord {
                ctrl: true,
                alt: true,
                win: false,
                key: proto::KEY_DELETE,
            })
        );
    }

    #[test]
    fn the_slider_rails_do_nothing_on_touch_down_and_only_act_on_drag() {
        assert_eq!(overlay_zone_action(PcOverlayZone::DpiSlider), None);
        assert_eq!(overlay_zone_action(PcOverlayZone::ScrollSlider), None);

        let mut overlay = PcOverlayTouch::default();
        assert!(
            overlay
                .handle(
                    &finger_down(3, FRONT_TOUCH_DEVICE_ID, 0.01, 0.4),
                    true,
                    true
                )
                .is_empty()
        );
        let motion = overlay.handle(
            &finger_motion(3, FRONT_TOUCH_DEVICE_ID, 0.01, 0.5),
            true,
            true,
        );
        assert_eq!(motion.len(), 1);
        match motion[0] {
            PcOverlayAction::DpiDelta(dy) => assert!((dy - 0.1).abs() < 1e-5),
            other => panic!("expected a DPI delta, got {other:?}"),
        }
    }

    /// A finger that started on a rail keeps driving that rail even after it wanders off the
    /// rail's box, so a slider drag is not cut short by a slightly diagonal thumb.
    #[test]
    fn a_slider_drag_keeps_its_zone_after_wandering_off_it() {
        let mut overlay = PcOverlayTouch::default();
        overlay.handle(
            &finger_down(4, FRONT_TOUCH_DEVICE_ID, 0.99, 0.4),
            true,
            true,
        );
        let motion = overlay.handle(
            &finger_motion(4, FRONT_TOUCH_DEVICE_ID, 0.60, 0.5),
            true,
            true,
        );
        assert!(matches!(motion[0], PcOverlayAction::ScrollDelta(_)));
    }

    #[test]
    fn rear_delta_scales_with_sensitivity_percent() {
        let stream_size = (960.0, 544.0);
        let (dx_100, dy_100) = scale_rear_delta(0.1, 0.0, stream_size, 100);
        let (dx_50, _) = scale_rear_delta(0.1, 0.0, stream_size, 50);
        let (dx_200, _) = scale_rear_delta(0.1, 0.0, stream_size, 200);
        assert_eq!(dx_100, 96);
        assert_eq!(dy_100, 0);
        assert_eq!(
            dx_50, 48,
            "half sensitivity halves the delta (the L-shoulder sniper mode)"
        );
        assert_eq!(dx_200, 192, "double sensitivity doubles the delta");
    }

    #[test]
    fn a_rear_tap_clicks_by_which_half_of_the_panel_it_started_on() {
        assert!(!rear_tap_is_right_click(0.1));
        assert!(!rear_tap_is_right_click(0.49));
        assert!(rear_tap_is_right_click(0.5));
        assert!(rear_tap_is_right_click(0.9));
    }

    #[test]
    fn only_a_short_and_still_press_counts_as_a_tap() {
        assert!(rear_press_is_tap(Duration::from_millis(80), 0.01));
        assert!(
            !rear_press_is_tap(Duration::from_millis(900), 0.01),
            "a long hold is a drag, not a click"
        );
        assert!(
            !rear_press_is_tap(Duration::from_millis(80), 0.4),
            "a press that travelled is a drag, not a click"
        );
    }

    #[test]
    fn a_quick_rear_tap_emits_a_paired_press_and_release() {
        let mut rear = RearOverlayMouse::default();
        assert!(
            rear.map(
                &finger_down(1, REAR_TOUCH_DEVICE_ID, 0.8, 0.5),
                (960.0, 544.0),
                100
            )
            .is_empty(),
            "nothing happens until the finger lifts"
        );
        let up = rear.map(
            &finger_up(1, REAR_TOUCH_DEVICE_ID, 0.8, 0.5),
            (960.0, 544.0),
            100,
        );
        assert_eq!(
            up,
            vec![
                MouseEvent::Button {
                    button: MouseButton::Right,
                    pressed: true
                },
                MouseEvent::Button {
                    button: MouseButton::Right,
                    pressed: false
                },
            ]
        );
    }

    #[test]
    fn a_rear_drag_moves_the_cursor_and_does_not_click() {
        let stream = (960.0, 544.0);
        let mut rear = RearOverlayMouse::default();
        rear.map(&finger_down(2, REAR_TOUCH_DEVICE_ID, 0.2, 0.5), stream, 100);
        let motion = rear.map(
            &finger_motion(2, REAR_TOUCH_DEVICE_ID, 0.4, 0.5),
            stream,
            100,
        );
        assert_eq!(motion, vec![MouseEvent::MoveBy { dx: 192, dy: 0 }]);
        let up = rear.map(&finger_up(2, REAR_TOUCH_DEVICE_ID, 0.4, 0.5), stream, 100);
        assert!(up.is_empty(), "a drag must never click");
    }

    /// The front panel and the rear panel must not answer each other's events.
    #[test]
    fn the_rear_mouse_ignores_front_panel_touches() {
        let mut rear = RearOverlayMouse::default();
        let out = rear.map(
            &finger_down(1, FRONT_TOUCH_DEVICE_ID, 0.5, 0.5),
            (960.0, 544.0),
            100,
        );
        assert!(out.is_empty());

        let mut overlay = PcOverlayTouch::default();
        assert!(
            overlay
                .handle(
                    &finger_down(1, REAR_TOUCH_DEVICE_ID, 0.02, 0.05),
                    true,
                    true
                )
                .is_empty()
        );
    }

    fn tick() -> Duration {
        Duration::from_millis(16)
    }

    #[test]
    fn the_left_stick_moves_the_cursor_and_the_deadzone_holds_it_still() {
        let mut pad = DesktopPad::default();
        let mut state = DesktopPadState::default();

        state.left_stick = (0.1, 0.0);
        let idle = pad.update(state, 100, tick());
        assert!(
            idle.is_empty(),
            "inside the deadzone the cursor must not drift"
        );

        state.left_stick = (1.0, 0.0);
        let moved = pad.update(state, 100, tick());
        let dx = moved
            .iter()
            .find_map(|action| match action {
                DesktopPadAction::Mouse(MouseEvent::MoveBy { dx, .. }) => Some(*dx),
                _ => None,
            })
            .expect("full deflection should move the cursor");
        assert!(dx > 0, "right on the stick moves the cursor right");
    }

    #[test]
    fn sniper_mode_halves_the_stick_cursor_speed() {
        let state = DesktopPadState {
            left_stick: (1.0, 0.0),
            ..Default::default()
        };
        let sniper = DesktopPadState { l1: true, ..state };

        let travel = |mut pad: DesktopPad, state: DesktopPadState| -> i32 {
            // Several ticks, so the sub-pixel remainder does not dominate the comparison.
            (0..10)
                .flat_map(|_| pad.update(state, 100, tick()))
                .filter_map(|action| match action {
                    DesktopPadAction::Mouse(MouseEvent::MoveBy { dx, .. }) => Some(i32::from(dx)),
                    _ => None,
                })
                .sum()
        };

        let normal = travel(DesktopPad::default(), state);
        let halved = travel(DesktopPad::default(), sniper);
        assert!(normal > 0 && halved > 0);
        assert!(
            (normal - halved * 2).abs() <= 2,
            "L-shoulder should halve the speed: {normal} vs {halved}"
        );
    }

    #[test]
    fn the_right_stick_scrolls_in_whole_notches() {
        let mut pad = DesktopPad::default();
        let state = DesktopPadState {
            right_stick: (0.0, 1.0),
            ..Default::default()
        };
        let deltas: Vec<i16> = (0..20)
            .flat_map(|_| pad.update(state, 100, tick()))
            .filter_map(|action| match action {
                DesktopPadAction::Mouse(MouseEvent::WheelBy { delta }) => Some(delta),
                _ => None,
            })
            .collect();
        assert!(!deltas.is_empty(), "a held stick should scroll");
        for delta in deltas {
            assert_eq!(
                delta, -WHEEL_NOTCH,
                "stick down scrolls the page down, i.e. a negative wheel delta"
            );
        }
    }

    #[test]
    fn the_dpad_taps_an_arrow_once_then_repeats_only_after_the_delay() {
        let mut pad = DesktopPad::default();
        let state = DesktopPadState {
            dpad_down: true,
            ..Default::default()
        };

        let first = pad.update(state, 100, tick());
        assert_eq!(first, vec![DesktopPadAction::KeyTap(proto::KEY_DOWN)]);

        // Still inside the repeat delay: nothing more.
        let quiet: Vec<_> = (0..5)
            .flat_map(|_| pad.update(state, 100, tick()))
            .collect();
        assert!(
            quiet.is_empty(),
            "held d-pad must not machine-gun immediately"
        );

        // Past the delay it starts repeating.
        let later: Vec<_> = (0..40)
            .flat_map(|_| pad.update(state, 100, tick()))
            .collect();
        assert!(
            later.contains(&DesktopPadAction::KeyTap(proto::KEY_DOWN)),
            "a long hold should repeat"
        );
    }

    #[test]
    fn the_face_buttons_click_hold_and_type() {
        let mut pad = DesktopPad::default();
        let mut state = DesktopPadState::default();

        state.cross = true;
        assert_eq!(
            pad.update(state, 100, tick()),
            vec![DesktopPadAction::Mouse(MouseEvent::Button {
                button: MouseButton::Left,
                pressed: true
            })]
        );
        // Held, not re-fired: that is what makes click-and-drag work.
        assert!(pad.update(state, 100, tick()).is_empty());
        state.cross = false;
        assert_eq!(
            pad.update(state, 100, tick()),
            vec![DesktopPadAction::Mouse(MouseEvent::Button {
                button: MouseButton::Left,
                pressed: false
            })]
        );

        state.circle = true;
        assert_eq!(
            pad.update(state, 100, tick()),
            vec![DesktopPadAction::Mouse(MouseEvent::Button {
                button: MouseButton::Right,
                pressed: true
            })]
        );
        state.circle = false;
        pad.update(state, 100, tick());

        state.triangle = true;
        assert_eq!(
            pad.update(state, 100, tick()),
            vec![DesktopPadAction::KeyTap(proto::KEY_ENTER)]
        );
        state.triangle = false;
        state.square = true;
        assert_eq!(
            pad.update(state, 100, tick()),
            vec![DesktopPadAction::KeyTap(proto::KEY_BACKSPACE)]
        );
    }

    #[test]
    fn select_toggles_the_keyboard_and_start_taps_the_windows_key() {
        let mut pad = DesktopPad::default();
        let mut state = DesktopPadState::default();

        state.select = true;
        assert_eq!(
            pad.update(state, 100, tick()),
            vec![DesktopPadAction::ToggleKeyboard]
        );
        assert!(
            pad.update(state, 100, tick()).is_empty(),
            "holding SELECT must not toggle repeatedly"
        );
        state.select = false;
        pad.update(state, 100, tick());

        state.start = true;
        assert_eq!(
            pad.update(state, 100, tick()),
            vec![DesktopPadAction::KeyTap(proto::KEY_LEFT_WIN)]
        );
    }

    #[test]
    fn r1_sends_a_complete_double_click() {
        let mut pad = DesktopPad::default();
        let state = DesktopPadState {
            r1: true,
            ..Default::default()
        };
        let out = pad.update(state, 100, tick());
        let presses = out
            .iter()
            .filter(|action| {
                matches!(
                    action,
                    DesktopPadAction::Mouse(MouseEvent::Button {
                        button: MouseButton::Left,
                        pressed: true
                    })
                )
            })
            .count();
        let releases = out
            .iter()
            .filter(|action| {
                matches!(
                    action,
                    DesktopPadAction::Mouse(MouseEvent::Button {
                        button: MouseButton::Left,
                        pressed: false
                    })
                )
            })
            .count();
        assert_eq!((presses, releases), (2, 2), "every press must be released");
    }

    /// Switching back to the game profile mid-press must not leave a button stuck down on the
    /// host, which would otherwise look like a broken mouse until the next click.
    #[test]
    fn release_all_lets_go_of_held_buttons() {
        let mut pad = DesktopPad::default();
        let state = DesktopPadState {
            cross: true,
            ..Default::default()
        };
        pad.update(state, 100, tick());
        assert_eq!(
            pad.release_all(),
            vec![DesktopPadAction::Mouse(MouseEvent::Button {
                button: MouseButton::Left,
                pressed: false
            })]
        );
        assert!(
            pad.release_all().is_empty(),
            "releasing twice must be a no-op"
        );
    }
}
