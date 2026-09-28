//! Stream-screen widgets and overlays: icons, toolbar, PC overlay, mode pill and toast,
//! on-screen keyboard, stream modals, and the streaming screen itself. Split out of `ui.rs`
//! (v0.7 F5) without behavioural change - moved, not rewritten.

use super::catalog_ui::LAUNCH_MODAL_WIDTH;
use super::hud;
use super::settings_ui::settings_row;
use super::ui::{
    allocate_device_image, battery_color, paint_battery, text1, uv_subrect, vita_back,
    vita_front, FRONT_SCREEN_X, FRONT_SCREEN_Y, REAR_PAD_X, REAR_PAD_Y,
};
use super::theme::{ACCENT, ACCENT_DIM, BG_DEEP, BG_PANEL, BG_RAISED, BORDER, DANGER, TEXT_DIM, WARNING};
use crate::gfn::catalog::GameSummary;
use crate::gfn::stream_prefs::ControlProfile;
use crate::i18n::I18n;
use crate::input::AppCommand;

pub(crate) enum StreamIcon {
    Keyboard,
    Stats,
    Power,
    Mouse,
    Collapse,
    Expand,
    Controls,
    Clock,
    Globe,
    Menu,
    Monitor,
    Person,
    Signal,
    Check,
    ChevronDown,
}

pub(crate) fn paint_stream_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: StreamIcon,
    tint: egui::Color32,
) {
    match icon {
        StreamIcon::Keyboard => {
            let stroke = egui::Stroke::new(1.0_f32, tint);
            painter.rect_stroke(rect, 2u8, stroke, egui::StrokeKind::Inside);

            // Two rows of keys plus a spacebar, which reads as a keyboard at this size where
            // anything more detailed turns to mush.
            let inset = rect.shrink2(egui::vec2(2.5, 3.0));
            let key = egui::vec2(inset.width() / 5.5, 1.5);
            for row in 0..2 {
                let y = inset.min.y + row as f32 * 3.0;
                for column in 0..4 {
                    let x = inset.min.x + column as f32 * (key.x + 1.0);
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(x, y), key),
                        0.5,
                        tint,
                    );
                }
            }
            let bar_y = inset.min.y + 6.0;
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(inset.min.x + key.x, bar_y),
                    egui::vec2(inset.width() - key.x * 2.0, 1.5),
                ),
                0.5,
                tint,
            );
        }
        // Three rising bars - a chart, for the counters.
        StreamIcon::Stats => {
            let inset = rect.shrink(2.0);
            let bar_width = inset.width() / 5.0;
            for (index, height_fraction) in [0.45_f32, 0.75, 1.0].into_iter().enumerate() {
                let height = inset.height() * height_fraction;
                let x = inset.min.x + index as f32 * bar_width * 1.8;
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(x, inset.max.y - height),
                        egui::vec2(bar_width, height),
                    ),
                    0.5,
                    tint,
                );
            }
        }
        StreamIcon::Power => {
            let c = rect.center();
            let r = rect.width().min(rect.height()) * 0.40_f32;
            painter.circle_stroke(c, r, egui::Stroke::new(1.5_f32, tint));
            painter.line_segment(
                [
                    egui::pos2(c.x, c.y - r * 1.15_f32),
                    egui::pos2(c.x, c.y - r * 0.15_f32),
                ],
                egui::Stroke::new(2.0_f32, tint),
            );
        }
        StreamIcon::Mouse => {
            let s = egui::Stroke::new(1.5_f32, tint);
            let tl = rect.min + egui::vec2(2.0, 1.0);
            let bot = tl + egui::vec2(0.0, rect.height() - 3.0);
            let rt = tl + egui::vec2(rect.width() * 0.55, (rect.height() - 3.0) * 0.65);
            painter.line_segment([tl, bot], s);
            painter.line_segment([tl, rt], s);
            painter.line_segment([bot, rt], s);
        }
        StreamIcon::Collapse => {
            let s = egui::Stroke::new(2.0_f32, tint);
            let (cx, cy) = (rect.center().x, rect.center().y);
            let (dx, dy) = (rect.width() * 0.22, rect.height() * 0.32);
            painter.line_segment([egui::pos2(cx + dx, cy - dy), egui::pos2(cx - dx, cy)], s);
            painter.line_segment([egui::pos2(cx - dx, cy), egui::pos2(cx + dx, cy + dy)], s);
        }
        StreamIcon::Expand => {
            let s = egui::Stroke::new(2.0_f32, tint);
            let (cx, cy) = (rect.center().x, rect.center().y);
            let (dx, dy) = (rect.width() * 0.22, rect.height() * 0.32);
            painter.line_segment([egui::pos2(cx - dx, cy - dy), egui::pos2(cx + dx, cy)], s);
            painter.line_segment([egui::pos2(cx + dx, cy), egui::pos2(cx - dx, cy + dy)], s);
        }
        StreamIcon::Menu => {
            // Hamburger: three bars wide enough to read as "menu" at 16 px, thick enough to
            // survive the toolbar's translucent plate.
            let s = egui::Stroke::new(1.8_f32, tint);
            let (cx, cy) = (rect.center().x, rect.center().y);
            let (dx, dy) = (rect.width() * 0.26, rect.height() * 0.22);
            for row in -1..=1 {
                let y = cy + row as f32 * dy;
                painter.line_segment([egui::pos2(cx - dx, y), egui::pos2(cx + dx, y)], s);
            }
        }
        StreamIcon::Controls => {
            // Gamepad icon: outer rounded rectangle body with d-pad cross and action buttons
            let stroke = egui::Stroke::new(1.2_f32, tint);
            let inset = rect.shrink2(egui::vec2(1.0, 2.5));
            painter.rect_stroke(inset, 3.0, stroke, egui::StrokeKind::Inside);

            // D-Pad cross on left
            let dpad_cx = inset.min.x + inset.width() * 0.3;
            let dpad_cy = inset.center().y;
            let arm = 2.5;
            painter.line_segment(
                [
                    egui::pos2(dpad_cx - arm, dpad_cy),
                    egui::pos2(dpad_cx + arm, dpad_cy),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(dpad_cx, dpad_cy - arm),
                    egui::pos2(dpad_cx, dpad_cy + arm),
                ],
                stroke,
            );

            // Two action buttons on right
            let btn_cx = inset.min.x + inset.width() * 0.7;
            painter.circle_filled(egui::pos2(btn_cx - 1.8, dpad_cy + 1.2), 1.0, tint);
            painter.circle_filled(egui::pos2(btn_cx + 1.8, dpad_cy - 1.2), 1.0, tint);
        }
        StreamIcon::Clock => {
            let stroke = egui::Stroke::new(1.2_f32, tint);
            let center = rect.center();
            let radius = rect.width().min(rect.height()) * 0.45;
            painter.circle_stroke(center, radius, stroke);

            let cx = center.x;
            let cy = center.y;
            painter.line_segment(
                [center, egui::pos2(cx + radius * 0.4, cy - radius * 0.5)],
                stroke,
            );
            painter.line_segment([center, egui::pos2(cx - radius * 0.5, cy)], stroke);
        }
        StreamIcon::Globe => {
            let stroke = egui::Stroke::new(1.2_f32, tint);
            let center = rect.center();
            let radius = rect.width().min(rect.height()) * 0.42;
            painter.circle_stroke(center, radius, stroke);
            let meridian: Vec<egui::Pos2> = (0..=8)
                .map(|step| {
                    let t = step as f32 / 8.0 * std::f32::consts::PI - std::f32::consts::FRAC_PI_2;
                    egui::pos2(
                        center.x + radius * 0.42 * t.sin(),
                        center.y - radius * t.cos(),
                    )
                })
                .collect();
            painter.line(meridian, stroke);
            painter.line_segment(
                [
                    egui::pos2(center.x - radius, center.y),
                    egui::pos2(center.x + radius, center.y),
                ],
                stroke,
            );
        }
        StreamIcon::Monitor => {
            let stroke = egui::Stroke::new(1.2_f32, tint);
            let inset = rect.shrink2(egui::vec2(1.0, 3.0));
            let screen = egui::Rect::from_min_size(
                inset.min,
                egui::vec2(inset.width(), inset.height() * 0.75),
            );
            painter.rect_stroke(screen, 1.5, stroke, egui::StrokeKind::Inside);
            let stand_top = screen.max.y;
            let cx = inset.center().x;
            painter.line_segment(
                [egui::pos2(cx, stand_top), egui::pos2(cx, inset.max.y)],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(cx - inset.width() * 0.22, inset.max.y),
                    egui::pos2(cx + inset.width() * 0.22, inset.max.y),
                ],
                stroke,
            );
        }
        StreamIcon::Person => {
            let stroke = egui::Stroke::new(1.2_f32, tint);
            let center = rect.center();
            let head_r = rect.height() * 0.16;
            let head_c = egui::pos2(center.x, rect.min.y + rect.height() * 0.32);
            painter.circle_stroke(head_c, head_r, stroke);
            let shoulders = egui::Rect::from_center_size(
                egui::pos2(center.x, rect.max.y - rect.height() * 0.10),
                egui::vec2(rect.width() * 0.62, rect.height() * 0.38),
            );
            painter.rect_stroke(
                shoulders,
                egui::CornerRadius {
                    nw: (shoulders.width() * 0.5) as u8,
                    ne: (shoulders.width() * 0.5) as u8,
                    sw: 0,
                    se: 0,
                },
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        StreamIcon::Signal => {
            let inset = rect.shrink(2.0);
            let bar_width = inset.width() / 5.0;
            for (index, height_fraction) in [0.35_f32, 0.62, 0.85, 1.0].into_iter().enumerate() {
                let height = inset.height() * height_fraction;
                let x = inset.min.x + index as f32 * bar_width * 1.3;
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(x, inset.max.y - height),
                        egui::vec2(bar_width * 0.7, height),
                    ),
                    0.5,
                    tint,
                );
            }
        }
        StreamIcon::Check => {
            let stroke = egui::Stroke::new(1.8_f32, tint);
            let c = rect.center();
            let dx = rect.width() * 0.22;
            let dy = rect.height() * 0.22;
            painter.line_segment(
                [
                    egui::pos2(c.x - dx, c.y),
                    egui::pos2(c.x - dx * 0.15, c.y + dy),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x - dx * 0.15, c.y + dy),
                    egui::pos2(c.x + dx, c.y - dy),
                ],
                stroke,
            );
        }
        StreamIcon::ChevronDown => {
            let stroke = egui::Stroke::new(1.6_f32, tint);
            let c = rect.center();
            let dx = rect.width() * 0.24;
            let dy = rect.height() * 0.16;
            painter.line_segment(
                [egui::pos2(c.x - dx, c.y - dy), egui::pos2(c.x, c.y + dy)],
                stroke,
            );
            painter.line_segment(
                [egui::pos2(c.x, c.y + dy), egui::pos2(c.x + dx, c.y - dy)],
                stroke,
            );
        }
    }
}

/// A heart, drawn rather than typed: the bundled font has no heart glyph, exactly as it had no
/// multiplication-X, and a tofu box is worse than no icon at all.
pub(crate) fn paint_heart(painter: &egui::Painter, rect: egui::Rect, filled: bool, color: egui::Color32) {
    let center = rect.center();
    let width = rect.width();
    let height = rect.height();
    // Two lobes and a point. Coarse, but at 12 px anything finer is indistinguishable.
    let lobe_radius = width * 0.26;
    let left_lobe = egui::pos2(center.x - lobe_radius, center.y - height * 0.12);
    let right_lobe = egui::pos2(center.x + lobe_radius, center.y - height * 0.12);
    let tip = egui::pos2(center.x, center.y + height * 0.38);

    if filled {
        painter.circle_filled(left_lobe, lobe_radius, color);
        painter.circle_filled(right_lobe, lobe_radius, color);
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(left_lobe.x - lobe_radius, left_lobe.y),
                egui::pos2(right_lobe.x + lobe_radius, right_lobe.y),
                tip,
            ],
            color,
            egui::Stroke::NONE,
        ));
    } else {
        let stroke = egui::Stroke::new(1.2_f32, color);
        painter.circle_stroke(left_lobe, lobe_radius, stroke);
        painter.circle_stroke(right_lobe, lobe_radius, stroke);
        painter.line_segment(
            [egui::pos2(left_lobe.x - lobe_radius, left_lobe.y), tip],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(right_lobe.x + lobe_radius, right_lobe.y), tip],
            stroke,
        );
    }
}

/// A streaming-overlay button: a painted glyph in a round-cornered square.
///
/// Icon-only. Labels were tried first, but three of them side by side ate most of a 960 px screen
/// and sat on top of the game.
pub(crate) fn stream_icon_button(ui: &mut egui::Ui, icon: StreamIcon, tint: egui::Color32) -> egui::Response {
    // Comfortably above the ~9 mm a fingertip covers on this screen.
    const BUTTON_SIZE: f32 = 30.0;
    const ICON_SIZE: f32 = 14.0;

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(BUTTON_SIZE, BUTTON_SIZE), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let painter = ui.painter();
    let fill = if response.is_pointer_button_down_on() {
        BG_DEEP
    } else {
        // Translucent so the game still shows through: this sits over live video.
        egui::Color32::from_rgba_unmultiplied(24, 24, 24, 210)
    };
    painter.rect_filled(rect, 6.0, fill);

    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(ICON_SIZE, ICON_SIZE));
    paint_stream_icon(painter, icon_rect, icon, tint);
    response
}


pub(crate) const STREAM_UI_RECTS: &str = "stream_ui_rects";

/// Screen-space rects (egui points) of the streaming screen's own controls as of the last frame.
///
/// While a session is live the touchscreen drives the host cursor, so every control the client
/// still owns has to carve its patch back out - otherwise it is drawn on screen but unreachable.
pub(crate) fn stream_ui_rects(ctx: &egui::Context) -> Vec<egui::Rect> {
    ctx.data(|data| {
        data.get_temp::<Vec<egui::Rect>>(egui::Id::new(STREAM_UI_RECTS))
            .unwrap_or_default()
    })
}

/// Claims `rect` for the client UI for the rest of this frame.
pub(crate) fn reserve_stream_touch(ctx: &egui::Context, rect: egui::Rect) {
    ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<Vec<egui::Rect>>(egui::Id::new(STREAM_UI_RECTS))
            .push(rect)
    });
}

/// Drops last frame's claims, so a control that is no longer drawn stops swallowing touches.
pub(crate) fn clear_stream_touch_reservations(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new(STREAM_UI_RECTS), Vec::<egui::Rect>::new()));
}

pub(crate) const KEYBOARD_CAP_SIZE: egui::Vec2 = egui::vec2(38.0, 26.0);
pub(crate) const KEYBOARD_CAP_SPACING: f32 = 2.0;
pub(crate) const KEYBOARD_COLUMNS: f32 = 15.0;
pub(crate) const KEYBOARD_ROWS: f32 = 7.0;
pub(crate) const KEYBOARD_PADDING: f32 = 8.0;

pub(crate) fn keyboard_panel_rect(screen: egui::Rect) -> egui::Rect {
    let height = KEYBOARD_ROWS * KEYBOARD_CAP_SIZE.y
        + (KEYBOARD_ROWS - 1.0) * KEYBOARD_CAP_SPACING
        + KEYBOARD_PADDING * 2.0;
    let min = egui::pos2(screen.min.x, screen.max.y - height);
    egui::Rect::from_min_size(min, egui::vec2(screen.width(), height))
}

pub(crate) fn keyboard_unit_width(inner_width: f32) -> f32 {
    (inner_width - (KEYBOARD_COLUMNS - 1.0) * KEYBOARD_CAP_SPACING) / KEYBOARD_COLUMNS
}

pub(crate) fn keyboard_key_width(units: f32, unit_width: f32) -> f32 {
    unit_width * units + KEYBOARD_CAP_SPACING * (units - 1.0)
}

/// Resolves the currently highlighted game.
pub(crate) fn controls_hint_overlay(ctx: &egui::Context, i18n: &I18n) -> Option<AppCommand> {
    let mut command = None;
    const HINT_ANIMATION: f64 = 0.9;
    let started_id = egui::Id::new("controls_hint_started_at");
    let now = ctx.input(|input| input.time);
    let started_at = ctx.data_mut(|data| *data.get_temp_mut_or_insert_with(started_id, || now));
    let progress = ((now - started_at) / HINT_ANIMATION).clamp(0.0, 1.0) as f32;
    if progress < 1.0 {
        ctx.request_repaint();
    }

    egui::Modal::new(egui::Id::new("controls_hint"))
        .backdrop_color(egui::Color32::from_black_alpha(200))
        .frame(
            egui::Frame::default()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0_f32, BORDER))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(16, 14)),
        )
        .show(ctx, |ui| {
            ui.set_width(320.0);
            ui.heading(egui::RichText::new(i18n.text("controls-hint-heading").as_ref()).size(15.0));
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(i18n.text("controls-hint-rear").as_ref())
                    .size(10.0)
                    .color(TEXT_DIM),
            );
            ui.add_space(8.0);
            rear_touch_diagram(ui, 112.0, Some(progress));
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(i18n.text("controls-hint-sticks").as_ref())
                    .size(10.0)
                    .color(TEXT_DIM),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(i18n.text("controls-hint-touch").as_ref())
                    .size(10.0)
                    .color(TEXT_DIM),
            );
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                if ui
                    .add_sized(
                        [130.0, 28.0],
                        egui::Button::new(i18n.text("controls-hint-dismiss").as_ref())
                            .fill(BG_RAISED),
                    )
                    .clicked()
                {
                    command = Some(AppCommand::DismissControlsHint);
                }
            });
        });
    command
}

pub(crate) fn pc_overlay_desktop_zones_visible() -> bool {
    use crate::gfn::stream_prefs as prefs;
    prefs::pc_overlay_enabled()
        && prefs::control_profile() == prefs::ControlProfile::Desktop
        && prefs::overlay_revealed()
}

/// Converts a normalized overlay rectangle into screen coordinates.
pub(crate) fn overlay_rect(screen: egui::Rect, bounds: opennow_core::input::layout::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(
            screen.min.x + screen.width() * bounds.x0,
            screen.min.y + screen.height() * bounds.y0,
        ),
        egui::pos2(
            screen.min.x + screen.width() * bounds.x1,
            screen.min.y + screen.height() * bounds.y1,
        ),
    )
}

/// When the control manual should stop being drawn, on egui's clock.
///
/// The manual used to be permanently painted across the middle of the screen in *both* profiles,
/// which meant a text card sat over Death Stranding for the whole session. It is a reference, so
/// it now lives in Settings, and this only flashes it briefly at the one moment it is actually
/// wanted: right after you switch profiles, when you might not remember what changed.
pub(crate) static MANUAL_UNTIL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub(crate) const MANUAL_FLASH_SECS: f64 = 4.0;

/// Called when the profile changes, to flash the manual.
pub(crate) fn flash_control_manual(now: f64) {
    MANUAL_UNTIL.store(
        (now + MANUAL_FLASH_SECS).to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
}

pub(crate) fn manual_is_flashing(now: f64) -> bool {
    now < f64::from_bits(MANUAL_UNTIL.load(std::sync::atomic::Ordering::Relaxed))
}

/// The mode toast: a brief, unmistakable answer to "what did the pill just do", for players who
/// tapped it without looking at the pill itself. Same lifetime pattern as the manual flash.
pub(crate) static TOAST_UNTIL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// 0 = game, 1 = desktop; matches `ControlProfile` order.
pub(crate) static TOAST_PROFILE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
pub(crate) const TOAST_TOTAL_SECS: f64 = 2.5;
pub(crate) const TOAST_FADE_SECS: f64 = 0.5;

/// Called from the shell when the profile actually changed, mirroring `flash_control_manual`.
pub(crate) fn flash_profile_toast(now: f64, profile: ControlProfile) {
    TOAST_PROFILE.store(
        match profile {
            ControlProfile::Game => 0,
            ControlProfile::Desktop => 1,
        },
        std::sync::atomic::Ordering::Relaxed,
    );
    TOAST_UNTIL.store(
        (now + TOAST_TOTAL_SECS).to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
}

pub(crate) fn toast_state(now: f64) -> Option<(ControlProfile, u8)> {
    let until = f64::from_bits(TOAST_UNTIL.load(std::sync::atomic::Ordering::Relaxed));
    if now >= until {
        return None;
    }
    let remaining = until - now;
    let alpha = ((remaining / TOAST_FADE_SECS).clamp(0.0, 1.0) * 255.0) as u8;
    let profile = match TOAST_PROFILE.load(std::sync::atomic::Ordering::Relaxed) {
        1 => ControlProfile::Desktop,
        _ => ControlProfile::Game,
    };
    Some((profile, alpha))
}

/// The always-reachable JUEGO|PC switch. One tap on the half you want; the active half is
/// accent-filled, the idle half dims with the same idle clock as the overlay so "always visible"
/// costs the picture as little as the key strip does. Its rect is reserved like the toolbar's,
/// so a tap lands on the pill and never on the game.
///
/// The halves are allocated with `Sense::click` and painted by hand, like `stream_icon_button`:
/// a plain `Frame::show` only senses hover, and a pill that never fires is worse than no pill.
pub(crate) fn mode_pill(ctx: &egui::Context, i18n: &I18n) -> Option<AppCommand> {
    use crate::gfn::stream_prefs as prefs;

    let profile = prefs::control_profile();
    let now = ctx.input(|input| input.time);
    // Same idle clock and floor as the game profile's key strip: dimmed is still readable,
    // because knowing which mode you are in is the pill's whole job.
    let faded = idle_seconds(now) > OVERLAY_FADE_AFTER;
    let body_alpha = if faded { 89 } else { 216 };
    let dim_alpha = if faded { 70 } else { 160 };

    let mut command = None;

    egui::Area::new(egui::Id::new("stream_mode_pill"))
        // Below the clock/battery pill's anchor row, so the two right-top pills stack instead of
        // overlap whether or not the session timer is on.
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 40.0))
        .show(ctx, |ui| {
            let halves = [
                (ControlProfile::Game, "🎮", i18n.text("stream-mode-pill-game")),
                (ControlProfile::Desktop, "🖱", i18n.text("stream-mode-pill-pc")),
            ];
            let half_widths: Vec<f32> = halves
                .iter()
                .map(|&(_, emoji, ref label)| {
                    pill_half_width(ui, emoji, label.as_ref())
                })
                .collect();
            let height = 24.0;
            let mut origin = ui.cursor().min;
            for (i, &(half, emoji, ref label)) in halves.iter().enumerate() {
                let rect = egui::Rect::from_min_size(
                    origin,
                    egui::vec2(half_widths[i], height),
                );
                origin.x += half_widths[i];
                let response = ui.allocate_rect(rect, egui::Sense::click());
                reserve_stream_touch(ui.ctx(), response.rect);
                if response.clicked() && command.is_none() {
                    command = Some(AppCommand::SelectControlProfile(half));
                }
                let active = profile == half;
                let painter = ui.painter();
                if active {
                    painter.rect_filled(rect, 6.0, ACCENT_DIM);
                    painter.rect_stroke(
                        rect,
                        6u8,
                        egui::Stroke::new(1.5_f32, ACCENT),
                        egui::StrokeKind::Inside,
                    );
                } else {
                    painter.rect_filled(
                        rect,
                        6.0,
                        egui::Color32::from_black_alpha(body_alpha),
                    );
                }
                let emoji_color = if active {
                    ACCENT
                } else {
                    egui::Color32::from_rgba_unmultiplied(0xa0, 0xa4, 0xac, dim_alpha)
                };
                let text_color = if active {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_rgba_unmultiplied(0xa0, 0xa4, 0xac, dim_alpha)
                };
                let emoji_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.min.x + 8.0, rect.center().y - 7.0),
                    egui::pos2(rect.min.x + 22.0, rect.center().y + 7.0),
                );
                painter.text(
                    emoji_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    emoji,
                    egui::FontId::proportional(12.0),
                    emoji_color,
                );
                let text_pos = egui::pos2(emoji_rect.max.x + 4.0, rect.center().y);
                painter.text(
                    text_pos,
                    egui::Align2::LEFT_CENTER,
                    label.as_ref(),
                    egui::FontId::proportional(11.0),
                    text_color,
                );
            }
        });

    command
}

/// Width of one pill half: its own label, measured - not guessed - plus icon, gaps and padding,
/// so a translated label never overflows its half.
pub(crate) fn pill_half_width(ui: &egui::Ui, emoji: &str, label: &str) -> f32 {
    let fonts = ui.fonts(|f| f.clone());
    let label_width = fonts
        .layout_no_wrap(label.to_owned(), egui::FontId::proportional(11.0), egui::Color32::WHITE)
        .size()
        .x;
    let _ = emoji; // the emoji box is a fixed 14px square; measured labels are what vary
    8.0 + 14.0 + 4.0 + label_width + 8.0
}

/// Paints the mode toast top-centre, over the video. Alpha comes from `toast_state`, so the
/// toast fades out over its last half second rather than blinking off.
pub(crate) fn paint_mode_toast(ctx: &egui::Context, i18n: &I18n, now: f64) {
    let Some((profile, alpha)) = toast_state(now) else {
        return;
    };
    let (title_key, hint_key) = match profile {
        ControlProfile::Game => ("stream-mode-toast-game", "stream-mode-toast-game-hint"),
        ControlProfile::Desktop => ("stream-mode-toast-pc", "stream-mode-toast-pc-hint"),
    };
    egui::Area::new(egui::Id::new("stream_mode_toast"))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 34.0))
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(egui::Color32::from_rgba_unmultiplied(0x0e, 0x0e, 0x0e, alpha))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(14, 8))
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    ACCENT.gamma_multiply(alpha as f32 / 255.0),
                ))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(i18n.text(title_key).as_ref())
                                .size(14.0)
                                .strong()
                                .color(egui::Color32::WHITE.gamma_multiply(alpha as f32 / 255.0)),
                        );
                        ui.label(
                            egui::RichText::new(i18n.text(hint_key).as_ref())
                                .size(11.0)
                                .color(egui::Color32::from_rgba_unmultiplied(
                                    0xa0, 0xa4, 0xac, alpha,
                                )),
                        );
                    });
                });
        });
    // Repaint until the toast is gone, or the fade freezes mid-opacity.
    ctx.request_repaint();
}

/// Paints the overlay from `opennow_core::input::layout::ZONES` - the same table the hit-test
/// reads, so a button cannot be drawn somewhere it does not respond.
///
/// Nothing here is an egui widget: registering these would hand the touches back to egui and break
/// the stream router.
pub(crate) fn paint_pc_overlay(ui: &mut egui::Ui, config: opennow_core::config::InputConfig) {
    use crate::gfn::stream_prefs as prefs;
    use opennow_core::input::layout::{ZoneId, live_zones};

    let screen = ui.ctx().screen_rect();
    let now = ui.ctx().input(|input| input.time);
    let base_alpha = prefs::overlay_opacity().alpha();
    let desktop = config.profile == opennow_core::config::ControlProfile::Desktop;

    // Auto-fade: after a few seconds without a touch the strip drops to a fraction of its
    // opacity, so "always visible" does not mean "always competing with the picture". Any touch
    // brings it straight back - `note_overlay_touch` is called from the shell's router.
    let faded = prefs::overlay_autofade() && !desktop && idle_seconds(now) > OVERLAY_FADE_AFTER;
    let alpha = if faded {
        (f32::from(base_alpha) * 0.35) as u8
    } else {
        base_alpha
    };

    let painter = ui.painter();
    for zone in live_zones(config) {
        if zone.id == ZoneId::Eye {
            continue; // drawn last, so nothing can ever paint over the way back
        }
        let rect = overlay_rect(screen, zone.rect).shrink(1.5);
        let (fill, border) = match zone.id {
            ZoneId::StickLeft | ZoneId::StickRight => (
                egui::Color32::from_rgba_unmultiplied(60, 110, 190, alpha),
                egui::Color32::from_rgba_unmultiplied(120, 170, 235, alpha),
            ),
            ZoneId::ModeToggle => (
                egui::Color32::from_rgba_unmultiplied(20, 24, 32, alpha.max(70)),
                egui::Color32::from_rgba_unmultiplied(200, 140, 40, alpha.max(70)),
            ),
            _ => (
                egui::Color32::from_rgba_unmultiplied(30, 34, 44, alpha),
                egui::Color32::from_rgba_unmultiplied(200, 140, 40, alpha),
            ),
        };
        if zone.id == ZoneId::StickLeft || zone.id == ZoneId::StickRight {
            // The stick corners are hidden, not absent, when the player asks for that: they still
            // work, they just stop drawing.
            if !prefs::stick_zones().is_visible() {
                continue;
            }
        }
        painter.rect_filled(rect, 5.0_f32, fill);
        painter.rect_stroke(
            rect,
            5u8,
            egui::Stroke::new(1.0_f32, border),
            egui::StrokeKind::Inside,
        );
        let label = if zone.id == ZoneId::ModeToggle {
            if desktop { "\u{1f5b1}" } else { "\u{1f3ae}" }
        } else {
            zone.label
        };
        if !label.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(if zone.id == ZoneId::ModeToggle {
                    16.0
                } else {
                    13.0
                }),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha.saturating_add(70)),
            );
        }
    }

    if manual_is_flashing(now) {
        ui.ctx().request_repaint();
        paint_control_manual(painter, screen, base_alpha, config.profile);
    }

    // The eye last, at a guaranteed alpha floor: it is the only way back once collapsed, so it can
    // never fade out of sight, not even under the auto-fade above.
    let eye_bounds = opennow_core::input::layout::ZONES
        .iter()
        .find(|zone| zone.id == ZoneId::Eye)
        .map(|zone| zone.rect)
        .expect("the eye is in the layout table");
    let eye = overlay_rect(screen, eye_bounds).shrink(3.0);
    let eye_alpha = base_alpha.max(70);
    painter.rect_filled(
        eye,
        6.0_f32,
        egui::Color32::from_rgba_unmultiplied(20, 24, 32, eye_alpha),
    );
    painter.text(
        eye.center(),
        egui::Align2::CENTER_CENTER,
        if prefs::overlay_revealed() {
            "\u{1f441}"
        } else {
            "\u{25cb}"
        },
        egui::FontId::proportional(18.0),
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, eye_alpha.saturating_add(60)),
    );
    painter.text(
        egui::pos2(eye.center().x, eye.max.y - 5.0),
        egui::Align2::CENTER_BOTTOM,
        if desktop { "PC" } else { "GAME" },
        egui::FontId::proportional(9.0),
        egui::Color32::from_rgba_unmultiplied(200, 140, 40, eye_alpha.saturating_add(60)),
    );
}

/// Seconds since the last front-panel touch, on egui's clock.
pub(crate) static LAST_TOUCH_AT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub(crate) const OVERLAY_FADE_AFTER: f64 = 6.0;

pub(crate) fn note_overlay_touch(now: f64) {
    LAST_TOUCH_AT.store(now.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn idle_seconds(now: f64) -> f64 {
    now - f64::from_bits(LAST_TOUCH_AT.load(std::sync::atomic::Ordering::Relaxed))
}

/// The control manual, generated from `opennow_core::input::bindings` rather than written out by
/// hand - a hand-written manual drifts from the code the first time a binding changes, and then
/// confidently describes a layout the client does not have.
pub(crate) fn paint_control_manual(
    painter: &egui::Painter,
    screen: egui::Rect,
    alpha: u8,
    profile: opennow_core::config::ControlProfile,
) {
    use opennow_core::config::ControlProfile;
    use opennow_core::input::bindings::manual;

    let rows: Vec<(&str, &str)> = manual(profile).collect();
    let title = match profile {
        ControlProfile::Desktop => "MODO ESCRITORIO",
        ControlProfile::Game => "MODO JUEGO",
    };

    let line_height = 13.0;
    let card = egui::Rect::from_center_size(
        screen.center(),
        egui::vec2(
            screen.width() * 0.52,
            line_height * (rows.len() as f32 + 1.8),
        ),
    );
    let card_alpha = alpha.saturating_sub(20).max(30);
    painter.rect_filled(
        card,
        6.0_f32,
        egui::Color32::from_rgba_unmultiplied(16, 19, 26, card_alpha),
    );
    painter.rect_stroke(
        card,
        6u8,
        egui::Stroke::new(
            1.0_f32,
            egui::Color32::from_rgba_unmultiplied(200, 140, 40, card_alpha),
        ),
        egui::StrokeKind::Inside,
    );
    let text_alpha = card_alpha.saturating_add(90);
    painter.text(
        egui::pos2(card.center().x, card.min.y + 4.0),
        egui::Align2::CENTER_TOP,
        title,
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgba_unmultiplied(200, 140, 40, text_alpha),
    );
    for (index, (control, effect)) in rows.iter().enumerate() {
        let y = card.min.y + line_height * (index as f32 + 1.8);
        painter.text(
            egui::pos2(card.min.x + 8.0, y),
            egui::Align2::LEFT_TOP,
            *control,
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgba_unmultiplied(235, 235, 245, text_alpha),
        );
        painter.text(
            egui::pos2(card.max.x - 8.0, y),
            egui::Align2::RIGHT_TOP,
            *effect,
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgba_unmultiplied(190, 196, 214, text_alpha),
        );
    }
}

pub(crate) fn paint_pulsing_zone(
    painter: &egui::Painter,
    cell: egui::Rect,
    label: &str,
    time: f64,
    phase: f64,
    font_size: f32,
) {
    let pulse = 0.5 + 0.5 * ((time * 3.0 + phase * std::f64::consts::TAU).sin() as f32);
    let cell = cell.shrink(2.0);
    painter.rect_filled(cell, 4.0, ACCENT.gamma_multiply(0.12 + pulse * 0.25));
    painter.rect_stroke(
        cell,
        4u8,
        egui::Stroke::new(1.5_f32, ACCENT.gamma_multiply(0.4 + pulse * 0.6)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        cell.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(font_size),
        egui::Color32::WHITE,
    );
}

pub(crate) fn paint_intro_zone(
    painter: &egui::Painter,
    cell: egui::Rect,
    label: &str,
    local: f32,
    font_size: f32,
) {
    if local <= 0.0 {
        return;
    }
    let cell = cell.shrink(2.0);
    let alpha = (local * 255.0) as u8;
    painter.rect_filled(
        cell,
        4.0,
        ACCENT.gamma_multiply(0.18).linear_multiply(local),
    );
    painter.rect_stroke(
        cell,
        4u8,
        egui::Stroke::new(1.0_f32, ACCENT.linear_multiply(local)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        cell.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(font_size),
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
    );
}

pub(crate) fn rear_touch_diagram(ui: &mut egui::Ui, max_height: f32, intro: Option<f32>) {
    let Some(texture) = vita_back(ui.ctx()) else {
        return;
    };
    let Some(image) = allocate_device_image(ui, &texture, max_height) else {
        return;
    };

    let mode = crate::gfn::stream_prefs::rear_touch_mode();
    let trigger_swap = crate::gfn::stream_prefs::trigger_swap_enabled();
    let (tl, tr) = if trigger_swap {
        ("L1", "R1")
    } else {
        ("L2", "R2")
    };
    let (bl, br) = ("L3", "R3");

    let pad = uv_subrect(image, REAR_PAD_X, REAR_PAD_Y);
    let painter = ui.painter();

    if let Some(progress) = intro {
        let halves = [("L2", 0.0_f32, 0.0), ("R2", 1.0, 0.5)];
        let cell_size = egui::vec2(pad.width() / 2.0, pad.height());
        for (index, (label, column, phase)) in halves.iter().enumerate() {
            let start = index as f32 * 0.18;
            let local = ((progress - start) / 0.4).clamp(0.0, 1.0);
            let cell = egui::Rect::from_min_size(
                egui::pos2(pad.min.x + column * cell_size.x, pad.min.y),
                cell_size,
            );
            let _ = phase;
            paint_intro_zone(painter, cell, label, local, 12.0);
        }
        return;
    }

    let time = ui.ctx().input(|input| input.time);
    ui.ctx().request_repaint();

    match mode {
        crate::gfn::stream_prefs::RearTouchMode::Halves => {
            let cell_size = egui::vec2(pad.width() / 2.0, pad.height());
            for (label, column, phase) in [(tl, 0.0_f32, 0.0_f64), (tr, 1.0, 0.5)] {
                let cell = egui::Rect::from_min_size(
                    egui::pos2(pad.min.x + column * cell_size.x, pad.min.y),
                    cell_size,
                );
                paint_pulsing_zone(painter, cell, label, time, phase, 12.0);
            }
        }
        crate::gfn::stream_prefs::RearTouchMode::Quadrant => {
            let cell_size = egui::vec2(pad.width() / 2.0, pad.height() / 2.0);
            for (label, column, row, phase) in [
                (tl, 0.0_f32, 0.0_f32, 0.00_f64),
                (tr, 1.0, 0.0, 0.25),
                (bl, 0.0, 1.0, 0.50),
                (br, 1.0, 1.0, 0.75),
            ] {
                let cell = egui::Rect::from_min_size(
                    egui::pos2(
                        pad.min.x + column * cell_size.x,
                        pad.min.y + row * cell_size.y,
                    ),
                    cell_size,
                );
                paint_pulsing_zone(painter, cell, label, time, phase, 10.0);
            }
        }
    }
}

pub(crate) fn front_stick_zones_diagram(ui: &mut egui::Ui, max_height: f32) {
    let Some(texture) = vita_front(ui.ctx()) else {
        return;
    };
    let Some(image) = allocate_device_image(ui, &texture, max_height) else {
        return;
    };

    let zones = crate::gfn::stream_prefs::stick_zones();
    if !zones.is_active() {
        return;
    }

    let screen = uv_subrect(image, FRONT_SCREEN_X, FRONT_SCREEN_Y);
    let time = ui.ctx().input(|input| input.time);
    ui.ctx().request_repaint();

    let painter = ui.painter();
    // Straight off the layout table, so the explainer can never point at a corner that moved.
    let config = crate::gfn::stream_prefs::input_config(true);
    let corner = |id| {
        opennow_core::input::layout::zone_rect(id, config)
            .map(|bounds| overlay_rect(screen, bounds))
            .unwrap_or(screen)
    };
    let left = corner(opennow_core::input::layout::ZoneId::StickLeft);
    let right = corner(opennow_core::input::layout::ZoneId::StickRight);
    paint_pulsing_zone(painter, left, "L3", time, 0.0, 10.0);
    paint_pulsing_zone(painter, right, "R3", time, 0.5, 10.0);
}

pub(crate) fn confirm_exit_modal(ctx: &egui::Context, i18n: &I18n) -> Option<AppCommand> {
    let mut command = None;
    // A plain `Window` here used to render behind the launch overlay's `Modal`: `Modal` claims
    // egui's dedicated modal input layer, so the exit confirmation was drawn but unreachable -
    // "Cancel session" looked like it did nothing. `Modal` stacks on top of an existing `Modal`
    // (the most recently shown one wins), which is what actually lets this dialog take clicks
    // while a session is being created.
    egui::Modal::new(egui::Id::new("confirm_exit_modal"))
        .backdrop_color(egui::Color32::from_black_alpha(180))
        .frame(
            egui::Frame::default()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0_f32, BORDER))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(16, 14)),
        )
        .show(ctx, |ui| {
            ui.set_width(LAUNCH_MODAL_WIDTH);
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading(egui::RichText::new(i18n.text("exit-heading").as_ref()).size(17.0));
                ui.add_space(10.0);
                ui.label(i18n.text("exit-body").as_ref());
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(i18n.text("exit-cancel").as_ref()).fill(BG_RAISED))
                        .clicked()
                    {
                        command = Some(AppCommand::CancelConfirmExit);
                    }
                    ui.add_space(16.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(i18n.text("exit-confirm").as_ref())
                                    .color(DANGER),
                            )
                            .fill(BG_RAISED),
                        )
                        .clicked()
                    {
                        command = Some(AppCommand::ConfirmExitSession);
                    }
                });
                ui.add_space(8.0);
            });
        });
    command
}

pub(crate) fn streaming_screen(
    ctx: &egui::Context,
    i18n: &I18n,
    game: Option<&GameSummary>,
    has_video: bool,
    status_note: Option<&str>,
    fps_history: &std::collections::VecDeque<f32>,
    keyboard_open: bool,
    show_stats: bool,
    toolbar_expanded: bool,
    mouse_trackpad_enabled: bool,
    battery: Option<crate::power::BatteryStatus>,
    session_start: Option<std::time::Instant>,
    membership_tier: Option<&str>,
    pause_menu_open: bool,
    hud: &crate::app::hud::HudState,
) -> Option<AppCommand> {
    let mut command = None;

    let mut frame = egui::Frame::central_panel(&ctx.style());
    frame.fill = egui::Color32::TRANSPARENT;
    egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
        if !has_video {
            ui.vertical_centered(|ui| {
                ui.add_space(70.0);
                ui.spinner();
                ui.add_space(16.0);
                match game {
                    Some(game) => ui.heading(
                        egui::RichText::new(
                            text1(i18n, "streaming-game", "game", &game.title).as_ref(),
                        )
                        .size(18.0),
                    ),
                    None => ui.heading(
                        egui::RichText::new(i18n.text("streaming-generic").as_ref()).size(18.0),
                    ),
                };
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new(i18n.text("streaming-signaling-done").as_ref())
                        .color(ACCENT)
                        .strong(),
                );
                ui.add_space(8.0);
                ui.label(
                    status_note
                        .map(str::to_owned)
                        .unwrap_or_else(|| i18n.text("streaming-waiting-negotiation").to_string()),
                );
            });
        }

        // Rebuilt every frame: a control that stops being drawn must stop claiming its touches.
        clear_stream_touch_reservations(ui.ctx());

        // The pause menu replaces the overlay while it is open: two different touch languages
        // stacked on the same screen is how a tap meant for one lands in the other.
        if has_video && !pause_menu_open && crate::gfn::stream_prefs::pc_overlay_enabled() {
            paint_pc_overlay(ui, crate::gfn::stream_prefs::input_config(true));
        }

        if crate::gfn::stream_prefs::session_timer_enabled() {
            let (timer_text, timer_color) = if let Some(start) = session_start {
                let elapsed = start.elapsed().as_secs() as u32;
                let max_duration = crate::gfn::auth::tier_max_duration_secs(membership_tier);
                let remaining = max_duration.saturating_sub(elapsed);
                let hours = remaining / 3600;
                let mins = (remaining % 3600) / 60;
                let s = remaining % 60;
                let text = format!("{hours:02}:{mins:02}:{s:02}");
                let color = if remaining < 180 {
                    DANGER
                } else if remaining < 600 {
                    WARNING
                } else {
                    egui::Color32::WHITE
                };
                (text, color)
            } else {
                (crate::power::formatted_system_time(), egui::Color32::WHITE)
            };

            egui::Area::new(egui::Id::new("stream_status_pill"))
                .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 8.0))
                .interactable(false)
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_black_alpha(190))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                        ))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;

                                let (icon_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(14.0, 14.0),
                                    egui::Sense::hover(),
                                );
                                paint_stream_icon(
                                    ui.painter(),
                                    icon_rect,
                                    StreamIcon::Clock,
                                    timer_color,
                                );

                                ui.label(
                                    egui::RichText::new(&timer_text)
                                        .size(12.0)
                                        .strong()
                                        .color(timer_color),
                                );

                                if let Some(battery) = battery {
                                    ui.label(egui::RichText::new("|").size(11.0).color(TEXT_DIM));
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(20.0, 14.0),
                                        egui::Sense::hover(),
                                    );
                                    paint_battery(ui.painter(), rect, battery);
                                    ui.label(
                                        egui::RichText::new(format!("{}%", battery.percent))
                                            .size(11.0)
                                            .strong()
                                            .color(battery_color(battery)),
                                    );
                                }
                            });
                        });
                });
        }

        // The JUEGO|PC pill: mode is a one-tap question, even with the toolbar collapsed.
        if let Some(cmd) = mode_pill(ctx, i18n) && command.is_none() {
            command = Some(cmd);
        }
        let toast_now = ctx.input(|input| input.time);
        paint_mode_toast(ctx, i18n, toast_now);

        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                if toolbar_expanded {
                    // 0. Pause menu: everything below also lives inside it, one tap deeper.
                    let menu_open = pause_menu_open;
                    let menu_button = stream_icon_button(
                        ui,
                        StreamIcon::Menu,
                        if menu_open { ACCENT } else { TEXT_DIM },
                    );
                    reserve_stream_touch(ui.ctx(), menu_button.rect);
                    if menu_button.clicked() && command.is_none() {
                        command = Some(AppCommand::OpenPauseMenu);
                    }

                    // 1. Power (Exit)
                    let power = stream_icon_button(ui, StreamIcon::Power, DANGER);
                    reserve_stream_touch(ui.ctx(), power.rect);
                    if power.clicked() {
                        command = Some(AppCommand::ToggleConfirmExit);
                    }

                    // 2. Stats
                    let stats = stream_icon_button(
                        ui,
                        StreamIcon::Stats,
                        if show_stats { ACCENT } else { TEXT_DIM },
                    );
                    reserve_stream_touch(ui.ctx(), stats.rect);
                    if stats.clicked() {
                        command = Some(AppCommand::ToggleStreamStats);
                    }

                    let timer_active = crate::gfn::stream_prefs::session_timer_enabled();
                    let timer = stream_icon_button(
                        ui,
                        StreamIcon::Clock,
                        if timer_active { ACCENT } else { TEXT_DIM },
                    );
                    reserve_stream_touch(ui.ctx(), timer.rect);
                    if timer.clicked() {
                        command = Some(AppCommand::ToggleSessionTimer);
                    }

                    // 3. Controls Settings (L2/R2 and L3/R3 modal)
                    let controls_active = crate::gfn::stream_prefs::stick_zones().is_active()
                        || crate::gfn::stream_prefs::trigger_intensity().value() > 0;
                    let controls = stream_icon_button(
                        ui,
                        StreamIcon::Controls,
                        if controls_active { ACCENT } else { TEXT_DIM },
                    );
                    reserve_stream_touch(ui.ctx(), controls.rect);
                    if controls.clicked() {
                        command = Some(AppCommand::ToggleControlsModal);
                    }

                    // 4. Mouse trackpad
                    let mouse = stream_icon_button(
                        ui,
                        StreamIcon::Mouse,
                        if mouse_trackpad_enabled {
                            ACCENT
                        } else {
                            TEXT_DIM
                        },
                    );
                    reserve_stream_touch(ui.ctx(), mouse.rect);
                    if mouse.clicked() {
                        command = Some(AppCommand::ToggleMouseTrackpad);
                    }

                    // 5. In-game keyboard
                    let keyboard = stream_icon_button(
                        ui,
                        StreamIcon::Keyboard,
                        if keyboard_open { ACCENT } else { TEXT_DIM },
                    );
                    reserve_stream_touch(ui.ctx(), keyboard.rect);
                    if keyboard.clicked() {
                        command = Some(AppCommand::ToggleKeyboard);
                    }

                    // 6. Collapse ◀
                    let collapse = stream_icon_button(ui, StreamIcon::Collapse, ACCENT);
                    reserve_stream_touch(ui.ctx(), collapse.rect);
                    if collapse.clicked() {
                        command = Some(AppCommand::ToggleToolbar);
                    }
                } else {
                    let expand = stream_icon_button(ui, StreamIcon::Expand, ACCENT);
                    reserve_stream_touch(ui.ctx(), expand.rect);
                    if expand.clicked() {
                        command = Some(AppCommand::ToggleToolbar);
                    }
                }
            });
        });

        if show_stats {
            egui::Area::new(egui::Id::new("stream_hud_area"))
                .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(8.0, -8.0))
                .interactable(false)
                .show(ctx, |ui| {
                    hud::paint_hud(ui, i18n, hud, fps_history, battery);
                });
        }
    });

    command
}

/// In-stream quick modal for adjusting L2/R2 rear-panel triggers and L3/R3 front-stick zones.
pub(crate) fn stream_controls_modal(ctx: &egui::Context, i18n: &I18n) -> Option<AppCommand> {
    let mut command = None;

    egui::Modal::new(egui::Id::new("stream_controls_modal"))
        .backdrop_color(egui::Color32::from_black_alpha(160))
        .frame(
            egui::Frame::default()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0_f32, BORDER))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(12, 10)),
        )
        .show(ctx, |ui| {
            ui.set_width(280.0);

            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new(i18n.text("controls-hint-heading").as_ref())
                        .size(14.0)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_sized(
                            [26.0, 22.0],
                            egui::Button::new(egui::RichText::new("X").size(12.0).strong()),
                        )
                        .clicked()
                    {
                        command = Some(AppCommand::ToggleControlsModal);
                    }
                });
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            rear_touch_diagram(ui, 72.0, None);
            ui.add_space(6.0);

            if let Some(chosen) = settings_row(
                ui,
                i18n,
                "settings-trigger-heading",
                crate::gfn::stream_prefs::TriggerIntensity::ALL
                    .iter()
                    .copied(),
                crate::gfn::stream_prefs::trigger_intensity(),
                |candidate| format!("{}%", u32::from(candidate.value()) * 100 / 255),
            ) {
                command = Some(AppCommand::SetTriggerIntensity(chosen));
            }

            if let Some(chosen) = settings_row(
                ui,
                i18n,
                "settings-rear-touch-mode-heading",
                crate::gfn::stream_prefs::RearTouchMode::ALL.iter().copied(),
                crate::gfn::stream_prefs::rear_touch_mode(),
                |candidate| i18n.text(candidate.label_key()).to_string(),
            ) {
                command = Some(AppCommand::SetRearTouchMode(chosen));
            }

            ui.add_space(2.0);

            if let Some(chosen) = settings_row(
                ui,
                i18n,
                "settings-stick-zones-heading",
                crate::gfn::stream_prefs::StickZones::ALL.iter().copied(),
                crate::gfn::stream_prefs::stick_zones(),
                |candidate| i18n.text(candidate.label_key()).to_string(),
            ) {
                command = Some(AppCommand::SetStickZones(chosen));
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(i18n.text("settings-trigger-swap-heading").as_ref())
                        .size(11.0)
                        .color(egui::Color32::WHITE),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut value = crate::gfn::stream_prefs::trigger_swap_enabled();
                    if ui.add(egui::Checkbox::without_text(&mut value)).changed() {
                        command = Some(AppCommand::ToggleTriggerSwap);
                    }
                });
            });

            ui.add_space(8.0);

            if ui
                .add_sized(
                    [ui.available_width(), 24.0],
                    egui::Button::new(
                        egui::RichText::new(i18n.text("account-close").as_ref()).size(11.0),
                    )
                    .fill(BG_RAISED),
                )
                .clicked()
            {
                command = Some(AppCommand::ToggleControlsModal);
            }
        });

    command
}

pub(crate) enum KeyCap {
    Char(char, char),
    /// Switches the panel to the Windows-shortcut page.
    Shortcuts,
    Key(&'static str, crate::gfn::input_protocol::KeyStroke),
    Backspace,
    Enter,
    Space,
    Shift,
    Ctrl,
    Alt,
}

pub(crate) fn keyboard_layout() -> [Vec<(KeyCap, f32)>; 7] {
    use crate::gfn::input_protocol::*;
    [
        vec![
            (KeyCap::Key("Esc", KEY_ESCAPE), 1.0),
            (KeyCap::Key("F1", KEY_F1), 1.0),
            (KeyCap::Key("F2", KEY_F2), 1.0),
            (KeyCap::Key("F3", KEY_F3), 1.0),
            (KeyCap::Key("F4", KEY_F4), 1.0),
            (KeyCap::Key("F5", KEY_F5), 1.0),
            (KeyCap::Key("F6", KEY_F6), 1.0),
            (KeyCap::Key("F7", KEY_F7), 1.0),
            (KeyCap::Key("F8", KEY_F8), 1.0),
            (KeyCap::Key("F9", KEY_F9), 1.0),
            (KeyCap::Key("F10", KEY_F10), 1.0),
            (KeyCap::Key("F11", KEY_F11), 1.0),
            (KeyCap::Key("F12", KEY_F12), 1.0),
            (KeyCap::Key("Home", KEY_HOME), 1.0),
            (KeyCap::Key("End", KEY_END), 1.0),
        ],
        vec![
            (KeyCap::Key("Ins", KEY_INSERT), 1.0),
            (KeyCap::Key("Supr", KEY_DELETE), 1.0),
            (KeyCap::Key("PgUp", KEY_PAGE_UP), 1.0),
            (KeyCap::Key("PgDn", KEY_PAGE_DOWN), 1.0),
            (KeyCap::Key("Caps", KEY_CAPS_LOCK), 1.5),
            (KeyCap::Shortcuts, 2.0),
        ],
        vec![
            (KeyCap::Char('`', '~'), 1.0),
            (KeyCap::Char('1', '!'), 1.0),
            (KeyCap::Char('2', '@'), 1.0),
            (KeyCap::Char('3', '#'), 1.0),
            (KeyCap::Char('4', '$'), 1.0),
            (KeyCap::Char('5', '%'), 1.0),
            (KeyCap::Char('6', '^'), 1.0),
            (KeyCap::Char('7', '&'), 1.0),
            (KeyCap::Char('8', '*'), 1.0),
            (KeyCap::Char('9', '('), 1.0),
            (KeyCap::Char('0', ')'), 1.0),
            (KeyCap::Char('-', '_'), 1.0),
            (KeyCap::Char('=', '+'), 1.0),
            (KeyCap::Backspace, 2.0),
        ],
        vec![
            (KeyCap::Key("Tab", KEY_TAB), 1.5),
            (KeyCap::Char('q', 'Q'), 1.0),
            (KeyCap::Char('w', 'W'), 1.0),
            (KeyCap::Char('e', 'E'), 1.0),
            (KeyCap::Char('r', 'R'), 1.0),
            (KeyCap::Char('t', 'T'), 1.0),
            (KeyCap::Char('y', 'Y'), 1.0),
            (KeyCap::Char('u', 'U'), 1.0),
            (KeyCap::Char('i', 'I'), 1.0),
            (KeyCap::Char('o', 'O'), 1.0),
            (KeyCap::Char('p', 'P'), 1.0),
            (KeyCap::Char('[', '{'), 1.0),
            (KeyCap::Char(']', '}'), 1.0),
            (KeyCap::Char('\\', '|'), 1.5),
        ],
        vec![
            (KeyCap::Key("Caps", KEY_CAPS_LOCK), 1.75),
            (KeyCap::Char('a', 'A'), 1.0),
            (KeyCap::Char('s', 'S'), 1.0),
            (KeyCap::Char('d', 'D'), 1.0),
            (KeyCap::Char('f', 'F'), 1.0),
            (KeyCap::Char('g', 'G'), 1.0),
            (KeyCap::Char('h', 'H'), 1.0),
            (KeyCap::Char('j', 'J'), 1.0),
            (KeyCap::Char('k', 'K'), 1.0),
            (KeyCap::Char('l', 'L'), 1.0),
            (KeyCap::Char(';', ':'), 1.0),
            (KeyCap::Char('\'', '"'), 1.0),
            (KeyCap::Enter, 2.25),
        ],
        vec![
            (KeyCap::Shift, 2.25),
            (KeyCap::Char('z', 'Z'), 1.0),
            (KeyCap::Char('x', 'X'), 1.0),
            (KeyCap::Char('c', 'C'), 1.0),
            (KeyCap::Char('v', 'V'), 1.0),
            (KeyCap::Char('b', 'B'), 1.0),
            (KeyCap::Char('n', 'N'), 1.0),
            (KeyCap::Char('m', 'M'), 1.0),
            (KeyCap::Char(',', '<'), 1.0),
            (KeyCap::Char('.', '>'), 1.0),
            (KeyCap::Char('/', '?'), 1.0),
            (KeyCap::Shift, 2.75),
        ],
        vec![
            (KeyCap::Ctrl, 1.25),
            (KeyCap::Alt, 1.25),
            (KeyCap::Key("Win", KEY_LEFT_WIN), 1.25),
            (KeyCap::Space, 4.25),
            (KeyCap::Key("AltGr", KEY_RIGHT_ALT), 1.0),
            (KeyCap::Key("Menu", KEY_MENU), 1.0),
            (KeyCap::Key("Ctrl", KEY_RIGHT_CTRL), 1.0),
            (KeyCap::Key("<", KEY_LEFT), 1.0),
            (KeyCap::Key("^", KEY_UP), 1.0),
            (KeyCap::Key("v", KEY_DOWN), 1.0),
            (KeyCap::Key(">", KEY_RIGHT), 1.0),
        ],
    ]
}

/// One row of the Windows-shortcut page: what it is called and what it sends.
///
/// These exist because reaching `Win+D` by latching Win and then pressing D is three deliberate
/// actions on a touchscreen you are holding with both hands. Every one of these is a single tap.
pub(crate) fn windows_shortcuts() -> Vec<(&'static str, AppCommand)> {
    use crate::gfn::input_protocol as proto;
    let chord = |shift, ctrl, alt, win, key| AppCommand::SendChord {
        shift,
        ctrl,
        alt,
        win,
        key,
    };
    let letter = |ch: char| proto::key_for_char(ch).expect("an ASCII letter always maps");
    vec![
        ("Inicio", AppCommand::SendKey(proto::KEY_LEFT_WIN)),
        ("Escritorio", chord(false, false, false, true, letter('d'))),
        ("Explorador", chord(false, false, false, true, letter('e'))),
        (
            "Vista tareas",
            chord(false, false, false, true, proto::KEY_TAB),
        ),
        ("Maximizar", chord(false, false, false, true, proto::KEY_UP)),
        (
            "Cambiar ventana",
            chord(false, false, true, false, proto::KEY_TAB),
        ),
        (
            "Cerrar ventana",
            chord(false, false, true, false, proto::KEY_F4),
        ),
        // The one that was impossible before: it needs Shift as a real held key.
        (
            "Administrador",
            chord(true, true, false, false, proto::KEY_ESCAPE),
        ),
        ("Copiar", chord(false, true, false, false, letter('c'))),
        ("Pegar", chord(false, true, false, false, letter('v'))),
        ("Deshacer", chord(false, true, false, false, letter('z'))),
        (
            "Seleccionar todo",
            chord(false, true, false, false, letter('a')),
        ),
        ("Pantalla completa", AppCommand::SendKey(proto::KEY_F11)),
        ("Bloquear PC", chord(false, false, false, true, letter('l'))),
        (
            "Seguridad",
            chord(false, true, true, false, proto::KEY_DELETE),
        ),
        ("Buscar", chord(false, false, false, true, letter('s'))),
    ]
}

/// The shortcut page, in place of the keys.
pub(crate) fn windows_shortcuts_page(ctx: &egui::Context) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    let panel_rect = keyboard_panel_rect(ctx.screen_rect());
    reserve_stream_touch(ctx, panel_rect);

    egui::Area::new(egui::Id::new("windows_shortcuts_page"))
        .fixed_pos(panel_rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let translucency = crate::gfn::stream_prefs::overlay_opacity()
                .multiplier()
                .clamp(0.35, 0.9);
            egui::Frame::window(&ui.style())
                .fill(BG_PANEL.gamma_multiply(translucency))
                .inner_margin(egui::Margin::same(KEYBOARD_PADDING as i8))
                .outer_margin(egui::Margin::ZERO)
                .shadow(egui::Shadow::NONE)
                .corner_radius(0.0)
                .show(ui, |ui| {
                    let inner_width = panel_rect.width() - KEYBOARD_PADDING * 2.0;
                    ui.set_width(inner_width);
                    ui.spacing_mut().item_spacing =
                        egui::vec2(KEYBOARD_CAP_SPACING, KEYBOARD_CAP_SPACING);

                    let shortcuts = windows_shortcuts();
                    let columns = 4.0_f32;
                    let width = (inner_width - (columns - 1.0) * KEYBOARD_CAP_SPACING) / columns;
                    for row in shortcuts.chunks(4) {
                        ui.horizontal(|ui| {
                            for (label, command) in row {
                                let button =
                                    egui::Button::new(egui::RichText::new(*label).size(10.0))
                                        .fill(BG_RAISED.gamma_multiply(translucency));
                                if ui.add_sized([width, KEYBOARD_CAP_SIZE.y], button).clicked() {
                                    commands.push(command.clone());
                                }
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        let back =
                            egui::Button::new(egui::RichText::new("\u{2328} Teclado").size(10.0))
                                .fill(ACCENT.gamma_multiply(0.35));
                        if ui.add_sized([width, KEYBOARD_CAP_SIZE.y], back).clicked() {
                            commands.push(AppCommand::OpenShortcuts);
                        }
                    });
                });
        });

    commands
}

pub(crate) fn on_screen_keyboard(ctx: &egui::Context, shift: bool, ctrl: bool, alt: bool) -> Vec<AppCommand> {
    use crate::gfn::input_protocol::key_for_char;

    let mut commands = Vec::new();
    let panel_rect = keyboard_panel_rect(ctx.screen_rect());
    reserve_stream_touch(ctx, panel_rect);

    egui::Area::new(egui::Id::new("on_screen_keyboard"))
        .fixed_pos(panel_rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            // Tied to the overlay's opacity preference so the picture stays readable behind the
            // keyboard; it used to be effectively solid at 0.96.
            let translucency = crate::gfn::stream_prefs::overlay_opacity()
                .multiplier()
                .clamp(0.35, 0.9);
            egui::Frame::window(&ui.style())
                .fill(BG_PANEL.gamma_multiply(translucency))
                .inner_margin(egui::Margin::same(KEYBOARD_PADDING as i8))
                .outer_margin(egui::Margin::ZERO)
                .shadow(egui::Shadow::NONE)
                .corner_radius(0.0)
                .show(ui, |ui| {
                    let inner_width = panel_rect.width() - KEYBOARD_PADDING * 2.0;
                    let unit_width = keyboard_unit_width(inner_width);
                    ui.set_width(inner_width);
                    ui.spacing_mut().item_spacing =
                        egui::vec2(KEYBOARD_CAP_SPACING, KEYBOARD_CAP_SPACING);

                    for row in keyboard_layout() {
                        ui.horizontal(|ui| {
                            for (cap, units) in row {
                                let (label, active) = match &cap {
                                    KeyCap::Char(lower, upper) => (
                                        if shift {
                                            upper.to_string()
                                        } else {
                                            lower.to_string()
                                        },
                                        false,
                                    ),
                                    KeyCap::Key(label, _) => (label.to_string(), false),
                                    KeyCap::Backspace => ("Bksp".to_string(), false),
                                    KeyCap::Enter => ("Enter".to_string(), false),
                                    KeyCap::Space => (String::new(), false),
                                    KeyCap::Shortcuts => ("Atajos".to_string(), false),
                                    KeyCap::Shift => ("Shift".to_string(), shift),
                                    KeyCap::Ctrl => ("Ctrl".to_string(), ctrl),
                                    KeyCap::Alt => ("Alt".to_string(), alt),
                                };
                                let width = keyboard_key_width(units, unit_width);
                                let mut button =
                                    egui::Button::new(egui::RichText::new(label).size(11.0));
                                button = if active {
                                    button.fill(ACCENT.gamma_multiply(0.35))
                                } else {
                                    button.fill(BG_RAISED.gamma_multiply(translucency))
                                };
                                let response = ui.add_sized([width, KEYBOARD_CAP_SIZE.y], button);
                                if !response.clicked() {
                                    continue;
                                }
                                match cap {
                                    KeyCap::Char(lower, upper) => {
                                        let ch = if shift { upper } else { lower };
                                        if let Some(key) = key_for_char(ch) {
                                            // Shift is deliberately not forwarded here: it already
                                            // picked the character, and sending it as a key too
                                            // would make the host see Shift+A rather than `A`.
                                            commands.push(if ctrl || alt {
                                                AppCommand::SendChord {
                                                    shift: false,
                                                    ctrl,
                                                    alt,
                                                    win: false,
                                                    key,
                                                }
                                            } else {
                                                AppCommand::SendKey(key)
                                            });
                                        }
                                    }
                                    KeyCap::Key(_, key) => {
                                        commands.push(if ctrl || alt || shift {
                                            AppCommand::SendChord {
                                                shift,
                                                ctrl,
                                                alt,
                                                win: false,
                                                key,
                                            }
                                        } else {
                                            AppCommand::SendKey(key)
                                        });
                                    }
                                    KeyCap::Backspace => {
                                        let key = crate::gfn::input_protocol::KEY_BACKSPACE;
                                        commands.push(if ctrl || alt || shift {
                                            AppCommand::SendChord {
                                                shift,
                                                ctrl,
                                                alt,
                                                win: false,
                                                key,
                                            }
                                        } else {
                                            AppCommand::SendKey(key)
                                        });
                                    }
                                    KeyCap::Enter => {
                                        let key = crate::gfn::input_protocol::KEY_ENTER;
                                        commands.push(if ctrl || alt || shift {
                                            AppCommand::SendChord {
                                                shift,
                                                ctrl,
                                                alt,
                                                win: false,
                                                key,
                                            }
                                        } else {
                                            AppCommand::SendKey(key)
                                        });
                                    }
                                    KeyCap::Space => {
                                        let key = crate::gfn::input_protocol::KEY_SPACE;
                                        commands.push(if ctrl || alt || shift {
                                            AppCommand::SendChord {
                                                shift,
                                                ctrl,
                                                alt,
                                                win: false,
                                                key,
                                            }
                                        } else {
                                            AppCommand::SendKey(key)
                                        });
                                    }
                                    KeyCap::Shortcuts => commands.push(AppCommand::OpenShortcuts),
                                    KeyCap::Shift => commands.push(AppCommand::ToggleKeyShift),
                                    KeyCap::Ctrl => commands.push(AppCommand::ToggleKeyCtrl),
                                    KeyCap::Alt => commands.push(AppCommand::ToggleKeyAlt),
                                }
                            }
                        });
                    }
                });
        });

    commands
}
