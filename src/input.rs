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

/// One fixed hit-zone of the PC-touch overlay's front screen. Active only while
/// `stream_prefs::pc_overlay_enabled()` is true, and gated in `shell::run` to take priority over
/// `FrontStickZones`/the trackpad in the same way the existing UI rects do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcOverlayZone {
    /// Top-left corner: taps Escape.
    Esc,
    /// Top-right corner: opens the native settings menu and, while it is open, input stops
    /// reaching the game (the settings modal already claims touch via `stream_ui_rects`).
    OpenSettings,
    /// Left edge, vertical strip: dragging up/down raises/lowers the rear-panel trackpad's DPI.
    DpiSlider,
    /// Right edge, upper vertical strip: dragging up/down scrolls the mouse wheel.
    ScrollSlider,
    /// Right edge, between the scroll slider and the bottom-right corner: taps Enter.
    Enter,
    /// Bottom-left corner - deliberately the *same* geometry as `FrontStickZones`'s L3 corner.
    /// The overlay and the stick zones are mutually exclusive by construction: `shell::run` only
    /// asks `FrontStickZones` for a click when the overlay is off, and only asks
    /// `overlay_zone_at` for one when it is on, so the two can never fight over the same touch.
    LeftClick,
    /// Bottom-right corner - same relationship to `FrontStickZones`'s R3 corner as above.
    RightClick,
}

/// Top strip reserved for ESC (left) / open-settings (right). Chosen to sit fully above the
/// side sliders (which start at `OVERLAY_EDGE_TOP`) so none of the zones can overlap.
const OVERLAY_CORNER_SIZE: f32 = 0.22;
/// How wide the ESC/open-settings corners reach in from each side, and how far the DPI/scroll
/// sliders reach in from the left/right edges respectively.
const OVERLAY_EDGE_WIDTH: f32 = 0.14;
/// Vertical span shared by both side sliders. The bottom bound is deliberately
/// `STICK_ZONE_TOP` - the same constant `FrontStickZones` uses for its own top edge - so the
/// sliders end exactly where the click corners begin, with no gap or overlap between them.
const OVERLAY_EDGE_TOP: f32 = 0.34;
/// Splits the right edge's slider band into scroll (above) and Enter (below), per the spec:
/// "borde derecho, entre el scroll y el clic derecho: ENTER".
const OVERLAY_ENTER_TOP: f32 = 0.54;

/// Maps a normalized (0..1) front-touch position to the overlay zone it lands in, or `None` if
/// it is over the middle of the screen - i.e. still the game/trackpad's touch, not the overlay's.
pub fn overlay_zone_at(x: f32, y: f32) -> Option<PcOverlayZone> {
    if y < OVERLAY_CORNER_SIZE {
        if x < OVERLAY_CORNER_SIZE {
            return Some(PcOverlayZone::Esc);
        }
        if x >= 1.0 - OVERLAY_CORNER_SIZE {
            return Some(PcOverlayZone::OpenSettings);
        }
        return None;
    }
    if is_in_stick_zone(x, y) {
        return Some(if x < STICK_ZONE_WIDTH {
            PcOverlayZone::LeftClick
        } else {
            PcOverlayZone::RightClick
        });
    }
    if (OVERLAY_EDGE_TOP..STICK_ZONE_TOP).contains(&y) {
        if x < OVERLAY_EDGE_WIDTH {
            return Some(PcOverlayZone::DpiSlider);
        }
        if x >= 1.0 - OVERLAY_EDGE_WIDTH {
            return Some(if y < OVERLAY_ENTER_TOP {
                PcOverlayZone::ScrollSlider
            } else {
                PcOverlayZone::Enter
            });
        }
    }
    None
}

/// One entry from `overlay_zone_rects`: which zone, its label, and its normalized
/// `(x0, y0, x1, y1)` bounding box.
type OverlayZoneRect = (PcOverlayZone, &'static str, (f32, f32, f32, f32));

/// A normalized (0..1) rectangle for one overlay zone, paired with a short label key, for the
/// renderer in `app::ui` to draw. Kept in sync with `overlay_zone_at` by construction: both read
/// from the same constants, so the drawn boxes and the actual hit-test can never drift apart.
pub fn overlay_zone_rects() -> [OverlayZoneRect; 7] {
    [
        (
            PcOverlayZone::Esc,
            "ESC",
            (0.0, 0.0, OVERLAY_CORNER_SIZE, OVERLAY_CORNER_SIZE),
        ),
        (
            PcOverlayZone::OpenSettings,
            "\u{2699}",
            (1.0 - OVERLAY_CORNER_SIZE, 0.0, 1.0, OVERLAY_CORNER_SIZE),
        ),
        (
            PcOverlayZone::DpiSlider,
            "DPI",
            (0.0, OVERLAY_EDGE_TOP, OVERLAY_EDGE_WIDTH, STICK_ZONE_TOP),
        ),
        (
            PcOverlayZone::ScrollSlider,
            "\u{2195}",
            (
                1.0 - OVERLAY_EDGE_WIDTH,
                OVERLAY_EDGE_TOP,
                1.0,
                OVERLAY_ENTER_TOP,
            ),
        ),
        (
            PcOverlayZone::Enter,
            "\u{23ce}",
            (
                1.0 - OVERLAY_EDGE_WIDTH,
                OVERLAY_ENTER_TOP,
                1.0,
                STICK_ZONE_TOP,
            ),
        ),
        (
            PcOverlayZone::LeftClick,
            "LMB",
            (0.0, STICK_ZONE_TOP, STICK_ZONE_WIDTH, 1.0),
        ),
        (
            PcOverlayZone::RightClick,
            "RMB",
            (1.0 - STICK_ZONE_WIDTH, STICK_ZONE_TOP, 1.0, 1.0),
        ),
    ]
}

/// One user-facing effect of an overlay touch gesture. `shell::run` turns these into
/// `AppCommand`s / `MouseEvent`s / key taps; kept separate from those types so the mapping logic
/// here stays pure and unit-testable without an `App` or a live peer connection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PcOverlayAction {
    /// ESC / Enter: press-and-release paired with the finger, like a physical key.
    Key(PcOverlayZone),
    OpenSettings,
    /// Left/right click, held for as long as the finger is down - so drag-to-select still works.
    Click {
        right: bool,
        pressed: bool,
    },
    /// Vertical slider drag, normalized screen units (positive = finger moved down).
    DpiDelta(f32),
    ScrollDelta(f32),
}

/// Front-screen half of the PC-touch overlay: ESC/settings/DPI-slider/scroll-slider/enter/click.
/// Tracks which zone each active finger landed in on finger-down, so a finger that drifts out of
/// its zone mid-drag (sliders in particular) keeps controlling the same thing until it lifts.
#[derive(Default)]
pub struct PcOverlayTouch {
    // finger id -> (zone, last x, last y)
    fingers: Vec<(i64, PcOverlayZone, f32, f32)>,
}

impl PcOverlayTouch {
    pub fn handle(&mut self, event: &Event) -> Vec<PcOverlayAction> {
        let mut out = Vec::new();
        match *event {
            Event::FingerDown {
                touch_id,
                finger_id,
                x,
                y,
                ..
            } if touch_id == FRONT_TOUCH_DEVICE_ID => {
                let Some(zone) = overlay_zone_at(x, y) else {
                    return out;
                };
                self.fingers.push((finger_id, zone, x, y));
                match zone {
                    PcOverlayZone::Esc | PcOverlayZone::Enter => {
                        out.push(PcOverlayAction::Key(zone))
                    }
                    PcOverlayZone::OpenSettings => out.push(PcOverlayAction::OpenSettings),
                    PcOverlayZone::LeftClick => out.push(PcOverlayAction::Click {
                        right: false,
                        pressed: true,
                    }),
                    PcOverlayZone::RightClick => out.push(PcOverlayAction::Click {
                        right: true,
                        pressed: true,
                    }),
                    PcOverlayZone::DpiSlider | PcOverlayZone::ScrollSlider => {}
                }
            }
            Event::FingerMotion {
                touch_id,
                finger_id,
                x,
                y,
                ..
            } if touch_id == FRONT_TOUCH_DEVICE_ID => {
                if let Some(slot) = self.fingers.iter_mut().find(|(id, ..)| *id == finger_id) {
                    let (_, zone, last_x, last_y) = *slot;
                    let dy = y - last_y;
                    slot.2 = x;
                    slot.3 = y;
                    let _ = last_x;
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
                if let Some(pos) = self.fingers.iter().position(|(id, ..)| *id == finger_id) {
                    let (_, zone, _, _) = self.fingers.remove(pos);
                    match zone {
                        PcOverlayZone::LeftClick => out.push(PcOverlayAction::Click {
                            right: false,
                            pressed: false,
                        }),
                        PcOverlayZone::RightClick => out.push(PcOverlayAction::Click {
                            right: true,
                            pressed: false,
                        }),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        out
    }
}

/// Rear panel -> host mouse cursor, active only while the PC-touch overlay is on.
///
/// The NVST input protocol has no absolute-position mouse packet (see `INPUT_MOUSE_MOVE_REL`'s
/// doc comment in `gfn::input_protocol`) - only relative deltas - so, exactly like the front
/// screen's own `StreamTouchState` trackpad, this drives the cursor by the distance dragged
/// rather than by mapping touch position directly onto the host screen. `sensitivity_percent`
/// (100 = 1x) is the configurable "DPI": see `stream_prefs::OverlaySensitivity` and the
/// L-trigger "sniper mode" halving in `shell::run`.
#[derive(Default)]
pub struct RearOverlayMouse {
    last: Option<(f32, f32)>,
}

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

impl RearOverlayMouse {
    /// Translates one SDL event into the host-mouse move it implies, if any. Unlike the front
    /// screen's trackpad, a rear-panel tap never clicks: the panel is out of sight, so an
    /// accidental tap-click would be far harder to notice and undo than an accidental cursor
    /// nudge.
    pub fn map(
        &mut self,
        event: &Event,
        stream_size: (f32, f32),
        sensitivity_percent: u16,
    ) -> Option<crate::gfn::input_protocol::MouseEvent> {
        use crate::gfn::input_protocol::MouseEvent;
        match *event {
            Event::FingerDown { touch_id, x, y, .. } if touch_id == REAR_TOUCH_DEVICE_ID => {
                self.last = Some((x, y));
                None
            }
            Event::FingerMotion { touch_id, x, y, .. } if touch_id == REAR_TOUCH_DEVICE_ID => {
                let (prev_x, prev_y) = self.last?;
                self.last = Some((x, y));
                let (dx, dy) =
                    scale_rear_delta(x - prev_x, y - prev_y, stream_size, sensitivity_percent);
                if dx == 0 && dy == 0 {
                    return None;
                }
                Some(MouseEvent::MoveBy { dx, dy })
            }
            Event::FingerUp { touch_id, .. } if touch_id == REAR_TOUCH_DEVICE_ID => {
                self.last = None;
                None
            }
            _ => None,
        }
    }
}

/// Clears the D-Pad Up/Down bits from a gamepad snapshot's button field. Used while the PC-touch
/// overlay is active: those two buttons are repurposed as the Win+D / Ctrl+Alt+Del macros (see
/// `shell::run`), so they must stop reaching the game - otherwise a menu behind the overlay would
/// also see them as ordinary D-Pad input.
pub fn mask_overlay_dpad_updown(buttons: u16) -> u16 {
    const DPAD_UP: u16 = 0x0001;
    const DPAD_DOWN: u16 = 0x0002;
    buttons & !(DPAD_UP | DPAD_DOWN)
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

    /// The middle of the screen must stay mouse/game input even with the overlay on - otherwise
    /// the overlay would swallow the whole screen instead of just its fixed corners/edges.
    #[test]
    fn the_centre_of_the_screen_is_still_mouse() {
        assert_eq!(overlay_zone_at(0.5, 0.5), None);
        assert_eq!(
            overlay_zone_at(0.5, 0.1),
            None,
            "top centre, between the two corners"
        );
        assert_eq!(
            overlay_zone_at(0.5, 0.9),
            None,
            "bottom centre, between the two clicks"
        );
    }

    #[test]
    fn the_top_corners_are_esc_and_settings() {
        assert_eq!(overlay_zone_at(0.01, 0.01), Some(PcOverlayZone::Esc));
        assert_eq!(
            overlay_zone_at(0.99, 0.01),
            Some(PcOverlayZone::OpenSettings)
        );
    }

    #[test]
    fn the_bottom_corners_replace_the_stick_zones_exactly() {
        // Same geometry `FrontStickZones` uses for L3/R3 - the overlay's click corners must line
        // up exactly, since only one of the two owns a given touch (see `shell::run`).
        assert_eq!(overlay_zone_at(0.05, 0.95), Some(PcOverlayZone::LeftClick));
        assert_eq!(overlay_zone_at(0.95, 0.95), Some(PcOverlayZone::RightClick));
        assert!(is_in_stick_zone(0.05, 0.95));
        assert!(is_in_stick_zone(0.95, 0.95));
    }

    #[test]
    fn the_side_edges_are_the_dpi_and_scroll_sliders() {
        assert_eq!(overlay_zone_at(0.01, 0.5), Some(PcOverlayZone::DpiSlider));
        assert_eq!(
            overlay_zone_at(0.99, 0.4),
            Some(PcOverlayZone::ScrollSlider)
        );
    }

    #[test]
    fn the_right_edge_has_enter_between_scroll_and_the_click_corner() {
        assert_eq!(overlay_zone_at(0.99, 0.6), Some(PcOverlayZone::Enter));
    }

    #[test]
    fn tapping_the_esc_corner_emits_a_key_action() {
        let mut overlay = PcOverlayTouch::default();
        let actions = overlay.handle(&Event::FingerDown {
            timestamp: 0,
            touch_id: FRONT_TOUCH_DEVICE_ID,
            finger_id: 1,
            x: 0.01,
            y: 0.01,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
        });
        assert_eq!(actions, vec![PcOverlayAction::Key(PcOverlayZone::Esc)]);
    }

    #[test]
    fn holding_the_left_click_corner_presses_then_releases_on_lift() {
        let mut overlay = PcOverlayTouch::default();
        let down = overlay.handle(&Event::FingerDown {
            timestamp: 0,
            touch_id: FRONT_TOUCH_DEVICE_ID,
            finger_id: 7,
            x: 0.05,
            y: 0.95,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
        });
        assert_eq!(
            down,
            vec![PcOverlayAction::Click {
                right: false,
                pressed: true
            }]
        );
        let up = overlay.handle(&Event::FingerUp {
            timestamp: 0,
            touch_id: FRONT_TOUCH_DEVICE_ID,
            finger_id: 7,
            x: 0.05,
            y: 0.95,
            dx: 0.0,
            dy: 0.0,
            pressure: 0.0,
        });
        assert_eq!(
            up,
            vec![PcOverlayAction::Click {
                right: false,
                pressed: false
            }]
        );
    }

    #[test]
    fn dragging_the_dpi_slider_emits_deltas() {
        let mut overlay = PcOverlayTouch::default();
        overlay.handle(&Event::FingerDown {
            timestamp: 0,
            touch_id: FRONT_TOUCH_DEVICE_ID,
            finger_id: 3,
            x: 0.01,
            y: 0.4,
            dx: 0.0,
            dy: 0.0,
            pressure: 1.0,
        });
        let motion = overlay.handle(&Event::FingerMotion {
            timestamp: 0,
            touch_id: FRONT_TOUCH_DEVICE_ID,
            finger_id: 3,
            x: 0.01,
            y: 0.5,
            dx: 0.0,
            dy: 0.1,
            pressure: 1.0,
        });
        assert_eq!(motion, vec![PcOverlayAction::DpiDelta(0.1)]);
    }

    #[test]
    fn rear_delta_scales_with_sensitivity_percent() {
        let stream_size = (960.0, 544.0);
        let (dx_100, dy_100) = scale_rear_delta(0.1, 0.0, stream_size, 100);
        let (dx_50, dy_50) = scale_rear_delta(0.1, 0.0, stream_size, 50);
        let (dx_200, dy_200) = scale_rear_delta(0.1, 0.0, stream_size, 200);
        assert_eq!(dx_100, 96);
        assert_eq!(dy_100, 0);
        assert_eq!(
            dx_50, 48,
            "half sensitivity halves the delta (the L-trigger sniper mode)"
        );
        assert_eq!(dx_200, 192, "double sensitivity doubles the delta");
    }

    #[test]
    fn mask_overlay_dpad_updown_clears_only_up_and_down() {
        const DPAD_UP: u16 = 0x0001;
        const DPAD_DOWN: u16 = 0x0002;
        const DPAD_LEFT: u16 = 0x0004;
        const A: u16 = 0x1000;
        let all = DPAD_UP | DPAD_DOWN | DPAD_LEFT | A;
        assert_eq!(mask_overlay_dpad_updown(all), DPAD_LEFT | A);
    }
}
