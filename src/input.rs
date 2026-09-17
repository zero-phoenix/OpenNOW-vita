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
    /// A key with modifiers held around it as real key presses.
    ///
    /// All four modifiers are here, Shift included. It used to carry only ctrl/alt/win, with
    /// Shift handled as "pick the other character" - which works for typing `A` and not at all
    /// for `Ctrl+Shift+Esc`, so the entire family of Shift shortcuts was unreachable.
    SendChord {
        shift: bool,
        ctrl: bool,
        alt: bool,
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
    /// Master switch for the on-screen overlay (rear-panel mouse plus the front-panel key
    /// strips and slider rails). The layout itself lives in `opennow_core::input::layout`.
    TogglePcOverlay,
    SetOverlayOpacity(crate::gfn::stream_prefs::OverlayOpacity),
    SetOverlaySensitivity(crate::gfn::stream_prefs::OverlaySensitivity),
    /// Switches between the game and desktop control profiles - see
    /// `stream_prefs::ControlProfile`.
    SetControlProfile(crate::gfn::stream_prefs::ControlProfile),
    /// Swaps to the other control profile, from the on-screen switch.
    ToggleControlProfile,
    /// Shows or hides everything on the overlay except the eye.
    ToggleOverlayReveal,
    /// Opens the on-screen keyboard's Windows-shortcut page.
    OpenShortcuts,
    /// Nudges the pointer sensitivity by this many percentage points, from the DPI rail.
    AdjustSensitivity(i32),
    /// Turns the game profile's idle dimming of the key strip on and off.
    ToggleOverlayAutofade,
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
pub const REAR_TOUCH_DEVICE_ID: i64 = 2;

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


/// Streaming-session input lives in `crate::input_stream`, which is the SDL half of the mapping;
/// everything it decides comes from `opennow_core`, where it can be tested without a console.
pub use crate::input_stream::{StreamInput, gamepad_snapshot, read_pad, stats};
