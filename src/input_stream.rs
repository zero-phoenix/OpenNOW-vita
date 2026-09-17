//! Layer 1 of the input stack: SDL events in, `opennow_core` values out.
//!
//! Everything that *decides* anything - which zone a finger landed in, what a button means, how a
//! stick deflection becomes cursor pixels - lives in `opennow_core` and is covered by tests that
//! run on an ordinary PC. What is left here is the part that genuinely cannot be: reading SDL and
//! holding the per-gesture state.
//!
//! The one rule that keeps this file honest: **ownership is decided once, on finger-down, and the
//! rest of the gesture follows it.** Re-deciding per event would let a drag that starts on the game
//! and ends on a button swallow the release, leaving the host holding a key down.

use crate::input::{AppCommand, FRONT_TOUCH_DEVICE_ID, REAR_TOUCH_DEVICE_ID};
use opennow_core::config::{ControlProfile, InputConfig};
use opennow_core::input::bindings::{Action, zone_action};
use opennow_core::input::layout::ZoneId;
use opennow_core::input::mapper::{
    self, DesktopPad, ModifierLatch, OutputEvent, TriggerZones, press_is_tap, rear_tap_button,
    scale_pointer_delta,
};
use opennow_core::input::physical::{PadState, Panel};
use opennow_core::input::router::{StickSide, TouchOwner, route_touch};
use opennow_core::protocol::MouseEvent;
use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use std::time::{Duration, Instant};

/// How far a slider drag has to travel before it is worth a step, in normalized screen units.
const SLIDER_STEP: f32 = 0.02;
/// Sensitivity points per slider step.
const DPI_STEP_PERCENT: i32 = 10;
/// Wheel delta per unit of scroll-rail travel. One full sweep of the rail is a few notches.
const SCROLL_RAIL_GAIN: f32 = 900.0;
const SCROLL_RAIL_LIMIT: f32 = 1200.0;

/// A contact the router has already assigned an owner to.
struct ActiveFinger {
    id: i64,
    owner: TouchOwner,
    last: (f32, f32),
    start: (f32, f32),
    down_at: Instant,
    /// Travelled far enough that this is a drag, not a tap. Latched, so a finger that wanders and
    /// comes back does not turn into a click on release.
    dragged: bool,
    /// Accumulated slider travel not yet turned into a step.
    slider_travel: f32,
}

impl ActiveFinger {
    fn travel_from_start(&self, x: f32, y: f32) -> f32 {
        let (dx, dy) = (x - self.start.0, y - self.start.1);
        (dx * dx + dy * dy).sqrt()
    }
}

/// All touch and pad handling for a streaming session.
#[derive(Default)]
pub struct StreamInput {
    fingers: Vec<ActiveFinger>,
    triggers: TriggerZones,
    latch: ModifierLatch,
    pad: DesktopPad,
    /// Front-screen stand-ins for L3/R3, by side. Multi-touch, so both can be held at once and
    /// releasing one must not cancel the other.
    stick_held: (bool, bool),
    pad_last_polled: Option<Instant>,
    /// Whether the L shoulder is held, i.e. whether the pointer is in precision mode. Polled
    /// rather than event-driven, so it is refreshed by `poll_pad`.
    precision: bool,
}

impl StreamInput {
    /// Feeds one SDL event through the router and returns what it implies.
    ///
    /// `ui_claims` answers "does the client's own interface have a widget at this point" - the one
    /// question that cannot be answered without egui.
    pub fn handle_event(
        &mut self,
        event: &Event,
        config: InputConfig,
        stream_size: (f32, f32),
        ui_claims: &dyn Fn(f32, f32) -> bool,
    ) -> Vec<OutputEvent> {
        let mut out = Vec::new();
        match *event {
            Event::FingerDown {
                touch_id,
                finger_id,
                x,
                y,
                ..
            } => {
                let Some(panel) = panel_of(touch_id) else {
                    return out;
                };
                let owner = route_touch(panel, x, y, config, panel == Panel::Front && ui_claims(x, y));
                self.begin(finger_id, owner, x, y);
                out.extend(self.on_press(owner, x, y));
            }
            Event::FingerMotion {
                touch_id,
                finger_id,
                x,
                y,
                ..
            } => {
                if panel_of(touch_id).is_none() {
                    return out;
                }
                out.extend(self.on_motion(finger_id, x, y, config, stream_size));
            }
            Event::FingerUp {
                touch_id,
                finger_id,
                ..
            } => {
                if panel_of(touch_id).is_none() {
                    return out;
                }
                out.extend(self.on_release(finger_id, config));
            }
            _ => {}
        }
        out
    }

    fn begin(&mut self, id: i64, owner: TouchOwner, x: f32, y: f32) {
        // A finger id can be reused after a lost `FingerUp`; replacing rather than pushing keeps
        // the list from growing without bound over a long session.
        self.fingers.retain(|finger| finger.id != id);
        self.fingers.push(ActiveFinger {
            id,
            owner,
            last: (x, y),
            start: (x, y),
            down_at: Instant::now(),
            dragged: false,
            slider_travel: 0.0,
        });
    }

    fn on_press(&mut self, owner: TouchOwner, x: f32, y: f32) -> Vec<OutputEvent> {
        match owner {
            TouchOwner::RearTrigger => {
                self.triggers.press(finger_key(x, y), x, y);
                Vec::new()
            }
            TouchOwner::StickZone(side) => {
                self.set_stick(side, true);
                Vec::new()
            }
            // Keys fire on *release*, not on press - see `on_release`. The eye and the profile
            // switch are the exception: they are client-side, harmless to repeat, and they should
            // feel instant.
            TouchOwner::Zone(ZoneId::Eye) => vec![OutputEvent::ToggleReveal],
            TouchOwner::Zone(ZoneId::ModeToggle) => vec![OutputEvent::ToggleProfile],
            _ => Vec::new(),
        }
    }

    fn on_motion(
        &mut self,
        id: i64,
        x: f32,
        y: f32,
        config: InputConfig,
        stream_size: (f32, f32),
    ) -> Vec<OutputEvent> {
        let Some(index) = self.fingers.iter().position(|finger| finger.id == id) else {
            return Vec::new();
        };
        let (owner, previous, travelled, start) = {
            let finger = &mut self.fingers[index];
            let previous = finger.last;
            finger.last = (x, y);
            if finger.travel_from_start(x, y) > mapper::REAR_TAP_MAX_TRAVEL {
                finger.dragged = true;
            }
            (finger.owner, previous, y - previous.1, finger.start)
        };

        match owner {
            TouchOwner::RearPointer | TouchOwner::Trackpad => {
                let sensitivity = effective_sensitivity(config, self.precision_held());
                let (dx, dy) = scale_pointer_delta(
                    x - previous.0,
                    y - previous.1,
                    stream_size,
                    sensitivity,
                );
                if dx == 0 && dy == 0 {
                    Vec::new()
                } else {
                    vec![OutputEvent::Mouse(MouseEvent::MoveBy { dx, dy })]
                }
            }
            TouchOwner::RearTrigger => {
                // The rear panel reports position continuously; re-registering keeps L2/R2
                // following a thumb that slides across the middle instead of sticking.
                self.triggers.release(finger_key(previous.0, previous.1));
                self.triggers.press(finger_key(x, y), x, y);
                Vec::new()
            }
            TouchOwner::Zone(ZoneId::DpiSlider) => {
                self.slider_steps(index, travelled, |steps| {
                    // Finger up (negative travel) raises sensitivity, matching every volume slider
                    // ever made.
                    vec![OutputEvent::AdjustSensitivity(-steps * DPI_STEP_PERCENT)]
                })
            }
            TouchOwner::Zone(ZoneId::ScrollSlider) => {
                // Finger down means the content follows it, i.e. scroll down, which is a negative
                // wheel delta under the +-120 convention.
                let delta = (-travelled * SCROLL_RAIL_GAIN)
                    .clamp(-SCROLL_RAIL_LIMIT, SCROLL_RAIL_LIMIT) as i16;
                if delta == 0 {
                    Vec::new()
                } else {
                    vec![OutputEvent::Mouse(MouseEvent::WheelBy { delta })]
                }
            }
            TouchOwner::StickZone(side) => {
                // A thumb that slides out of the corner releases the stick click, as a real button
                // would.
                let still_inside = route_touch(Panel::Front, x, y, config, false)
                    == TouchOwner::StickZone(side);
                self.set_stick(side, still_inside);
                Vec::new()
            }
            _ => {
                let _ = start;
                Vec::new()
            }
        }
    }

    fn slider_steps(
        &mut self,
        index: usize,
        travelled: f32,
        build: impl Fn(i32) -> Vec<OutputEvent>,
    ) -> Vec<OutputEvent> {
        let finger = &mut self.fingers[index];
        finger.slider_travel += travelled;
        let steps = (finger.slider_travel / SLIDER_STEP).trunc();
        if steps == 0.0 {
            return Vec::new();
        }
        finger.slider_travel -= steps * SLIDER_STEP;
        build(steps as i32)
    }

    fn on_release(&mut self, id: i64, config: InputConfig) -> Vec<OutputEvent> {
        let Some(index) = self.fingers.iter().position(|finger| finger.id == id) else {
            return Vec::new();
        };
        let finger = self.fingers.remove(index);
        let held = finger.down_at.elapsed();
        let travel = finger.travel_from_start(finger.last.0, finger.last.1);
        let tapped = !finger.dragged && press_is_tap(held, travel);

        match finger.owner {
            TouchOwner::RearTrigger => {
                self.triggers
                    .release(finger_key(finger.last.0, finger.last.1));
                self.triggers
                    .release(finger_key(finger.start.0, finger.start.1));
                Vec::new()
            }
            TouchOwner::StickZone(side) => {
                self.set_stick(side, false);
                Vec::new()
            }
            TouchOwner::RearPointer if tapped => {
                let button = rear_tap_button(finger.start.0);
                vec![
                    OutputEvent::Mouse(MouseEvent::Button {
                        button,
                        pressed: true,
                    }),
                    OutputEvent::Mouse(MouseEvent::Button {
                        button,
                        pressed: false,
                    }),
                ]
            }
            TouchOwner::Trackpad if tapped => {
                use opennow_core::protocol::MouseButton;
                vec![
                    OutputEvent::Mouse(MouseEvent::Button {
                        button: MouseButton::Left,
                        pressed: true,
                    }),
                    OutputEvent::Mouse(MouseEvent::Button {
                        button: MouseButton::Left,
                        pressed: false,
                    }),
                ]
            }
            // Keys fire on release, and only for a short, still press.
            //
            // This is the safeguard that makes a permanently visible key strip safe in the game
            // profile: a thumb resting on the top edge while playing does not fire Escape into the
            // title, because resting is neither short nor followed by a lift in place. Firing on
            // press - what v0.5.0 did - would have made the strip unusable there.
            TouchOwner::Zone(zone)
                if tapped && !matches!(zone, ZoneId::Eye | ZoneId::ModeToggle) =>
            {
                match zone_action(zone) {
                    Some(Action::DpiDrag) | Some(Action::ScrollDrag) | None => Vec::new(),
                    Some(action) => mapper::apply_action(action, &mut self.latch),
                }
            }
            _ => {
                let _ = config;
                Vec::new()
            }
        }
    }

    fn set_stick(&mut self, side: StickSide, held: bool) {
        match side {
            StickSide::Left => self.stick_held.0 = held,
            StickSide::Right => self.stick_held.1 = held,
        }
    }

    /// True while the L shoulder is being used as the precision modifier. Updated by `poll_pad`,
    /// since it is a polled button rather than an event.
    fn precision_held(&self) -> bool {
        self.precision
    }

    /// Polls the pad in the desktop profile. Called on its own cadence, not once per rendered
    /// frame: a render hiccup must not become input lag.
    pub fn poll_pad(&mut self, state: PadState, config: InputConfig, now: Instant) -> Vec<OutputEvent> {
        let dt = self
            .pad_last_polled
            .map(|last| now.duration_since(last))
            .unwrap_or(Duration::from_millis(8));
        self.pad_last_polled = Some(now);
        self.precision = state.l1;
        if !config.desktop_active() {
            return Vec::new();
        }
        self.pad
            .update(state, config.sensitivity_percent, dt)
    }

    /// Everything the host is still holding. Called when the profile switches and when the session
    /// ends - the two moments a key or button used to get stranded.
    pub fn release_all(&mut self) -> Vec<OutputEvent> {
        let mut out = self.pad.release_all();
        out.extend(self.latch.release_all());
        self.fingers.clear();
        self.triggers.clear();
        self.stick_held = (false, false);
        out
    }

    /// `(l2, r2)` from the rear panel, 0-255 each.
    pub fn triggers(&self, config: InputConfig) -> (u8, u8) {
        self.triggers.triggers(config)
    }

    /// `(l3, r3)`, from the front corners and, in quadrant mode, the rear panel.
    pub fn stick_clicks(&self, config: InputConfig) -> (bool, bool) {
        let (rear_l, rear_r) = self.triggers.stick_clicks(config);
        (self.stick_held.0 || rear_l, self.stick_held.1 || rear_r)
    }

    pub fn shift_latched(&self) -> bool {
        self.latch.shift()
    }

    pub fn ctrl_latched(&self) -> bool {
        self.latch.ctrl()
    }

    pub fn alt_latched(&self) -> bool {
        self.latch.alt()
    }

    /// Sends a key from the client's own UI - the on-screen keyboard - through the same latch the
    /// touch strips use, so a Shift set on the strip applies to a letter typed on the keyboard.
    pub fn send_key_latched(&mut self, key: opennow_core::protocol::KeyStroke) -> Vec<OutputEvent> {
        self.latch.send_latched(key)
    }

    pub fn send_chord(
        &mut self,
        shift: bool,
        ctrl: bool,
        alt: bool,
        win: bool,
        key: opennow_core::protocol::KeyStroke,
    ) -> Vec<OutputEvent> {
        self.latch.send_chord(shift, ctrl, alt, win, key)
    }

    pub fn toggle_modifier(&mut self, modifier: opennow_core::input::bindings::Modifier) {
        self.latch.toggle(modifier);
    }
}

/// Which panel an SDL touch device id refers to.
fn panel_of(touch_id: i64) -> Option<Panel> {
    match touch_id {
        FRONT_TOUCH_DEVICE_ID => Some(Panel::Front),
        REAR_TOUCH_DEVICE_ID => Some(Panel::Rear),
        _ => None,
    }
}

/// A stable key for a rear contact's zone, so `TriggerZones` can track it without SDL finger ids -
/// which the rear panel recycles aggressively.
fn finger_key(x: f32, y: f32) -> i64 {
    let half_x = if x < 0.5 { 0 } else { 1 };
    let half_y = if y < 0.5 { 0 } else { 1 };
    (half_y << 1) | half_x
}

fn effective_sensitivity(config: InputConfig, precision: bool) -> u16 {
    if precision {
        config.sensitivity_percent / 2
    } else {
        config.sensitivity_percent
    }
}

/// Reads the physical controls into a [`PadState`].
///
/// The registered Vita controller mapping puts Cross on `A`, Circle on `B`, Square on `X` and
/// Triangle on `Y`; the field names here are the Vita's, so the translation happens once, in one
/// place, instead of in every caller.
pub fn read_pad(controller: &GameController) -> PadState {
    let axis = |axis: Axis| f32::from(controller.axis(axis)) / f32::from(i16::MAX);
    let trigger = |value: i16| (value.max(0) / 129).min(255) as u8;
    PadState {
        left_stick: (axis(Axis::LeftX), axis(Axis::LeftY)),
        right_stick: (axis(Axis::RightX), axis(Axis::RightY)),
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
        l3: controller.button(Button::LeftStick),
        r3: controller.button(Button::RightStick),
        select: controller.button(Button::Back),
        start: controller.button(Button::Start),
        l2: trigger(controller.axis(Axis::TriggerLeft)),
        r2: trigger(controller.axis(Axis::TriggerRight)),
    }
}

/// The controller snapshot for the streaming session, in XInput conventions.
///
/// In the game profile this is built from the real hardware with no filtering at all - that is the
/// promise the profile makes. The desktop profile sends a neutral pad instead, from the shell.
pub fn gamepad_snapshot(
    pad: PadState,
    stream: &StreamInput,
    config: InputConfig,
) -> opennow_core::protocol::GamepadInput {
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
    let mut set = |held: bool, mask: u16| {
        if held {
            buttons |= mask;
        }
    };
    set(pad.dpad_up, DPAD_UP);
    set(pad.dpad_down, DPAD_DOWN);
    set(pad.dpad_left, DPAD_LEFT);
    set(pad.dpad_right, DPAD_RIGHT);
    set(pad.start, START);
    set(pad.select, BACK);
    set(pad.cross, A);
    set(pad.circle, B);
    set(pad.square, X);
    set(pad.triangle, Y);

    // Whichever source is pressing harder wins, so an attached DualShock on a Vita TV still works
    // while the rear panel covers the handheld.
    let (rear_l2, rear_r2) = stream.triggers(config);
    let phys_l2 = pad.l2.max(rear_l2);
    let phys_r2 = pad.r2.max(rear_r2);
    let (left_shoulder, right_shoulder, left_trigger, right_trigger) = if config.trigger_swap {
        (
            phys_l2 > 0,
            phys_r2 > 0,
            if pad.l1 { 255 } else { 0 },
            if pad.r1 { 255 } else { 0 },
        )
    } else {
        (pad.l1, pad.r1, phys_l2, phys_r2)
    };
    set(left_shoulder, LEFT_SHOULDER);
    set(right_shoulder, RIGHT_SHOULDER);

    let (zone_l3, zone_r3) = stream.stick_clicks(config);
    set(pad.l3 || zone_l3, LEFT_THUMB);
    set(pad.r3 || zone_r3, RIGHT_THUMB);

    let to_axis = |value: f32| (value * f32::from(i16::MAX)) as i16;
    opennow_core::protocol::GamepadInput {
        controller_id: 0,
        buttons,
        left_trigger,
        right_trigger,
        left_stick_x: to_axis(pad.left_stick.0),
        // XInput has +Y up, SDL reports +Y down.
        left_stick_y: to_axis(-pad.left_stick.1),
        right_stick_x: to_axis(pad.right_stick.0),
        right_stick_y: to_axis(-pad.right_stick.1),
        timestamp_us: 0,
    }
}

/// Translates a core output event into an `AppCommand`, for the ones the client handles itself.
pub fn app_command_for(event: OutputEvent) -> Option<AppCommand> {
    match event {
        OutputEvent::ToggleKeyboard => Some(AppCommand::ToggleKeyboard),
        OutputEvent::OpenSettings => Some(AppCommand::OpenSettings),
        OutputEvent::OpenShortcuts => Some(AppCommand::OpenShortcuts),
        OutputEvent::ToggleReveal => Some(AppCommand::ToggleOverlayReveal),
        OutputEvent::ToggleProfile => Some(AppCommand::ToggleControlProfile),
        OutputEvent::AdjustSensitivity(points) => Some(AppCommand::AdjustSensitivity(points)),
        OutputEvent::Mouse(_) | OutputEvent::Key { .. } => None,
    }
}

/// Live diagnostics, so a "the controls don't work" report can be checked against what the router
/// actually decided rather than guessed at from source.
pub mod stats {
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

    static LEFT: AtomicBool = AtomicBool::new(false);
    static RIGHT: AtomicBool = AtomicBool::new(false);
    static L2: AtomicU8 = AtomicU8::new(0);
    static R2: AtomicU8 = AtomicU8::new(0);

    pub(crate) fn record(left: bool, right: bool, l2: u8, r2: u8) {
        LEFT.store(left, Ordering::Relaxed);
        RIGHT.store(right, Ordering::Relaxed);
        L2.store(l2, Ordering::Relaxed);
        R2.store(r2, Ordering::Relaxed);
    }

    pub fn line() -> String {
        format!(
            "inp l3:{} r3:{} l2:{} r2:{}",
            u8::from(LEFT.load(Ordering::Relaxed)),
            u8::from(RIGHT.load(Ordering::Relaxed)),
            L2.load(Ordering::Relaxed),
            R2.load(Ordering::Relaxed),
        )
    }
}

/// The profile after a toggle, for the shell.
pub fn toggled_profile(profile: ControlProfile) -> ControlProfile {
    profile.toggled()
}
