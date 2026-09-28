//! The app's shared palette and the quality-graded colour scale.
//!
//! Moved verbatim out of `ui.rs` so the screens added for v0.7 (performance panel, pause menu)
//! stop picking their own greys in their own corners: surfaces, borders and status colours are
//! named here once. The values are the palette OpenNOW has always had, unchanged.

use egui::Color32;

/// GeForce NOW green. The only accent in the app.
pub const ACCENT: Color32 = Color32::from_rgb(0x76, 0xb9, 0x00);
/// Dimmer variant of the accent, for focus halos and pressed states.
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x1d, 0x2b, 0x10);

/// Surfaces, darkest to lightest.
pub const BG_DEEP: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
pub const BG_PANEL: Color32 = Color32::from_rgb(0x0a, 0x0a, 0x0a);
pub const BG_RAISED: Color32 = Color32::from_rgb(0x16, 0x16, 0x16);
pub const BORDER: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);

/// Text.
pub const TEXT_DIM: Color32 = Color32::from_rgb(0xa0, 0xa4, 0xac);

/// Status colours that predate the Grade scale; kept as-is for their existing call sites.
pub const DANGER: Color32 = Color32::from_rgb(0xff, 0x6b, 0x6b);
pub const WARNING: Color32 = Color32::from_rgb(0xff, 0xc1, 0x07);

/// How a displayed value is judged. The grade picks the colour, not the call site, so a metric
/// judged "good" is the same green in the stats panel, in the pause menu and anywhere else that
/// reports a value (Halyard's `ui::theme::Grade`, re-graded onto the OpenNOW palette).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    Good,
    Warn,
    Bad,
    Neutral,
}

/// The colour for a grade: green/amber/red from the existing status palette, dim text for
/// values that carry no judgement.
pub fn for_grade(grade: Grade) -> Color32 {
    match grade {
        Grade::Good => Color32::from_rgb(0x4c, 0xd9, 0x64),
        Grade::Warn => Color32::from_rgb(0xe8, 0xc1, 0x3a),
        Grade::Bad => DANGER,
        Grade::Neutral => TEXT_DIM,
    }
}

#[cfg(test)]
mod tests {
    use super::{Grade, for_grade};

    #[test]
    fn grades_are_distinct_and_stable() {
        let colours = [Grade::Good, Grade::Warn, Grade::Bad, Grade::Neutral].map(for_grade);
        for i in 0..colours.len() {
            for j in (i + 1)..colours.len() {
                assert_ne!(colours[i], colours[j], "grades {i} and {j} share a colour");
            }
        }
    }
}
