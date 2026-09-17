//! The one table of on-screen zones. Both the renderer and the hit-test read it.
//!
//! Layout rationale: the Vita's panel is 960x544 and the game needs all of it, so every control
//! lives on an edge where a thumb already rests and the middle is left completely clear. The
//! v0.5.0 layout got that right for the desktop profile but then drew a control manual across the
//! centre of the screen in *both* profiles, which put a text card over the picture the whole time
//! you were playing. The manual now lives in Settings, where a reference belongs.
//!
//! Coordinates are normalized 0..1 so they survive any future change of resolution.

use crate::config::{ControlProfile, InputConfig};

/// A normalized rectangle. `x1`/`y1` are exclusive, so adjacent cells sharing an edge do not both
/// claim the boundary pixel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Rect {
    pub const fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }

    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }
}

/// Every addressable zone. The same id can appear in the table more than once with a different
/// rectangle - `Esc` sits in a four-cell strip in the game profile and an eight-cell strip in the
/// desktop one - because what a zone *does* and where it *is* are separate questions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneId {
    /// Top-right, drawn in every profile whether or not anything else is: shows/hides the rest.
    /// It is the way back, so it can never be covered or switched off.
    Eye,
    /// Directly under the eye: swaps game/desktop. Live in both profiles while revealed, so the
    /// player can never end up locked in one of them.
    ModeToggle,

    // --- keys, shared between the two strips ---
    Esc,
    Enter,
    Keyboard,
    AltF4,
    Tab,
    Win,
    AltTab,
    Copy,
    Paste,
    Settings,
    Shift,
    Ctrl,
    Alt,
    Backspace,
    Delete,
    /// Opens the Windows-shortcut page of the on-screen keyboard.
    Shortcuts,
    CtrlAltDel,

    // --- slider rails ---
    DpiSlider,
    ScrollSlider,

    // --- front-screen stand-ins for the stick clicks the Vita has no buttons for ---
    StickLeft,
    StickRight,
}

/// When a zone is live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Live {
    /// Whenever the overlay is enabled at all, revealed or not.
    Always,
    /// While revealed, in either profile.
    Revealed,
    /// While revealed, game profile only.
    Game,
    /// While revealed, desktop profile only.
    Desktop,
    /// Game profile, and only while the stick-zone preference is on. Independent of `revealed`:
    /// L3/R3 are pad buttons, not overlay controls, and hiding the overlay must not disable them.
    GameSticks,
}

/// One row of the layout table.
#[derive(Debug, Clone, Copy)]
pub struct Zone {
    pub id: ZoneId,
    /// Short label for the renderer. Empty for zones that are drawn some other way.
    pub label: &'static str,
    pub rect: Rect,
    pub live: Live,
}

/// Height of the game profile's minimal strip, as a fraction of the 544 px panel (~44 px).
const GAME_STRIP_H: f32 = 0.08;
/// Height of the desktop profile's two key strips (~60 px).
const DESK_STRIP_H: f32 = 0.11;
/// Left edge of the always-on eye. Both top strips stop here so nothing can overlap it.
const EYE_LEFT: f32 = 0.88;
/// Width of the slider rails reaching in from the left and right edges.
const RAIL_W: f32 = 0.07;
const RAIL_TOP: f32 = 0.30;
const RAIL_BOTTOM: f32 = 0.72;
/// Top of the L3/R3 corners. v0.5.0 had this at 0.66, giving a third of the screen to two stick
/// clicks; 0.80 is still a comfortable thumb target and hands ~87 px of picture back.
const STICK_TOP: f32 = 0.80;
const STICK_W: f32 = 0.25;

/// `index`-th of `count` cells spanning `x0..x1`.
const fn cell(index: usize, count: usize, x0: f32, x1: f32, y0: f32, y1: f32) -> Rect {
    let width = (x1 - x0) / count as f32;
    Rect::new(x0 + width * index as f32, y0, x0 + width * (index + 1) as f32, y1)
}

const fn game_cell(index: usize) -> Rect {
    cell(index, 4, 0.0, EYE_LEFT, 0.0, GAME_STRIP_H)
}

const fn top_cell(index: usize) -> Rect {
    cell(index, 8, 0.0, EYE_LEFT, 0.0, DESK_STRIP_H)
}

const fn bottom_cell(index: usize) -> Rect {
    cell(index, 8, 0.0, 1.0, 1.0 - DESK_STRIP_H, 1.0)
}

/// The whole layout, in one place.
pub const ZONES: &[Zone] = &[
    // The eye first: it is checked before everything else and drawn last, so it is never covered.
    Zone {
        id: ZoneId::Eye,
        label: "\u{1f441}",
        rect: Rect::new(EYE_LEFT, 0.0, 1.0, DESK_STRIP_H),
        live: Live::Always,
    },
    Zone {
        id: ZoneId::ModeToggle,
        label: "",
        rect: Rect::new(EYE_LEFT, DESK_STRIP_H, 1.0, DESK_STRIP_H * 2.0),
        live: Live::Revealed,
    },
    // --- game profile: four keys along the top, nothing else over the picture ---
    Zone {
        id: ZoneId::Esc,
        label: "ESC",
        rect: game_cell(0),
        live: Live::Game,
    },
    Zone {
        id: ZoneId::Enter,
        label: "\u{23ce}",
        rect: game_cell(1),
        live: Live::Game,
    },
    Zone {
        id: ZoneId::Keyboard,
        label: "\u{2328}",
        rect: game_cell(2),
        live: Live::Game,
    },
    Zone {
        id: ZoneId::AltF4,
        label: "ALT F4",
        rect: game_cell(3),
        live: Live::Game,
    },
    // --- desktop profile: top strip ---
    Zone {
        id: ZoneId::Esc,
        label: "ESC",
        rect: top_cell(0),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Tab,
        label: "TAB",
        rect: top_cell(1),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Win,
        label: "\u{229e}",
        rect: top_cell(2),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::AltTab,
        label: "ALT\u{21b9}",
        rect: top_cell(3),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Copy,
        label: "COPY",
        rect: top_cell(4),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Paste,
        label: "PEGAR",
        rect: top_cell(5),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Keyboard,
        label: "\u{2328}",
        rect: top_cell(6),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Settings,
        label: "\u{2699}",
        rect: top_cell(7),
        live: Live::Desktop,
    },
    // --- desktop profile: bottom strip ---
    Zone {
        id: ZoneId::Shift,
        label: "SHIFT",
        rect: bottom_cell(0),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Ctrl,
        label: "CTRL",
        rect: bottom_cell(1),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Alt,
        label: "ALT",
        rect: bottom_cell(2),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Enter,
        label: "\u{23ce}",
        rect: bottom_cell(3),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Backspace,
        label: "\u{232b}",
        rect: bottom_cell(4),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Delete,
        label: "SUPR",
        rect: bottom_cell(5),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::Shortcuts,
        label: "ATAJOS",
        rect: bottom_cell(6),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::CtrlAltDel,
        label: "C-A-DEL",
        rect: bottom_cell(7),
        live: Live::Desktop,
    },
    // --- desktop profile: slider rails ---
    Zone {
        id: ZoneId::DpiSlider,
        label: "DPI",
        rect: Rect::new(0.0, RAIL_TOP, RAIL_W, RAIL_BOTTOM),
        live: Live::Desktop,
    },
    Zone {
        id: ZoneId::ScrollSlider,
        label: "\u{2195}",
        rect: Rect::new(1.0 - RAIL_W, RAIL_TOP, 1.0, RAIL_BOTTOM),
        live: Live::Desktop,
    },
    // --- game profile: stick clicks ---
    Zone {
        id: ZoneId::StickLeft,
        label: "L3",
        rect: Rect::new(0.0, STICK_TOP, STICK_W, 1.0),
        live: Live::GameSticks,
    },
    Zone {
        id: ZoneId::StickRight,
        label: "R3",
        rect: Rect::new(1.0 - STICK_W, STICK_TOP, 1.0, 1.0),
        live: Live::GameSticks,
    },
];

impl Live {
    /// Whether a zone with this liveness responds under the given settings.
    pub fn is_live(self, config: InputConfig) -> bool {
        if !config.overlay_enabled {
            return false;
        }
        let game = config.profile == ControlProfile::Game;
        match self {
            Live::Always => true,
            Live::Revealed => config.overlay_revealed,
            Live::Game => game && config.overlay_revealed,
            Live::Desktop => !game && config.overlay_revealed,
            // Independent of `revealed` on purpose: L3 and R3 are pad buttons standing on the
            // screen, not overlay controls. Hiding the overlay to see the picture must not take
            // two of the title's buttons away with it.
            Live::GameSticks => game && config.stick_zones_active,
        }
    }
}

/// The zones that are live right now, in table order.
pub fn live_zones(config: InputConfig) -> impl Iterator<Item = &'static Zone> {
    ZONES.iter().filter(move |zone| zone.live.is_live(config))
}

/// The rectangle a zone occupies in a given profile, for callers that need to draw one on its own
/// - the controls explainer, for instance, which shows L3/R3 on a diagram of the console.
pub fn zone_rect(id: ZoneId, config: InputConfig) -> Option<Rect> {
    live_zones(config)
        .find(|zone| zone.id == id)
        .map(|zone| zone.rect)
}

/// The zone a normalized front-touch lands in, or `None` for the clear middle of the screen.
///
/// Table order is the precedence order, and the eye is the first row - so it wins over anything
/// that might ever be placed under it.
pub fn zone_at(x: f32, y: f32, config: InputConfig) -> Option<&'static Zone> {
    live_zones(config).find(|zone| zone.rect.contains(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ControlProfile;

    fn game() -> InputConfig {
        InputConfig::default()
    }

    fn desktop() -> InputConfig {
        InputConfig {
            profile: ControlProfile::Desktop,
            ..InputConfig::default()
        }
    }

    /// The bug class this whole table exists to kill: two live zones fighting over the same pixel.
    #[test]
    fn no_two_live_zones_overlap_in_the_same_profile() {
        for config in [game(), desktop()] {
            let live: Vec<_> = live_zones(config).collect();
            for (index, a) in live.iter().enumerate() {
                for b in &live[index + 1..] {
                    assert!(
                        !a.rect.overlaps(&b.rect),
                        "{:?} and {:?} overlap in profile {:?}",
                        a.id,
                        b.id,
                        config.profile
                    );
                }
            }
        }
    }

    /// The eye is the way back. If anything could ever be drawn over it, a player who hides the
    /// overlay in the game profile would have no way to reach settings again.
    #[test]
    fn nothing_can_cover_the_eye() {
        let eye = ZONES
            .iter()
            .find(|zone| zone.id == ZoneId::Eye)
            .expect("the eye is in the table");
        for config in [game(), desktop()] {
            for zone in live_zones(config) {
                if zone.id == ZoneId::Eye {
                    continue;
                }
                assert!(
                    !zone.rect.overlaps(&eye.rect),
                    "{:?} overlaps the eye in profile {:?}",
                    zone.id,
                    config.profile
                );
            }
        }
    }

    /// Every zone that is drawn must be reachable, and every reachable point must belong to the
    /// zone that was drawn there. Testing the centre of each rect proves both directions at once.
    #[test]
    fn every_drawn_zone_answers_at_its_own_centre() {
        for config in [game(), desktop()] {
            for zone in live_zones(config) {
                let (x, y) = (
                    (zone.rect.x0 + zone.rect.x1) / 2.0,
                    (zone.rect.y0 + zone.rect.y1) / 2.0,
                );
                let hit = zone_at(x, y, config).expect("a drawn zone must be hittable");
                assert_eq!(hit.id, zone.id, "centre of {:?} resolved to {:?}", zone.id, hit.id);
            }
        }
    }

    /// The whole point of the redesign: the middle of the screen is the game's, in both profiles.
    #[test]
    fn the_middle_of_the_screen_belongs_to_nobody() {
        for config in [game(), desktop()] {
            for (x, y) in [(0.5, 0.5), (0.3, 0.45), (0.7, 0.6), (0.5, 0.25)] {
                assert!(
                    zone_at(x, y, config).is_none(),
                    "({x}, {y}) was claimed in profile {:?}",
                    config.profile
                );
            }
        }
    }

    /// In the game profile the picture must be clear apart from the thin top strip and the two
    /// bottom corners. Anything else over the middle band is a regression.
    #[test]
    fn the_game_profile_leaves_the_picture_alone() {
        for zone in live_zones(game()) {
            let touches_middle = zone.rect.y0 < STICK_TOP && zone.rect.y1 > DESK_STRIP_H * 2.0;
            assert!(
                !touches_middle,
                "{:?} reaches into the middle of the picture in the game profile",
                zone.id
            );
        }
    }

    /// Hiding the overlay must not silently disable two of the title's buttons.
    #[test]
    fn hiding_the_overlay_keeps_the_stick_corners_alive() {
        let hidden = InputConfig {
            overlay_revealed: false,
            ..game()
        };
        assert!(zone_at(0.1, 0.9, hidden).is_some_and(|z| z.id == ZoneId::StickLeft));
        assert!(zone_at(0.9, 0.9, hidden).is_some_and(|z| z.id == ZoneId::StickRight));
        // ...but the key strip is gone, which is what "hidden" means.
        assert!(zone_at(0.1, 0.04, hidden).is_none());
        // ...and the eye still answers, or there would be no way back.
        assert!(zone_at(0.94, 0.05, hidden).is_some_and(|z| z.id == ZoneId::Eye));
    }

    #[test]
    fn turning_the_overlay_off_leaves_nothing_live() {
        let off = InputConfig {
            overlay_enabled: false,
            ..desktop()
        };
        assert_eq!(live_zones(off).count(), 0);
    }

    #[test]
    fn stick_corners_are_absent_in_the_desktop_profile() {
        assert!(!live_zones(desktop()).any(|zone| zone.id == ZoneId::StickLeft));
    }

    /// Cells must tile their strip exactly - no seam a finger can fall through.
    #[test]
    fn strip_cells_tile_without_gaps() {
        let bottom: Vec<_> = live_zones(desktop())
            .filter(|zone| zone.rect.y0 > 0.8 && zone.rect.y1 >= 1.0)
            .collect();
        assert_eq!(bottom.len(), 8, "the bottom strip has eight cells");
        for pair in bottom.windows(2) {
            assert!(
                (pair[0].rect.x1 - pair[1].rect.x0).abs() < 1e-6,
                "gap between {:?} and {:?}",
                pair[0].id,
                pair[1].id
            );
        }
        assert!((bottom[0].rect.x0).abs() < 1e-6, "the strip starts at the left edge");
        assert!(
            (bottom[7].rect.x1 - 1.0).abs() < 1e-6,
            "the strip reaches the right edge"
        );
    }

    #[test]
    fn a_rect_excludes_its_far_edge() {
        let rect = Rect::new(0.0, 0.0, 0.5, 0.5);
        assert!(rect.contains(0.0, 0.0));
        assert!(rect.contains(0.49, 0.49));
        assert!(!rect.contains(0.5, 0.25), "x1 is exclusive");
        assert!(!rect.contains(0.25, 0.5), "y1 is exclusive");
    }
}
