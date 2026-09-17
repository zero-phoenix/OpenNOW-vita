//! The state of the hardware, as plain data.
//!
//! The binary fills these in from SDL; everything downstream works on them, which is what makes
//! the mapping testable without a console, an SDL context or a live peer connection.

/// Which of the Vita's two touch panels a contact is on.
///
/// VitaSDK's `SDL_vitatouch.c` registers them as SDL touch devices 1 and 2, in that order; the
/// binary translates those ids into this enum so nothing downstream has to know the numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Front,
    Rear,
}

/// One contact, in normalized 0..1 panel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Finger {
    pub id: i64,
    pub panel: Panel,
    pub x: f32,
    pub y: f32,
}

/// A snapshot of the physical controls, in Vita terms.
///
/// Face buttons are named after the Vita's own markings rather than SDL's A/B/X/Y, because the
/// mapping between the two is a source of off-by-one bugs: the registered controller mapping puts
/// Cross on `A`, Circle on `B`, Square on `X` and Triangle on `Y`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PadState {
    /// -1.0..1.0 each, already normalized from the raw axes. +Y is down, as SDL reports it.
    pub left_stick: (f32, f32),
    pub right_stick: (f32, f32),
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
    pub cross: bool,
    pub circle: bool,
    pub triangle: bool,
    pub square: bool,
    pub l1: bool,
    pub r1: bool,
    pub l3: bool,
    pub r3: bool,
    pub select: bool,
    pub start: bool,
    /// 0-255. Only ever non-zero on a Vita TV with a DualShock attached - the handheld has no
    /// physical L2/R2, which is the whole reason the rear panel stands in for them.
    pub l2: u8,
    pub r2: u8,
}

impl PadState {
    /// True when any face button, shoulder, d-pad direction or stick click is held. Used to
    /// decide whether a pad snapshot is worth sending ahead of the next tick.
    pub fn any_button(self) -> bool {
        self.dpad_up
            || self.dpad_down
            || self.dpad_left
            || self.dpad_right
            || self.cross
            || self.circle
            || self.triangle
            || self.square
            || self.l1
            || self.r1
            || self.l3
            || self.r3
            || self.select
            || self.start
            || self.l2 > 0
            || self.r2 > 0
    }
}

/// Applies a deadzone and rescales what is left back over the full 0..1 range, so the stick still
/// reaches full speed at the edge of its travel.
///
/// Only the desktop profile uses this. In the game profile the raw axis is forwarded untouched -
/// the title applies its own deadzone, and applying a second one on top is what makes a stick
/// feel dead near centre.
pub fn apply_deadzone(value: f32, deadzone: f32) -> f32 {
    if value.abs() <= deadzone {
        return 0.0;
    }
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    ((value.abs() - deadzone) / (1.0 - deadzone)).min(1.0) * sign
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_centred_stick_is_dead() {
        assert_eq!(apply_deadzone(0.0, 0.22), 0.0);
        assert_eq!(apply_deadzone(0.21, 0.22), 0.0);
        assert_eq!(apply_deadzone(-0.21, 0.22), 0.0);
    }

    /// The point of rescaling: full deflection must still mean full speed, not 78 % of it.
    #[test]
    fn full_deflection_still_reaches_full_speed() {
        assert!((apply_deadzone(1.0, 0.22) - 1.0).abs() < 1e-6);
        assert!((apply_deadzone(-1.0, 0.22) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn just_past_the_deadzone_is_barely_moving() {
        let value = apply_deadzone(0.23, 0.22);
        assert!(value > 0.0 && value < 0.02, "got {value}");
    }

    /// A worn Vita stick can report past 1.0 on the diagonals; that must not become >100 % speed.
    #[test]
    fn overshoot_is_clamped() {
        assert!((apply_deadzone(1.4, 0.22) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn an_idle_pad_has_no_buttons_held() {
        assert!(!PadState::default().any_button());
    }

    #[test]
    fn a_rear_trigger_counts_as_a_button() {
        let pad = PadState {
            l2: 200,
            ..Default::default()
        };
        assert!(pad.any_button());
    }
}
