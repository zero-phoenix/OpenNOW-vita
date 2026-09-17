//! Who owns this finger.
//!
//! Ownership used to be a 25-line `if`/`else if` chain inside the 60 Hz event loop, and the order
//! of its branches was something you had to deduce by reading. That is how v0.5.0 shipped with the
//! front screen still driving the host cursor in the desktop profile: the trackpad branch was
//! evaluated *before* the desktop-profile branch, and the trackpad preference defaults to on, so
//! the "the middle of the screen is deliberately dead space" branch below it was unreachable. The
//! comment describing that dead space was accurate about the intent and wrong about the behaviour.
//!
//! Here the order is a written list, and [`tests`] pins every step of it.

use super::layout::{ZoneId, zone_at};
use super::physical::Panel;
use crate::config::{ControlProfile, InputConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickSide {
    Left,
    Right,
}

/// What a contact drives. Decided once, on finger-down, and the rest of the gesture follows it -
/// deciding per event would let a drag that starts on the game and ends on a button swallow the
/// release, leaving the host holding a key down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchOwner {
    /// The client's own interface: menus, the on-screen keyboard, modal dialogs.
    ClientUi,
    /// An overlay zone - a key, the eye, the profile switch or a slider rail.
    Zone(ZoneId),
    /// A front-screen stand-in for L3/R3.
    StickZone(StickSide),
    /// Front-screen trackpad driving the host cursor.
    Trackpad,
    /// Rear panel driving the host cursor.
    RearPointer,
    /// Rear panel standing in for the analog triggers.
    RearTrigger,
    /// Nobody. The contact is ignored.
    None,
}

/// Resolves a contact to its owner.
///
/// `ui_claims` is whether the client's own interface has a widget under this point; the binary
/// works that out from egui's rectangles, which is the one thing here that cannot be pure.
///
/// Precedence, in order:
///
/// 1. The client's own interface
/// 2. Overlay zones, including the eye and the stick corners
/// 3. The front-screen trackpad - **game profile only**
/// 4. Nobody
pub fn route_touch(
    panel: Panel,
    x: f32,
    y: f32,
    config: InputConfig,
    ui_claims: bool,
) -> TouchOwner {
    if panel == Panel::Rear {
        return if config.desktop_active() {
            TouchOwner::RearPointer
        } else {
            TouchOwner::RearTrigger
        };
    }

    if ui_claims {
        return TouchOwner::ClientUi;
    }

    if let Some(zone) = zone_at(x, y, config) {
        return match zone.id {
            ZoneId::StickLeft => TouchOwner::StickZone(StickSide::Left),
            ZoneId::StickRight => TouchOwner::StickZone(StickSide::Right),
            other => TouchOwner::Zone(other),
        };
    }

    // The fix. In the desktop profile the rear panel is the pointer, so the clear middle of the
    // front screen is genuinely dead: it neither moves the cursor nor reaches the title. Consulting
    // `front_trackpad` here, inside the game-profile arm, is what makes that true rather than
    // aspirational.
    if config.profile == ControlProfile::Game && config.front_trackpad {
        return TouchOwner::Trackpad;
    }

    TouchOwner::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RearTouchMode;

    fn game() -> InputConfig {
        InputConfig::default()
    }

    fn desktop() -> InputConfig {
        InputConfig {
            profile: ControlProfile::Desktop,
            ..InputConfig::default()
        }
    }

    fn front(x: f32, y: f32, config: InputConfig) -> TouchOwner {
        route_touch(Panel::Front, x, y, config, false)
    }

    /// The regression test for the bug the user reported as "solo puedo usar el mouse con la
    /// pantalla delantera pero yo te dije la trasera". With the trackpad preference at its default
    /// (on), the middle of the front screen must still do nothing in the desktop profile.
    #[test]
    fn the_front_screen_does_not_drive_the_cursor_in_the_desktop_profile() {
        let config = desktop();
        assert!(config.front_trackpad, "the preference defaults to on");
        assert_eq!(front(0.5, 0.5, config), TouchOwner::None);
        assert_eq!(front(0.3, 0.45, config), TouchOwner::None);
        assert_eq!(front(0.6, 0.6, config), TouchOwner::None);
    }

    /// ...while in the game profile it still works, because there the rear panel is busy being
    /// L2/R2 and the front screen is the only pointer left.
    #[test]
    fn the_front_screen_is_still_a_trackpad_in_the_game_profile() {
        assert_eq!(front(0.5, 0.5, game()), TouchOwner::Trackpad);
    }

    #[test]
    fn turning_the_trackpad_off_leaves_the_front_screen_dead() {
        let config = InputConfig {
            front_trackpad: false,
            ..game()
        };
        assert_eq!(front(0.5, 0.5, config), TouchOwner::None);
    }

    #[test]
    fn the_rear_panel_is_the_pointer_in_desktop_and_the_triggers_in_game() {
        assert_eq!(
            route_touch(Panel::Rear, 0.2, 0.5, desktop(), false),
            TouchOwner::RearPointer
        );
        assert_eq!(
            route_touch(Panel::Rear, 0.2, 0.5, game(), false),
            TouchOwner::RearTrigger
        );
    }

    /// A rear contact belongs to the pad or the pointer no matter what the front screen is doing,
    /// including while a modal has the front screen.
    #[test]
    fn the_rear_panel_ignores_the_client_interface() {
        assert_eq!(
            route_touch(Panel::Rear, 0.5, 0.5, game(), true),
            TouchOwner::RearTrigger
        );
    }

    #[test]
    fn the_client_interface_wins_over_the_overlay() {
        assert_eq!(
            route_touch(Panel::Front, 0.94, 0.05, desktop(), true),
            TouchOwner::ClientUi
        );
    }

    #[test]
    fn the_eye_answers_in_both_profiles_even_when_collapsed() {
        for profile in ControlProfile::ALL {
            let config = InputConfig {
                profile,
                overlay_revealed: false,
                ..InputConfig::default()
            };
            assert_eq!(front(0.94, 0.05, config), TouchOwner::Zone(ZoneId::Eye));
        }
    }

    #[test]
    fn the_bottom_corners_are_stick_clicks_in_the_game_profile() {
        assert_eq!(front(0.1, 0.9, game()), TouchOwner::StickZone(StickSide::Left));
        assert_eq!(front(0.9, 0.9, game()), TouchOwner::StickZone(StickSide::Right));
    }

    /// In the desktop profile the same corners are the ends of the key strip instead, and the
    /// stick clicks are gone - the two are mutually exclusive by profile, not by geometry.
    #[test]
    fn the_bottom_corners_are_keys_in_the_desktop_profile() {
        assert_eq!(front(0.05, 0.95, desktop()), TouchOwner::Zone(ZoneId::Shift));
        assert_eq!(
            front(0.95, 0.95, desktop()),
            TouchOwner::Zone(ZoneId::CtrlAltDel)
        );
    }

    #[test]
    fn switching_the_stick_zones_off_hands_the_corners_back() {
        let config = InputConfig {
            stick_zones_active: false,
            ..game()
        };
        assert_eq!(front(0.1, 0.9, config), TouchOwner::Trackpad);
    }

    #[test]
    fn the_game_strip_sends_keys_without_leaving_the_profile() {
        assert_eq!(front(0.05, 0.04, game()), TouchOwner::Zone(ZoneId::Esc));
        assert_eq!(front(0.80, 0.04, game()), TouchOwner::Zone(ZoneId::AltF4));
    }

    /// With the overlay collapsed the strip is gone, so a thumb resting up there reaches the
    /// trackpad rather than firing Escape into the game.
    #[test]
    fn collapsing_the_overlay_frees_the_top_of_the_screen() {
        let config = InputConfig {
            overlay_revealed: false,
            ..game()
        };
        assert_eq!(front(0.05, 0.04, config), TouchOwner::Trackpad);
    }

    /// Settings unrelated to touch must not change routing. Cheap to assert, and it catches a
    /// whole family of "I changed X and Y broke" reports.
    #[test]
    fn unrelated_settings_do_not_change_routing() {
        let base = desktop();
        for config in [
            InputConfig {
                trigger_swap: true,
                ..base
            },
            InputConfig {
                rear_touch_mode: RearTouchMode::Quadrant,
                ..base
            },
            InputConfig {
                sensitivity_percent: 250,
                ..base
            },
            InputConfig {
                trigger_pressure: 128,
                ..base
            },
        ] {
            assert_eq!(front(0.5, 0.5, config), TouchOwner::None);
            assert_eq!(front(0.94, 0.05, config), TouchOwner::Zone(ZoneId::Eye));
        }
    }
}
