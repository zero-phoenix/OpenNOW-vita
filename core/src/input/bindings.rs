//! What each control means, as a table rather than as code.
//!
//! This is the switchboard. A control has exactly one row per profile; changing what a button does
//! is editing a row, not rewiring the event loop. Two consequences worth the trouble:
//!
//! - A clash is a failing test (`no_control_has_two_meanings_in_one_profile`), not a bug report
//!   from someone mid-game.
//! - The on-screen control manual is *generated from this table*, so it cannot describe a layout
//!   the client does not actually have - which is exactly what a hand-written manual drifts into.

use super::layout::ZoneId;
use crate::config::ControlProfile;
use crate::protocol::{self, KeyStroke};

/// Which host mouse button a control drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseTarget {
    Left,
    Right,
}

/// A modifier that latches until the next key is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    Shift,
    Ctrl,
    Alt,
}

/// What touching an overlay zone does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Show or hide everything except the eye.
    ToggleReveal,
    /// Swap between the game and desktop profiles.
    ToggleProfile,
    /// A plain key tap, press and release paired with the finger.
    Key(KeyStroke),
    /// A modified key tap. Every modifier is sent as a real key down/up around the tap, including
    /// Shift - `SendChord` used to carry only Ctrl/Alt/Win, which made Ctrl+Shift+Esc impossible.
    Chord {
        shift: bool,
        ctrl: bool,
        alt: bool,
        win: bool,
        key: KeyStroke,
    },
    OpenSettings,
    ToggleKeyboard,
    /// Opens the keyboard's Windows-shortcut page.
    OpenShortcuts,
    /// Latches a modifier for the next key.
    Latch(Modifier),
    /// Vertical drag on the DPI rail.
    DpiDrag,
    /// Vertical drag on the scroll rail.
    ScrollDrag,
}

/// The action a zone fires. `None` for the stick corners, which are pad buttons rather than
/// overlay controls and are handled by the gamepad snapshot.
pub fn zone_action(zone: ZoneId) -> Option<Action> {
    // Letters go through the same lookup the on-screen keyboard uses, so Ctrl+C from the strip and
    // Ctrl+C typed on the keyboard are guaranteed to send identical bytes.
    let letter = |ch: char| protocol::key_for_char(ch).expect("an ASCII letter always maps");
    let chord = |shift, ctrl, alt, win, key| Action::Chord {
        shift,
        ctrl,
        alt,
        win,
        key,
    };

    Some(match zone {
        ZoneId::Eye => Action::ToggleReveal,
        ZoneId::ModeToggle => Action::ToggleProfile,
        ZoneId::Esc => Action::Key(protocol::KEY_ESCAPE),
        ZoneId::Enter => Action::Key(protocol::KEY_ENTER),
        ZoneId::Tab => Action::Key(protocol::KEY_TAB),
        ZoneId::Win => Action::Key(protocol::KEY_LEFT_WIN),
        ZoneId::Backspace => Action::Key(protocol::KEY_BACKSPACE),
        ZoneId::Delete => Action::Key(protocol::KEY_DELETE),
        ZoneId::AltTab => chord(false, false, true, false, protocol::KEY_TAB),
        ZoneId::AltF4 => chord(false, false, true, false, protocol::KEY_F4),
        ZoneId::Copy => chord(false, true, false, false, letter('c')),
        ZoneId::Paste => chord(false, true, false, false, letter('v')),
        ZoneId::CtrlAltDel => chord(false, true, true, false, protocol::KEY_DELETE),
        ZoneId::Keyboard => Action::ToggleKeyboard,
        ZoneId::Settings => Action::OpenSettings,
        ZoneId::Shortcuts => Action::OpenShortcuts,
        ZoneId::Shift => Action::Latch(Modifier::Shift),
        ZoneId::Ctrl => Action::Latch(Modifier::Ctrl),
        ZoneId::Alt => Action::Latch(Modifier::Alt),
        ZoneId::DpiSlider => Action::DpiDrag,
        ZoneId::ScrollSlider => Action::ScrollDrag,
        ZoneId::StickLeft | ZoneId::StickRight => return None,
    })
}

/// One of the Vita's physical controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    LeftStick,
    RightStick,
    DPad,
    Cross,
    Circle,
    Triangle,
    Square,
    L,
    R,
    Select,
    Start,
    RearPanel,
    BottomCorners,
}

impl Control {
    pub const ALL: [Control; 13] = [
        Control::LeftStick,
        Control::RightStick,
        Control::DPad,
        Control::Cross,
        Control::Circle,
        Control::Triangle,
        Control::Square,
        Control::L,
        Control::R,
        Control::Select,
        Control::Start,
        Control::RearPanel,
        Control::BottomCorners,
    ];

    /// How it is written in the manual, in the player's own visual language.
    pub fn label(self) -> &'static str {
        match self {
            Control::LeftStick => "\u{25cf} Stick L",
            Control::RightStick => "\u{25cf} Stick R",
            Control::DPad => "\u{271a} Cruceta",
            Control::Cross => "\u{2715} Equis",
            Control::Circle => "\u{25cb} Circulo",
            Control::Triangle => "\u{25b3} Triangulo",
            Control::Square => "\u{25a1} Cuadrado",
            Control::L => "L",
            Control::R => "R",
            Control::Select => "SELECT",
            Control::Start => "START",
            Control::RearPanel => "Panel trasero",
            Control::BottomCorners => "Esquinas inf.",
        }
    }
}

/// What a control does in a given profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Forwarded to the title exactly as the hardware reports it.
    ToGame,
    /// Fine cursor movement.
    Cursor,
    /// Mouse wheel.
    Scroll,
    /// Arrow keys with hold-to-repeat.
    Arrows,
    /// A held mouse button, so click-and-drag works.
    HeldClick(MouseTarget),
    DoubleClick,
    /// Halves the pointer sensitivity while held.
    Precision,
    Key(KeyStroke),
    ToggleKeyboard,
    /// Rear panel as the pointer, with tap-to-click split down the middle.
    Pointer,
    /// Rear panel as the analog triggers.
    Triggers,
    /// Front-screen corners as L3/R3.
    StickClicks,
    /// Nothing; the control is idle in this profile.
    Idle,
}

impl Effect {
    /// One short line for the manual, in the player's language.
    pub fn describe(self) -> &'static str {
        match self {
            Effect::ToGame => "Al juego",
            Effect::Cursor => "Cursor fino",
            Effect::Scroll => "Scroll",
            Effect::Arrows => "Flechas",
            Effect::HeldClick(MouseTarget::Left) => "Clic izquierdo",
            Effect::HeldClick(MouseTarget::Right) => "Clic derecho",
            Effect::DoubleClick => "Doble clic",
            Effect::Precision => "Precision (1/2)",
            Effect::Key(_) => "Tecla",
            Effect::ToggleKeyboard => "Teclado",
            Effect::Pointer => "Raton + clic",
            Effect::Triggers => "L2 / R2 analogicos",
            Effect::StickClicks => "L3 / R3",
            Effect::Idle => "-",
        }
    }
}

/// One row of the switchboard.
#[derive(Debug, Clone, Copy)]
pub struct Binding {
    pub control: Control,
    pub profile: ControlProfile,
    pub effect: Effect,
}

const fn bind(control: Control, profile: ControlProfile, effect: Effect) -> Binding {
    Binding {
        control,
        profile,
        effect,
    }
}

/// The switchboard itself.
///
/// The game column is deliberately monotonous: every physical control forwards to the title, with
/// no exceptions. That monotony is the feature - it is what makes Death Stranding play the same
/// with the overlay on as with it off, and `the_game_profile_forwards_every_control` enforces it.
pub const BINDINGS: &[Binding] = &[
    // --- game profile: everything reaches the title ---
    bind(Control::LeftStick, ControlProfile::Game, Effect::ToGame),
    bind(Control::RightStick, ControlProfile::Game, Effect::ToGame),
    bind(Control::DPad, ControlProfile::Game, Effect::ToGame),
    bind(Control::Cross, ControlProfile::Game, Effect::ToGame),
    bind(Control::Circle, ControlProfile::Game, Effect::ToGame),
    bind(Control::Triangle, ControlProfile::Game, Effect::ToGame),
    bind(Control::Square, ControlProfile::Game, Effect::ToGame),
    bind(Control::L, ControlProfile::Game, Effect::ToGame),
    bind(Control::R, ControlProfile::Game, Effect::ToGame),
    bind(Control::Select, ControlProfile::Game, Effect::ToGame),
    bind(Control::Start, ControlProfile::Game, Effect::ToGame),
    bind(Control::RearPanel, ControlProfile::Game, Effect::Triggers),
    bind(
        Control::BottomCorners,
        ControlProfile::Game,
        Effect::StickClicks,
    ),
    // --- desktop profile: the pad becomes a mouse and keyboard ---
    bind(Control::LeftStick, ControlProfile::Desktop, Effect::Cursor),
    bind(Control::RightStick, ControlProfile::Desktop, Effect::Scroll),
    bind(Control::DPad, ControlProfile::Desktop, Effect::Arrows),
    bind(
        Control::Cross,
        ControlProfile::Desktop,
        Effect::HeldClick(MouseTarget::Left),
    ),
    bind(
        Control::Circle,
        ControlProfile::Desktop,
        Effect::HeldClick(MouseTarget::Right),
    ),
    bind(
        Control::Triangle,
        ControlProfile::Desktop,
        Effect::Key(protocol::KEY_ENTER),
    ),
    bind(
        Control::Square,
        ControlProfile::Desktop,
        Effect::Key(protocol::KEY_BACKSPACE),
    ),
    bind(Control::L, ControlProfile::Desktop, Effect::Precision),
    bind(Control::R, ControlProfile::Desktop, Effect::DoubleClick),
    bind(
        Control::Select,
        ControlProfile::Desktop,
        Effect::ToggleKeyboard,
    ),
    bind(
        Control::Start,
        ControlProfile::Desktop,
        Effect::Key(protocol::KEY_LEFT_WIN),
    ),
    bind(Control::RearPanel, ControlProfile::Desktop, Effect::Pointer),
    bind(
        Control::BottomCorners,
        ControlProfile::Desktop,
        Effect::Idle,
    ),
];

/// What `control` does in `profile`.
pub fn effect_of(control: Control, profile: ControlProfile) -> Effect {
    BINDINGS
        .iter()
        .find(|binding| binding.control == control && binding.profile == profile)
        .map(|binding| binding.effect)
        .unwrap_or(Effect::Idle)
}

/// The manual for one profile, as `(control, effect)` lines in table order.
pub fn manual(profile: ControlProfile) -> impl Iterator<Item = (&'static str, &'static str)> {
    BINDINGS
        .iter()
        .filter(move |binding| binding.profile == profile)
        .map(|binding| (binding.control.label(), binding.effect.describe()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::layout::ZONES;

    /// The clash test. Two rows for one control in one profile means the winner depends on
    /// iteration order, which is exactly the class of bug this table exists to prevent.
    #[test]
    fn no_control_has_two_meanings_in_one_profile() {
        for (index, a) in BINDINGS.iter().enumerate() {
            for b in &BINDINGS[index + 1..] {
                assert!(
                    !(a.control == b.control && a.profile == b.profile),
                    "{:?} is bound twice in profile {:?}",
                    a.control,
                    a.profile
                );
            }
        }
    }

    /// Every control must be accounted for in every profile - an unlisted control is a control
    /// that silently does nothing, which is how L2/R2 disappeared in 0.4.x.
    #[test]
    fn every_control_is_bound_in_every_profile() {
        for profile in ControlProfile::ALL {
            for control in Control::ALL {
                assert!(
                    BINDINGS
                        .iter()
                        .any(|b| b.control == control && b.profile == profile),
                    "{control:?} has no row in profile {profile:?}"
                );
            }
        }
    }

    /// The promise the game profile makes: nothing is intercepted. If a future edit repurposes a
    /// button here, this fails before it reaches a console.
    #[test]
    fn the_game_profile_forwards_every_control() {
        for control in Control::ALL {
            let effect = effect_of(control, ControlProfile::Game);
            let forwards = matches!(
                effect,
                Effect::ToGame | Effect::Triggers | Effect::StickClicks
            );
            assert!(
                forwards,
                "{control:?} does not reach the title in the game profile: {effect:?}"
            );
        }
    }

    /// Every zone in the layout must mean something, or it is a button that draws and does
    /// nothing. The stick corners are the deliberate exception.
    #[test]
    fn every_drawn_zone_has_an_action() {
        for zone in ZONES {
            let is_stick = matches!(zone.id, ZoneId::StickLeft | ZoneId::StickRight);
            assert_eq!(
                zone_action(zone.id).is_none(),
                is_stick,
                "{:?} has the wrong kind of action",
                zone.id
            );
        }
    }

    /// Ctrl+Shift+Esc was impossible before because chords carried no Shift. Pin it.
    #[test]
    fn chords_can_carry_shift() {
        let action = Action::Chord {
            shift: true,
            ctrl: true,
            alt: false,
            win: false,
            key: protocol::KEY_ESCAPE,
        };
        let Action::Chord { shift, ctrl, .. } = action else {
            panic!("built a chord");
        };
        assert!(shift && ctrl);
    }

    #[test]
    fn copy_and_paste_send_the_same_bytes_the_keyboard_would() {
        let Some(Action::Chord { key, ctrl, .. }) = zone_action(ZoneId::Copy) else {
            panic!("Copy is a chord");
        };
        assert!(ctrl);
        assert_eq!(key, protocol::key_for_char('c').unwrap());
    }

    #[test]
    fn alt_f4_is_alt_plus_f4_and_nothing_else() {
        let Some(Action::Chord {
            shift,
            ctrl,
            alt,
            win,
            key,
        }) = zone_action(ZoneId::AltF4)
        else {
            panic!("Alt+F4 is a chord");
        };
        assert_eq!((shift, ctrl, alt, win), (false, false, true, false));
        assert_eq!(key, protocol::KEY_F4);
    }

    #[test]
    fn the_manual_covers_every_control_and_never_invents_one() {
        for profile in ControlProfile::ALL {
            assert_eq!(manual(profile).count(), Control::ALL.len());
        }
    }
}
