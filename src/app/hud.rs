//! The in-stream performance HUD: the Halyard-style metrics panel (sections, graded rows,
//! sparklines) painted onto the OpenNOW palette.
//!
//! Efficiency contract (megaplan H2, budget < 2 ms/frame): the peer's stats line is parsed at
//! `hud_refresh_ms` intervals, never per frame; the paint is galley-free painter text plus one
//! polyline per chart, and when the HUD is off none of this code runs at all.

use crate::app::theme::{Grade, for_grade};
use crate::app::ui::battery_color;
use crate::gfn::stream_prefs as prefs;
use crate::i18n::I18n;
use egui::{Color32, Painter, Pos2, Rect, Vec2};
use opennow_core::hud::{HudSample, RingF32, decode_grade, jitter_grade, loss_grade, rtt_grade};
use std::time::Instant;

/// FPS colour bands, loosely matching GeForce NOW's own overlay (green/yellow/red) and the
/// `fps_grade` thresholds in core - the grade decides, this only colours it.
pub(crate) fn fps_color(fps: f32) -> Color32 {
    for_grade(opennow_core::hud::fps_grade(fps))
}

/// Sampled state for the HUD panel. Owned by `App`, fed by [`HudState::sample`] from the
/// shell's tick (which has `&mut app`), read by the paint.
pub(crate) struct HudState {
    last_parse: Instant,
    sample: HudSample,
    bitrate_ring: RingF32,
}

/// Ring length at the default 500 ms refresh is two minutes of history; at 250 ms, one minute.
/// Enough for "did it just get worse" without re-allocating ever.
const RING_CAPACITY: usize = 240;

impl Default for HudState {
    fn default() -> Self {
        Self {
            last_parse: Instant::now(),
            sample: HudSample::default(),
            bitrate_ring: RingF32::new(RING_CAPACITY),
        }
    }
}

impl HudState {
    /// Re-parses the stats line only when the refresh interval elapsed. The bitrate ring is
    /// pushed on every parse, so the chart's horizontal pace matches the refresh step.
    pub(crate) fn sample_stats(&mut self, note: Option<&str>, refresh_ms: u16) {
        let refresh = std::time::Duration::from_millis(refresh_ms.max(100) as u64);
        if self.last_parse.elapsed() < refresh {
            return;
        }
        self.last_parse = Instant::now();
        let Some(note) = note else {
            return;
        };
        self.sample = HudSample::parse(note);
        if let Some(kbps) = self.sample.kbps {
            self.bitrate_ring.push(kbps);
        }
    }
}

/// One display row: localized label, formatted value, and its grade (Neutral for rows that
/// carry no judgement, like the bitrate readout).
struct Row {
    label_key: &'static str,
    value: String,
    grade: Grade,
}

/// Paints the HUD card, bottom-left. Nothing interactive: it is a readout, so the whole rect
/// is `Sense::hover` and touches pass through to the stream untouched.
pub(crate) fn paint_hud(
    ui: &mut egui::Ui,
    i18n: &I18n,
    state: &HudState,
    fps_history: &std::collections::VecDeque<f32>,
    battery: Option<crate::power::BatteryStatus>,
) {
    let s = &state.sample;
    let fps = fps_history.back().copied().unwrap_or(0.0);

    let video_rows = [
        Row {
            label_key: "hud-fps",
            value: format!("{fps:.0}"),
            grade: opennow_core::hud::fps_grade(fps),
        },
        Row {
            label_key: "hud-bitrate",
            value: match s.kbps {
                Some(kbps) => format!("{:.1} Mbps", kbps / 1000.0),
                None => "-".to_owned(),
            },
            grade: Grade::Neutral,
        },
        Row {
            label_key: "hud-decode",
            value: fmt_ms(s.decode_ms),
            grade: graded(s.decode_ms, decode_grade),
        },
    ];
    let net_rows = [
        Row {
            label_key: "hud-ping",
            value: fmt_ms(s.rtt_ms),
            grade: graded(s.rtt_ms, rtt_grade),
        },
        Row {
            label_key: "hud-jitter",
            value: fmt_ms(s.jitter_ms),
            grade: graded(s.jitter_ms, jitter_grade),
        },
        Row {
            label_key: "hud-loss",
            value: match s.loss_pct {
                Some(v) => format!("{v:.1} %"),
                None => "-".to_owned(),
            },
            grade: graded(s.loss_pct, loss_grade),
        },
        Row {
            label_key: "hud-drop",
            value: match s.drop_per_sec {
                Some(v) => format!("{v:.0}/s"),
                None => "-".to_owned(),
            },
            grade: Grade::Neutral,
        },
    ];

    let opacity = prefs::hud_opacity_percent().min(100) as f32 / 100.0;
    let label_font = egui::FontId::proportional(9.0);
    let value_font = egui::FontId::monospace(9.0);
    let section_font = egui::FontId::proportional(8.5);
    let title_font = egui::FontId::proportional(10.5);
    let row_height = 13.0;
    let section_header = 10.0;
    let section_gap = 5.0;
    let padding = 8.0;
    let panel_width = 152.0;
    let chart_height = 22.0;
    let chart_count =
        usize::from(prefs::hud_fps_chart()) + usize::from(prefs::hud_bitrate_chart());
    let content_height = 18.0
        + (video_rows.len() + net_rows.len()) as f32 * row_height
        + 2.0 * (section_header + section_gap)
        + chart_count as f32 * (chart_height + 5.0)
        + 12.0; // input line

    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(panel_width, content_height + padding * 2.0),
        egui::Sense::hover(),
    );
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter().clone();
    painter.rect_filled(
        rect,
        6.0,
        Color32::from_rgba_unmultiplied(10, 10, 10, (215.0 * opacity) as u8),
    );
    painter.rect_stroke(
        rect,
        6u8,
        egui::Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(34, 34, 34, (255.0 * opacity) as u8),
        ),
        egui::StrokeKind::Inside,
    );

    let mut y = rect.min.y + padding;
    let left = rect.min.x + padding;
    let value_x = rect.max.x - padding;

    // Title + battery on one line.
    painter.text(
        Pos2::new(left, y + 6.0),
        egui::Align2::LEFT_CENTER,
        i18n.text("hud-title"),
        title_font,
        Color32::WHITE.gamma_multiply(opacity * 0.9),
    );
    if let Some(battery) = battery {
        painter.text(
            Pos2::new(value_x, y + 6.0),
            egui::Align2::RIGHT_CENTER,
            format!("{}%", battery.percent),
            value_font.clone(),
            battery_color(battery).gamma_multiply(opacity),
        );
    }
    y += 18.0;

    let paint_section = |painter: &Painter, title_key: &'static str, rows: &[Row], y: &mut f32| {
        painter.text(
            Pos2::new(left, *y + section_header / 2.0),
            egui::Align2::LEFT_CENTER,
            i18n.text(title_key),
            section_font.clone(),
            crate::app::theme::ACCENT.gamma_multiply(opacity * 0.85),
        );
        *y += section_header;
        for row in rows {
            painter.text(
                Pos2::new(left, *y + row_height / 2.0),
                egui::Align2::LEFT_CENTER,
                i18n.text(row.label_key),
                label_font.clone(),
                crate::app::theme::TEXT_DIM.gamma_multiply(opacity),
            );
            painter.text(
                Pos2::new(value_x, *y + row_height / 2.0),
                egui::Align2::RIGHT_CENTER,
                &row.value,
                value_font.clone(),
                for_grade(row.grade).gamma_multiply(opacity),
            );
            *y += row_height;
        }
        *y += section_gap;
    };

    paint_section(&painter, "hud-section-video", &video_rows, &mut y);
    paint_section(&painter, "hud-section-net", &net_rows, &mut y);

    if prefs::hud_fps_chart() {
        paint_sparkline(
            &painter,
            Rect::from_min_size(
                Pos2::new(left, y),
                Vec2::new(panel_width - padding * 2.0, chart_height),
            ),
            &fps_history.iter().copied().collect::<Vec<f32>>(),
            60.0,
            fps_color(fps).gamma_multiply(opacity),
            opacity,
        );
        y += chart_height + 5.0;
    }
    if prefs::hud_bitrate_chart() {
        let values = (0..state.bitrate_ring.len())
            .map(|i| state.bitrate_ring.at(i))
            .collect::<Vec<f32>>();
        paint_sparkline(
            &painter,
            Rect::from_min_size(
                Pos2::new(left, y),
                Vec2::new(panel_width - padding * 2.0, chart_height),
            ),
            &values,
            25_000.0,
            Color32::from_rgb(0x9c, 0xd3, 0x2b).gamma_multiply(opacity),
            opacity,
        );
    }

    // The router's own line: a "the controls don't work" report can be checked against what
    // the router actually decided (carried over from the strip this panel replaces).
    painter.text(
        Pos2::new(left, rect.max.y - padding),
        egui::Align2::LEFT_BOTTOM,
        crate::input::stats::line(),
        egui::FontId::monospace(7.5),
        Color32::from_rgba_unmultiplied(0xa0, 0xa4, 0xac, (150.0 * opacity) as u8),
    );
}

fn fmt_ms(value: Option<f32>) -> String {
    match value {
        Some(v) => format!("{v:.1} ms"),
        None => "-".to_owned(),
    }
}

/// A missing value is Neutral, not graded: "no data" and "bad data" are different facts.
fn graded(value: Option<f32>, grade: fn(f32) -> Grade) -> Grade {
    value.map_or(Grade::Neutral, grade)
}

/// A one-colour polyline over the last samples, newest at the right. The floor keeps a quiet
/// line flat instead of magnifying its noise into a mountain range.
fn paint_sparkline(
    painter: &Painter,
    rect: Rect,
    values: &[f32],
    floor_max: f32,
    color: Color32,
    opacity: f32,
) {
    painter.rect_filled(
        rect,
        3.0,
        Color32::from_rgba_unmultiplied(255, 255, 255, (14.0 * opacity) as u8),
    );
    if values.len() < 2 {
        return;
    }
    let scale = values.iter().copied().fold(0.0_f32, f32::max).max(floor_max);
    let n = values.len();
    let points: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let t = i as f32 / (n - 1) as f32;
            let norm = (v / scale).clamp(0.0, 1.0);
            Pos2::new(
                rect.min.x + t * rect.width(),
                rect.max.y - norm * rect.height(),
            )
        })
        .collect();
    painter.add(egui::Shape::line(points, egui::Stroke::new(1.0_f32, color)));
}
