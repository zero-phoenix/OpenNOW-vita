//! Turning hardware state into things to send. All pure: `Duration` in, events out, no clock of
//! its own, so every behaviour below is reproducible in a test.

use super::bindings::{Action, Modifier, MouseTarget};
use super::physical::{PadState, apply_deadzone};
use crate::config::{InputConfig, RearTouchMode};
use crate::protocol::{self, KeyStroke, MouseButton, MouseEvent};
use std::time::Duration;

/// One thing to send to the host, or one thing for the client to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputEvent {
    Mouse(MouseEvent),
    Key { key: KeyStroke, pressed: bool },
    ToggleKeyboard,
    OpenSettings,
    OpenShortcuts,
    ToggleReveal,
    ToggleProfile,
    /// Percentage points to add to the pointer sensitivity.
    AdjustSensitivity(i32),
}

impl OutputEvent {
    fn tap(key: KeyStroke) -> [OutputEvent; 2] {
        [
            OutputEvent::Key { key, pressed: true },
            OutputEvent::Key { key, pressed: false },
        ]
    }

    fn click(button: MouseButton) -> [OutputEvent; 2] {
        [
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
}

fn button_of(target: MouseTarget) -> MouseButton {
    match target {
        MouseTarget::Left => MouseButton::Left,
        MouseTarget::Right => MouseButton::Right,
    }
}

/// Holds modifier keys down around a chord and guarantees they come back up.
///
/// The invariant is the whole point: **a host that is left holding Ctrl is worse than a chord that
/// never fired.** The session can end, the profile can switch, the player can quit mid-press - so
/// `release_all` exists and is called on every one of those paths, and
/// `every_press_is_eventually_released` proves it over random input.
#[derive(Debug, Default)]
pub struct ModifierLatch {
    shift: bool,
    ctrl: bool,
    alt: bool,
    win: bool,
    /// Keys sent as `down` and not yet as `up`. A latched modifier is *pending*, not pressed -
    /// conflating the two would make `release_all` send releases for keys the host never saw
    /// pressed, which is its own kind of stuck-key bug.
    held: Vec<KeyStroke>,
}

impl ModifierLatch {
    pub fn shift(&self) -> bool {
        self.shift
    }

    pub fn ctrl(&self) -> bool {
        self.ctrl
    }

    pub fn alt(&self) -> bool {
        self.alt
    }

    pub fn toggle(&mut self, modifier: Modifier) {
        match modifier {
            Modifier::Shift => self.shift = !self.shift,
            Modifier::Ctrl => self.ctrl = !self.ctrl,
            Modifier::Alt => self.alt = !self.alt,
        }
    }

    /// Sends `key` wrapped in whatever is latched, then clears the latch - sticky modifiers are
    /// for one keystroke, like a phone keyboard's shift.
    pub fn send_latched(&mut self, key: KeyStroke) -> Vec<OutputEvent> {
        let (shift, ctrl, alt, win) = (self.shift, self.ctrl, self.alt, self.win);
        self.shift = false;
        self.ctrl = false;
        self.alt = false;
        self.win = false;
        self.wrap(shift, ctrl, alt, win, key)
    }

    /// Sends a chord with explicit modifiers, independent of anything latched.
    pub fn send_chord(
        &mut self,
        shift: bool,
        ctrl: bool,
        alt: bool,
        win: bool,
        key: KeyStroke,
    ) -> Vec<OutputEvent> {
        self.wrap(shift, ctrl, alt, win, key)
    }

    /// Modifiers go down outermost-first and come up in reverse, the way a hand does it: Windows
    /// treats Win+Ctrl+Alt+Del as an ordered sequence, not a set.
    fn wrap(
        &mut self,
        shift: bool,
        ctrl: bool,
        alt: bool,
        win: bool,
        key: KeyStroke,
    ) -> Vec<OutputEvent> {
        let modifiers = [
            (win, protocol::KEY_LEFT_WIN),
            (ctrl, protocol::KEY_LEFT_CTRL),
            (alt, protocol::KEY_LEFT_ALT),
            (shift, protocol::KEY_LEFT_SHIFT),
        ];
        let mut events = Vec::with_capacity(10);
        for (active, modifier) in modifiers {
            if active {
                events.push(self.press(modifier));
            }
        }
        events.push(self.press(key));
        events.push(self.release(key));
        for (active, modifier) in modifiers.iter().rev() {
            if *active {
                events.push(self.release(*modifier));
            }
        }
        events
    }

    fn press(&mut self, key: KeyStroke) -> OutputEvent {
        self.held.push(key);
        OutputEvent::Key { key, pressed: true }
    }

    fn release(&mut self, key: KeyStroke) -> OutputEvent {
        if let Some(index) = self.held.iter().rposition(|held| *held == key) {
            self.held.remove(index);
        }
        OutputEvent::Key {
            key,
            pressed: false,
        }
    }

    /// Releases everything the host is actually holding and forgets any pending latch.
    ///
    /// Called when the session ends, when the profile switches and when the stream closes - the
    /// three ways a modifier used to get stranded on a host that then behaved as if Ctrl were
    /// welded down. A *pending* latch sends nothing here: the host never saw it pressed, so
    /// releasing it would be a release with no matching press.
    pub fn release_all(&mut self) -> Vec<OutputEvent> {
        let mut events = Vec::with_capacity(self.held.len());
        while let Some(key) = self.held.pop() {
            events.push(OutputEvent::Key {
                key,
                pressed: false,
            });
        }
        self.shift = false;
        self.ctrl = false;
        self.alt = false;
        self.win = false;
        events
    }
}

/// Turns one overlay action into output events.
pub fn apply_action(action: Action, latch: &mut ModifierLatch) -> Vec<OutputEvent> {
    match action {
        Action::ToggleReveal => vec![OutputEvent::ToggleReveal],
        Action::ToggleProfile => vec![OutputEvent::ToggleProfile],
        Action::Key(key) => latch.send_latched(key),
        Action::Chord {
            shift,
            ctrl,
            alt,
            win,
            key,
        } => latch.send_chord(shift, ctrl, alt, win, key),
        Action::OpenSettings => vec![OutputEvent::OpenSettings],
        Action::ToggleKeyboard => vec![OutputEvent::ToggleKeyboard],
        Action::OpenShortcuts => vec![OutputEvent::OpenShortcuts],
        Action::Latch(modifier) => {
            latch.toggle(modifier);
            Vec::new()
        }
        Action::DpiDrag | Action::ScrollDrag => Vec::new(),
    }
}

/// Sticks below this fraction of full deflection count as centred. The Vita's sticks rest
/// noticeably off-centre once worn, so this is deliberately generous - and it applies only in the
/// desktop profile, never to what the title sees.
pub const PAD_STICK_DEADZONE: f32 = 0.22;
/// Cursor speed at full deflection, host pixels per second at 100 %.
const PAD_CURSOR_PIXELS_PER_SEC: f32 = 620.0;
/// Wheel notches per second at full right-stick deflection.
const PAD_SCROLL_NOTCHES_PER_SEC: f32 = 9.0;
/// One Windows wheel notch.
pub const WHEEL_NOTCH: i16 = 120;
const PAD_REPEAT_DELAY: Duration = Duration::from_millis(400);
const PAD_REPEAT_INTERVAL: Duration = Duration::from_millis(70);

/// The desktop profile's pad mapping.
///
/// Fractional movement is accumulated between updates, so a slow stick nudge produces motion
/// instead of rounding to zero on every frame and never moving at all.
#[derive(Debug, Default)]
pub struct DesktopPad {
    previous: PadState,
    cursor_remainder: (f32, f32),
    scroll_remainder: f32,
    repeat: Option<(KeyStroke, Duration)>,
    held: Duration,
}

impl DesktopPad {
    pub fn update(
        &mut self,
        state: PadState,
        sensitivity_percent: u16,
        dt: Duration,
    ) -> Vec<OutputEvent> {
        let mut out = Vec::new();
        let seconds = dt.as_secs_f32();
        let mut scale = f32::from(sensitivity_percent) / 100.0;
        if state.l1 {
            // Precision: the same halving the rear panel gets, so both pointers agree.
            scale *= 0.5;
        }

        // --- left stick: fine cursor ---
        let (sx, sy) = (
            apply_deadzone(state.left_stick.0, PAD_STICK_DEADZONE),
            apply_deadzone(state.left_stick.1, PAD_STICK_DEADZONE),
        );
        if sx != 0.0 || sy != 0.0 {
            let step = PAD_CURSOR_PIXELS_PER_SEC * scale * seconds;
            self.cursor_remainder.0 += sx * step;
            self.cursor_remainder.1 += sy * step;
        } else {
            self.cursor_remainder = (0.0, 0.0);
        }
        let (dx, dy) = (
            self.cursor_remainder.0.trunc(),
            self.cursor_remainder.1.trunc(),
        );
        if dx != 0.0 || dy != 0.0 {
            self.cursor_remainder.0 -= dx;
            self.cursor_remainder.1 -= dy;
            out.push(OutputEvent::Mouse(MouseEvent::MoveBy {
                dx: clamp_i16(dx),
                dy: clamp_i16(dy),
            }));
        }

        // --- right stick: scroll wheel ---
        let scroll = apply_deadzone(state.right_stick.1, PAD_STICK_DEADZONE);
        if scroll != 0.0 {
            self.scroll_remainder += scroll * PAD_SCROLL_NOTCHES_PER_SEC * seconds;
        } else {
            self.scroll_remainder = 0.0;
        }
        let notches = self.scroll_remainder.trunc();
        if notches != 0.0 {
            self.scroll_remainder -= notches;
            // Stick down (positive y) scrolls the page down, which is a negative wheel delta under
            // the +-120 convention.
            out.push(OutputEvent::Mouse(MouseEvent::WheelBy {
                delta: clamp_i16(-notches * f32::from(WHEEL_NOTCH)),
            }));
        }

        // --- d-pad: arrows with hold-to-repeat ---
        let direction = if state.dpad_up {
            Some(protocol::KEY_UP)
        } else if state.dpad_down {
            Some(protocol::KEY_DOWN)
        } else if state.dpad_left {
            Some(protocol::KEY_LEFT)
        } else if state.dpad_right {
            Some(protocol::KEY_RIGHT)
        } else {
            None
        };
        match direction {
            Some(key) if self.repeat.map(|(k, _)| k) == Some(key) => {
                self.held += dt;
                if let Some((_, next_at)) = self.repeat
                    && self.held >= next_at
                {
                    out.extend(OutputEvent::tap(key));
                    self.repeat = Some((key, self.held + PAD_REPEAT_INTERVAL));
                }
            }
            Some(key) => {
                out.extend(OutputEvent::tap(key));
                self.held = Duration::ZERO;
                self.repeat = Some((key, PAD_REPEAT_DELAY));
            }
            None => {
                self.repeat = None;
                self.held = Duration::ZERO;
            }
        }

        // --- face buttons: Cross/Circle are held, so click-and-drag works ---
        for (now, before, target) in [
            (state.cross, self.previous.cross, MouseTarget::Left),
            (state.circle, self.previous.circle, MouseTarget::Right),
        ] {
            if now != before {
                out.push(OutputEvent::Mouse(MouseEvent::Button {
                    button: button_of(target),
                    pressed: now,
                }));
            }
        }
        let pressed = |now: bool, before: bool| now && !before;
        if pressed(state.triangle, self.previous.triangle) {
            out.extend(OutputEvent::tap(protocol::KEY_ENTER));
        }
        if pressed(state.square, self.previous.square) {
            out.extend(OutputEvent::tap(protocol::KEY_BACKSPACE));
        }
        if pressed(state.r1, self.previous.r1) {
            out.extend(OutputEvent::click(MouseButton::Left));
            out.extend(OutputEvent::click(MouseButton::Left));
        }
        if pressed(state.select, self.previous.select) {
            out.push(OutputEvent::ToggleKeyboard);
        }
        if pressed(state.start, self.previous.start) {
            out.extend(OutputEvent::tap(protocol::KEY_LEFT_WIN));
        }

        self.previous = state;
        out
    }

    /// Releases anything still held, for when the profile switches mid-press.
    pub fn release_all(&mut self) -> Vec<OutputEvent> {
        let mut out = Vec::new();
        for (held, target) in [
            (self.previous.cross, MouseTarget::Left),
            (self.previous.circle, MouseTarget::Right),
        ] {
            if held {
                out.push(OutputEvent::Mouse(MouseEvent::Button {
                    button: button_of(target),
                    pressed: false,
                }));
            }
        }
        *self = Self::default();
        out
    }
}

fn clamp_i16(value: f32) -> i16 {
    value.clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

/// Longest press still counted as a tap rather than a drag.
pub const REAR_TAP_MAX_HOLD: Duration = Duration::from_millis(300);
/// Furthest a finger may wander (normalized panel units) and still count as a tap. The rear panel
/// reports a noisy position, so simply resting a finger must not disqualify the tap.
pub const REAR_TAP_MAX_TRAVEL: f32 = 0.05;

/// Scales a normalized rear-panel delta into host pixels.
///
/// The NVST protocol has no absolute-position packet, only relative deltas, so the panel is a
/// trackpad rather than a direct-touch pointer: the cursor moves by the distance dragged.
pub fn scale_pointer_delta(
    dx: f32,
    dy: f32,
    stream_size: (f32, f32),
    sensitivity_percent: u16,
) -> (i16, i16) {
    let scale = f32::from(sensitivity_percent) / 100.0;
    (
        clamp_i16((dx * stream_size.0 * scale).round()),
        clamp_i16((dy * stream_size.1 * scale).round()),
    )
}

/// Whether a press qualifies as a tap - and therefore a click - rather than a cursor drag.
pub fn press_is_tap(held: Duration, travel: f32) -> bool {
    held <= REAR_TAP_MAX_HOLD && travel <= REAR_TAP_MAX_TRAVEL
}

/// Which button a rear tap means, from where it *started*: splitting by the lift position instead
/// would let a tap that drifts a millimetre change buttons.
pub fn rear_tap_button(start_x: f32) -> MouseButton {
    if start_x >= 0.5 {
        MouseButton::Right
    } else {
        MouseButton::Left
    }
}

/// The rear panel as the analog triggers, in the game profile.
///
/// Split down the middle: left half is L2, right half R2. Quadrants were tried and dropped - the
/// panel is about 2.5 cm tall with no landmark to tell top from bottom by feel, so a quadrant is
/// 1.2 cm of guesswork and reaching for L2 can give L3.
#[derive(Debug, Default, Clone)]
pub struct TriggerZones {
    fingers: Vec<(i64, f32, f32)>,
}

impl TriggerZones {
    pub fn press(&mut self, id: i64, x: f32, y: f32) {
        match self.fingers.iter_mut().find(|(slot, _, _)| *slot == id) {
            Some(slot) => {
                slot.1 = x;
                slot.2 = y;
            }
            None => self.fingers.push((id, x, y)),
        }
    }

    pub fn release(&mut self, id: i64) {
        self.fingers.retain(|(slot, _, _)| *slot != id);
    }

    pub fn clear(&mut self) {
        self.fingers.clear();
    }

    fn half_held(&self, left: bool) -> bool {
        self.fingers
            .iter()
            .any(|(_, x, _)| if left { *x < 0.5 } else { *x >= 0.5 })
    }

    fn quadrant_held(&self, left: bool, top: bool) -> bool {
        self.fingers.iter().any(|(_, x, y)| {
            let x_ok = if left { *x < 0.5 } else { *x >= 0.5 };
            let y_ok = if top { *y < 0.5 } else { *y >= 0.5 };
            x_ok && y_ok
        })
    }

    /// `(left_trigger, right_trigger)`, 0-255 each.
    pub fn triggers(&self, config: InputConfig) -> (u8, u8) {
        let pressure = if config.trigger_pressure == 0 {
            255
        } else {
            config.trigger_pressure
        };
        let held = |left: bool| match config.rear_touch_mode {
            RearTouchMode::Halves => self.half_held(left),
            RearTouchMode::Quadrant => self.quadrant_held(left, true),
        };
        (
            if held(true) { pressure } else { 0 },
            if held(false) { pressure } else { 0 },
        )
    }

    /// `(l3, r3)` from the bottom quadrants, when that layout is selected.
    pub fn stick_clicks(&self, config: InputConfig) -> (bool, bool) {
        match config.rear_touch_mode {
            RearTouchMode::Quadrant => (
                self.quadrant_held(true, false),
                self.quadrant_held(false, false),
            ),
            RearTouchMode::Halves => (false, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ControlProfile;

    fn latch() -> ModifierLatch {
        ModifierLatch::default()
    }

    /// A latch is a promise about the *next* key, not a key the host is holding. Cancelling one
    /// must therefore put nothing on the wire at all.
    #[test]
    fn a_pending_latch_that_is_never_used_sends_nothing() {
        let mut latch = latch();
        latch.toggle(Modifier::Ctrl);
        latch.toggle(Modifier::Shift);
        assert!(
            latch.release_all().is_empty(),
            "releasing a pending latch would be a release with no matching press"
        );
        assert!(!latch.ctrl() && !latch.shift());
    }

    /// Nothing may be left pressed on the host. This walks a pseudo-random sequence of latches,
    /// chords and profile switches and asserts the books balance every time.
    #[test]
    fn every_press_is_eventually_released() {
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let modifiers = [Modifier::Shift, Modifier::Ctrl, Modifier::Alt];
        let keys = [
            protocol::KEY_ESCAPE,
            protocol::KEY_DELETE,
            protocol::KEY_TAB,
            protocol::KEY_F4,
        ];

        for _ in 0..500 {
            let mut latch = latch();
            let mut events = Vec::new();
            for _ in 0..12 {
                match next() % 4 {
                    0 => latch.toggle(modifiers[(next() % 3) as usize]),
                    1 => events.extend(latch.send_latched(keys[(next() % 4) as usize])),
                    2 => {
                        let bits = next();
                        events.extend(latch.send_chord(
                            bits & 1 != 0,
                            bits & 2 != 0,
                            bits & 4 != 0,
                            bits & 8 != 0,
                            keys[(next() % 4) as usize],
                        ));
                    }
                    _ => events.extend(latch.release_all()),
                }
            }
            // However the sequence ended, closing the session must settle every key.
            events.extend(latch.release_all());

            let mut depth = std::collections::BTreeMap::new();
            for event in &events {
                if let OutputEvent::Key { key, pressed } = event {
                    let entry = depth.entry((key.keycode, key.scancode)).or_insert(0i32);
                    *entry += if *pressed { 1 } else { -1 };
                    assert!(*entry >= 0, "released {key:?} more times than it was pressed");
                }
            }
            for ((keycode, _), count) in depth {
                assert_eq!(count, 0, "key {keycode:#x} was left held on the host");
            }
        }
    }

    #[test]
    fn a_latched_modifier_applies_to_exactly_one_key() {
        let mut latch = latch();
        latch.toggle(Modifier::Ctrl);
        let first = latch.send_latched(protocol::KEY_DELETE);
        assert!(
            first.contains(&OutputEvent::Key {
                key: protocol::KEY_LEFT_CTRL,
                pressed: true
            }),
            "the latch applies to the first key"
        );
        let second = latch.send_latched(protocol::KEY_DELETE);
        assert!(
            !second.contains(&OutputEvent::Key {
                key: protocol::KEY_LEFT_CTRL,
                pressed: true
            }),
            "and is spent afterwards"
        );
    }

    /// Modifiers go down outermost-first and come up in reverse, the way a hand does it.
    #[test]
    fn modifiers_nest_around_the_key() {
        let events = latch().send_chord(true, true, false, false, protocol::KEY_ESCAPE);
        let order: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                OutputEvent::Key { key, pressed } => Some((key.keycode, *pressed)),
                _ => None,
            })
            .collect();
        assert_eq!(
            order,
            vec![
                (protocol::KEY_LEFT_CTRL.keycode, true),
                (protocol::KEY_LEFT_SHIFT.keycode, true),
                (protocol::KEY_ESCAPE.keycode, true),
                (protocol::KEY_ESCAPE.keycode, false),
                (protocol::KEY_LEFT_SHIFT.keycode, false),
                (protocol::KEY_LEFT_CTRL.keycode, false),
            ]
        );
    }

    /// The slow-nudge bug: without sub-pixel accumulation a gentle stick deflection rounds to zero
    /// every frame and the cursor never moves at all.
    #[test]
    fn a_slow_stick_nudge_still_moves_the_cursor() {
        let mut pad = DesktopPad::default();
        let state = PadState {
            left_stick: (0.30, 0.0),
            ..Default::default()
        };
        let mut moved = 0;
        for _ in 0..60 {
            for event in pad.update(state, 100, Duration::from_millis(16)) {
                if let OutputEvent::Mouse(MouseEvent::MoveBy { dx, .. }) = event {
                    moved += i32::from(dx);
                }
            }
        }
        assert!(moved > 0, "a second of gentle deflection moved {moved} pixels");
    }

    #[test]
    fn a_centred_stick_moves_nothing() {
        let mut pad = DesktopPad::default();
        let state = PadState {
            left_stick: (0.1, -0.15),
            ..Default::default()
        };
        for _ in 0..60 {
            assert!(
                pad.update(state, 100, Duration::from_millis(16))
                    .iter()
                    .all(|event| !matches!(event, OutputEvent::Mouse(MouseEvent::MoveBy { .. })))
            );
        }
    }

    #[test]
    fn precision_halves_the_cursor_speed() {
        let travel = |precision: bool| {
            let mut pad = DesktopPad::default();
            let state = PadState {
                left_stick: (1.0, 0.0),
                l1: precision,
                ..Default::default()
            };
            let mut moved = 0;
            for _ in 0..60 {
                for event in pad.update(state, 100, Duration::from_millis(16)) {
                    if let OutputEvent::Mouse(MouseEvent::MoveBy { dx, .. }) = event {
                        moved += i32::from(dx);
                    }
                }
            }
            moved
        };
        let (full, half) = (travel(false), travel(true));
        assert!(full > 0);
        let ratio = half as f32 / full as f32;
        assert!((ratio - 0.5).abs() < 0.05, "precision gave {ratio} of full speed");
    }

    #[test]
    fn the_dpad_fires_once_then_repeats() {
        let mut pad = DesktopPad::default();
        let state = PadState {
            dpad_down: true,
            ..Default::default()
        };
        let count = |events: Vec<OutputEvent>| {
            events
                .iter()
                .filter(|event| {
                    matches!(event, OutputEvent::Key { key, pressed: true } if *key == protocol::KEY_DOWN)
                })
                .count()
        };
        assert_eq!(count(pad.update(state, 100, Duration::from_millis(16))), 1);
        // Well inside the repeat delay: still nothing.
        assert_eq!(count(pad.update(state, 100, Duration::from_millis(100))), 0);
        // Past it: repeats.
        assert_eq!(count(pad.update(state, 100, Duration::from_millis(400))), 1);
    }

    /// Cross is held rather than tapped, which is what makes click-and-drag possible.
    #[test]
    fn cross_holds_the_left_button_and_releasing_it_releases_the_button() {
        let mut pad = DesktopPad::default();
        let down = PadState {
            cross: true,
            ..Default::default()
        };
        let events = pad.update(down, 100, Duration::from_millis(16));
        assert!(events.contains(&OutputEvent::Mouse(MouseEvent::Button {
            button: MouseButton::Left,
            pressed: true
        })));
        let events = pad.update(PadState::default(), 100, Duration::from_millis(16));
        assert!(events.contains(&OutputEvent::Mouse(MouseEvent::Button {
            button: MouseButton::Left,
            pressed: false
        })));
    }

    /// Switching profiles mid-press must not leave the host holding a mouse button.
    #[test]
    fn switching_profiles_mid_click_releases_the_button() {
        let mut pad = DesktopPad::default();
        pad.update(
            PadState {
                cross: true,
                ..Default::default()
            },
            100,
            Duration::from_millis(16),
        );
        let released = pad.release_all();
        assert!(released.contains(&OutputEvent::Mouse(MouseEvent::Button {
            button: MouseButton::Left,
            pressed: false
        })));
    }

    #[test]
    fn stick_down_scrolls_the_page_down() {
        let mut pad = DesktopPad::default();
        let state = PadState {
            right_stick: (0.0, 1.0),
            ..Default::default()
        };
        let mut delta = 0;
        for _ in 0..60 {
            for event in pad.update(state, 100, Duration::from_millis(16)) {
                if let OutputEvent::Mouse(MouseEvent::WheelBy { delta: d }) = event {
                    delta += i32::from(d);
                }
            }
        }
        assert!(delta < 0, "scrolling down is a negative wheel delta, got {delta}");
    }

    #[test]
    fn a_quick_still_press_is_a_click_and_a_drag_is_not() {
        assert!(press_is_tap(Duration::from_millis(120), 0.01));
        assert!(!press_is_tap(Duration::from_millis(900), 0.01), "too slow");
        assert!(!press_is_tap(Duration::from_millis(120), 0.30), "too far");
    }

    #[test]
    fn the_rear_panel_splits_its_buttons_down_the_middle() {
        assert_eq!(rear_tap_button(0.2), MouseButton::Left);
        assert_eq!(rear_tap_button(0.8), MouseButton::Right);
    }

    #[test]
    fn pointer_sensitivity_scales_the_delta() {
        let (slow, _) = scale_pointer_delta(0.1, 0.0, (960.0, 544.0), 50);
        let (fast, _) = scale_pointer_delta(0.1, 0.0, (960.0, 544.0), 200);
        assert_eq!(slow, 48);
        assert_eq!(fast, 192);
    }

    #[test]
    fn both_rear_halves_can_be_held_at_once() {
        let mut zones = TriggerZones::default();
        zones.press(1, 0.2, 0.5);
        zones.press(2, 0.8, 0.5);
        let config = InputConfig::default();
        assert_eq!(zones.triggers(config), (255, 255));
        zones.release(1);
        assert_eq!(zones.triggers(config), (0, 255));
    }

    #[test]
    fn trigger_pressure_is_configurable() {
        let mut zones = TriggerZones::default();
        zones.press(1, 0.2, 0.5);
        let config = InputConfig {
            trigger_pressure: 128,
            ..Default::default()
        };
        assert_eq!(zones.triggers(config), (128, 0));
    }

    #[test]
    fn halves_mode_never_reports_stick_clicks() {
        let mut zones = TriggerZones::default();
        zones.press(1, 0.2, 0.9);
        assert_eq!(zones.stick_clicks(InputConfig::default()), (false, false));
    }

    #[test]
    fn quadrant_mode_puts_the_stick_clicks_on_the_bottom_half() {
        let mut zones = TriggerZones::default();
        zones.press(1, 0.2, 0.9);
        let config = InputConfig {
            rear_touch_mode: RearTouchMode::Quadrant,
            ..Default::default()
        };
        assert_eq!(zones.stick_clicks(config), (true, false));
        assert_eq!(zones.triggers(config), (0, 0), "the bottom half is not L2");
    }

    #[test]
    fn latching_a_modifier_sends_nothing_on_its_own() {
        let mut latch = latch();
        assert!(apply_action(Action::Latch(Modifier::Shift), &mut latch).is_empty());
        assert!(latch.shift());
    }

    #[test]
    fn the_profile_toggle_is_reported_not_sent() {
        let mut latch = latch();
        assert_eq!(
            apply_action(Action::ToggleProfile, &mut latch),
            vec![OutputEvent::ToggleProfile]
        );
        assert_eq!(ControlProfile::Game.toggled(), ControlProfile::Desktop);
        assert_eq!(ControlProfile::Desktop.toggled(), ControlProfile::Game);
    }
}
