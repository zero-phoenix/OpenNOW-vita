//! The in-stream pause menu: a paginated overlay over the live stream (Halyard's
//! `OverlayMenu`, on the OpenNOW palette) gathering every stream control that used to live in
//! scattered toolbar icons and modals.
//!
//! Navigation state is `opennow_core::menu::MenuState` (pure, test-walked); this file supplies
//! the tree, maps ids to `AppCommand`s, and paints. Conventions: `Action` leaves close the
//! menu; toggles and steppers stay open so the player watches the effect (the HUD is visible
//! behind the menu's transparent backdrop); Left/Right step the row under the cursor.

use crate::app::theme::{ACCENT, BG_PANEL, BG_RAISED, BORDER, TEXT_DIM};
use crate::app::stream_ui::reserve_stream_touch;
use crate::gfn::stream_prefs as prefs;
use crate::i18n::I18n;
use crate::input::AppCommand;
use egui::{Color32, Sense};
use opennow_core::menu::{MenuNode, MenuState};

/// Every row the menu can show. Copy + Eq so the tree can live as consts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PauseId {
    Continue,
    Video,
    Panel,
    Controls,
    Quit,
    // Vídeo page
    Bitrate,
    StatsToggle,
    // Panel page
    Opacity,
    Refresh,
    FpsChart,
    BitrateChart,
    Timer,
    // Controles page
    Profile,
    Keyboard,
    Trackpad,
    OverlayPc,
    Autofade,
    Triggers,
}

use PauseId::{
    Autofade as AUTOFADE, Bitrate as BITRATE, BitrateChart as BITRATE_CHART, Continue as CONTINUE,
    Controls as CONTROLS, FpsChart as FPS_CHART, Keyboard as KEYBOARD, Opacity as OPACITY,
    OverlayPc as OVERLAY_PC, Panel as PANEL, Profile as PROFILE, Quit as QUIT, Refresh as REFRESH,
    StatsToggle as STATS_TOGGLE, Timer as TIMER, Trackpad as TRACKPAD, Triggers as TRIGGERS,
    Video as VIDEO,
};

const VIDEO_PAGE: &[MenuNode<PauseId>] =
    &[MenuNode::Leaf(BITRATE), MenuNode::Leaf(STATS_TOGGLE)];

const PANEL_PAGE: &[MenuNode<PauseId>] = &[
    MenuNode::Leaf(OPACITY),
    MenuNode::Leaf(REFRESH),
    MenuNode::Leaf(FPS_CHART),
    MenuNode::Leaf(BITRATE_CHART),
    MenuNode::Leaf(TIMER),
];

const CONTROLS_PAGE: &[MenuNode<PauseId>] = &[
    MenuNode::Leaf(PROFILE),
    MenuNode::Leaf(KEYBOARD),
    MenuNode::Leaf(TRACKPAD),
    MenuNode::Leaf(OVERLAY_PC),
    MenuNode::Leaf(AUTOFADE),
    MenuNode::Leaf(TRIGGERS),
];

/// The root stays short on purpose (Halyard's rule): what you come looking for mid-game, and
/// nothing else. Everything rarer is one level down.
const ROOT: &[MenuNode<PauseId>] = &[
    MenuNode::Leaf(CONTINUE),
    MenuNode::Submenu(VIDEO, VIDEO_PAGE),
    MenuNode::Submenu(PANEL, PANEL_PAGE),
    MenuNode::Submenu(CONTROLS, CONTROLS_PAGE),
    MenuNode::Leaf(QUIT),
];

pub(crate) fn tree() -> &'static [MenuNode<PauseId>] {
    ROOT
}

/// What the paint needs to draw current values. Read fresh each frame - the whole point of a
/// value column is that it cannot lie about a toggle's state.
pub(crate) struct PauseView {
    pub stats_on: bool,
    pub keyboard_open: bool,
    pub trackpad_on: bool,
}

/// How a row renders its right-hand column.
enum RowValue {
    None,
    Chevron,          // submenu
    OnOff(bool),      // toggle
    Text(String),     // stepper's current value
}

fn row_value(id: PauseId, view: &PauseView) -> RowValue {
    match id {
        VIDEO | PANEL | CONTROLS => RowValue::Chevron,
        CONTINUE | QUIT | TRIGGERS => RowValue::None,
        STATS_TOGGLE => RowValue::OnOff(view.stats_on),
        FPS_CHART => RowValue::OnOff(prefs::hud_fps_chart()),
        BITRATE_CHART => RowValue::OnOff(prefs::hud_bitrate_chart()),
        TIMER => RowValue::OnOff(prefs::session_timer_enabled()),
        KEYBOARD => RowValue::OnOff(view.keyboard_open),
        TRACKPAD => RowValue::OnOff(view.trackpad_on),
        OVERLAY_PC => RowValue::OnOff(prefs::pc_overlay_enabled()),
        AUTOFADE => RowValue::OnOff(prefs::overlay_autofade()),
        BITRATE => RowValue::Text(bitrate_label()),
        OPACITY => RowValue::Text(format!("{} %", prefs::hud_opacity_percent())),
        REFRESH => RowValue::Text(format!("{} ms", prefs::hud_refresh_ms())),
        PROFILE => RowValue::Text(profile_label()),
    }
}

fn bitrate_label() -> String {
    match prefs::max_bitrate_kbps() {
        0 => "Auto".to_owned(),
        kbps => format!("{} Mbps", kbps / 1000),
    }
}

fn profile_label() -> String {
    match prefs::control_profile() {
        prefs::ControlProfile::Game => "JUEGO".to_owned(),
        prefs::ControlProfile::Desktop => "PC".to_owned(),
    }
}

fn label_key(id: PauseId) -> &'static str {
    match id {
        CONTINUE => "menu-continue",
        VIDEO => "menu-video",
        PANEL => "menu-panel",
        CONTROLS => "menu-controls",
        QUIT => "menu-quit",
        BITRATE => "menu-bitrate",
        STATS_TOGGLE => "menu-stats",
        OPACITY => "menu-opacity",
        REFRESH => "menu-refresh",
        FPS_CHART => "menu-fps-chart",
        BITRATE_CHART => "menu-bitrate-chart",
        TIMER => "menu-timer",
        PROFILE => "menu-profile",
        KEYBOARD => "menu-keyboard",
        TRACKPAD => "menu-trackpad",
        OVERLAY_PC => "menu-overlay",
        AUTOFADE => "menu-autofade",
        TRIGGERS => "menu-triggers",
    }
}

/// Activating a leaf: the commands to run, and whether the menu closes. Toggles and steppers
/// keep the menu open - their whole value is watching the HUD or the pill react behind the
/// backdrop; closing on every press would make a two-change setting two menus deep.
pub(crate) fn activate(id: PauseId) -> (Vec<AppCommand>, bool) {
    match id {
        CONTINUE => (vec![], true),
        QUIT => (vec![AppCommand::ToggleConfirmExit], true),
        TRIGGERS => (vec![AppCommand::ToggleControlsModal], true),
        BITRATE => (vec![AppCommand::MenuBitrateStep(1)], false),
        OPACITY => (vec![AppCommand::MenuOpacityStep(1)], false),
        REFRESH => (vec![AppCommand::MenuRefreshStep(1)], false),
        STATS_TOGGLE => (vec![AppCommand::ToggleStreamStats], false),
        FPS_CHART => (vec![AppCommand::ToggleHudFpsChart], false),
        BITRATE_CHART => (vec![AppCommand::ToggleHudBitrateChart], false),
        TIMER => (vec![AppCommand::ToggleSessionTimer], false),
        KEYBOARD => (vec![AppCommand::ToggleKeyboard], false),
        TRACKPAD => (vec![AppCommand::ToggleMouseTrackpad], false),
        OVERLAY_PC => (vec![AppCommand::TogglePcOverlay], false),
        AUTOFADE => (vec![AppCommand::ToggleOverlayAutofade], false),
        PROFILE => {
            // Cycles, like the pill's halves but from the menu; SelectControlProfile is a
            // no-op on the same profile, so cycling is the one honest verb here.
            let next = match prefs::control_profile() {
                prefs::ControlProfile::Game => prefs::ControlProfile::Desktop,
                prefs::ControlProfile::Desktop => prefs::ControlProfile::Game,
            };
            (vec![AppCommand::SelectControlProfile(next)], false)
        }
        VIDEO | PANEL | CONTROLS => (vec![], false),
    }
}

/// Left/Right on the cursor row, for rows that step. `None` for rows that ignore it.
pub(crate) fn step_command(id: PauseId, dir: i32) -> Option<AppCommand> {
    match id {
        BITRATE => Some(AppCommand::MenuBitrateStep(dir)),
        OPACITY => Some(AppCommand::MenuOpacityStep(dir)),
        REFRESH => Some(AppCommand::MenuRefreshStep(dir)),
        _ => None,
    }
}

/// Paints the menu as a centred card. Rows are tappable (each reserves its rect, so a tap
/// lands here and never in the game) and report `MenuActivateAt`; the handler owns all state
/// mutation, paint never does.
pub(crate) fn paint(
    ctx: &egui::Context,
    i18n: &I18n,
    menu: &MenuState,
    view: &PauseView,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();

    egui::Modal::new(egui::Id::new("pause_menu"))
        .backdrop_color(egui::Color32::from_black_alpha(150))
        .frame(
            egui::Frame::default()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0_f32, BORDER))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(10, 8)),
        )
        .show(ctx, |ui| {
            ui.set_width(252.0);
            ui.set_min_height(300.0);

            // Breadcrumb: root keeps its title; a pushed page shows the way in.
            let crumb = menu
                .breadcrumb(tree())
                .last()
                .copied()
                .map_or_else(|| i18n.text("menu-title"), |parent| i18n.text(label_key(parent)));
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(crumb.as_ref())
                        .size(13.0)
                        .strong()
                        .color(Color32::WHITE),
                );
            });
            ui.add_space(2.0);
            ui.separator();
            ui.add_space(2.0);

            let row_font = egui::FontId::proportional(11.5);
            let value_font = egui::FontId::monospace(10.0);
            let page: Vec<PauseId> = menu
                .page(tree())
                .iter()
                .map(|node| match node {
                    MenuNode::Leaf(id) | MenuNode::Submenu(id, _) => *id,
                })
                .collect();

            for (index, &id) in page.iter().enumerate() {
                let focused = menu.cursor() == index;
                let height = 24.0;
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
                reserve_stream_touch(ui.ctx(), response.rect);
                if response.clicked() {
                    commands.push(AppCommand::MenuActivateAt(index));
                }
                let painter = ui.painter();
                if focused {
                    painter.rect_filled(rect, 4.0, BG_RAISED);
                    // A left accent bar, not a full outline: visible at a glance, without
                    // boxing every row into its own cell.
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(rect.min.x, rect.min.y + 3.0),
                            egui::pos2(rect.min.x + 3.0, rect.max.y - 3.0),
                        ),
                        2.0,
                        ACCENT,
                    );
                }
                painter.text(
                    egui::pos2(rect.min.x + 10.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    i18n.text(label_key(id)),
                    row_font.clone(),
                    if focused {
                        Color32::WHITE
                    } else {
                        TEXT_DIM
                    },
                );
                match row_value(id, view) {
                    RowValue::None => {}
                    RowValue::Chevron => {
                        painter.text(
                            egui::pos2(rect.max.x - 10.0, rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            "›",
                            row_font.clone(),
                            TEXT_DIM,
                        );
                    }
                    RowValue::OnOff(on) => {
                        painter.text(
                            egui::pos2(rect.max.x - 10.0, rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            if on { "●" } else { "○" },
                            row_font.clone(),
                            if on { ACCENT } else { TEXT_DIM },
                        );
                    }
                    RowValue::Text(value) => {
                        painter.text(
                            egui::pos2(rect.max.x - 10.0, rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            value,
                            value_font.clone(),
                            Color32::WHITE,
                        );
                    }
                }
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(i18n.text("menu-footer").as_ref())
                        .size(9.0)
                        .color(TEXT_DIM),
                );
            });
        });

    commands
}
