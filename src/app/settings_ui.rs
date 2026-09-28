//! The settings modal: tabs, rows, chip choices, and the L2/R2 + L3/R3 panel. Split out of
//! `ui.rs` (v0.7 F5) without behavioural change.

use super::catalog_ui::{RegionsView, SettingsView};
use super::stream_ui::{front_stick_zones_diagram, paint_stream_icon, rear_touch_diagram, StreamIcon};
use super::theme::{ACCENT, BG_DEEP, BG_PANEL, BG_RAISED, BORDER, DANGER, TEXT_DIM};
use crate::gfn::auth::GfnUser;
use crate::i18n::I18n;
use crate::input::AppCommand;

pub(crate) const SETTINGS_MODAL_W: f32 = 520.0;
pub(crate) const SETTINGS_MODAL_H: f32 = 360.0;
pub(crate) const SETTINGS_BODY_H: f32 = 268.0;

///
pub(crate) fn settings_modal(
    ui: &mut egui::Ui,
    i18n: &I18n,
    user: &GfnUser,
    regions: &RegionsView<'_>,
    settings: SettingsView,
) -> Vec<AppCommand> {
    use crate::app::settings_menu::SettingsTab;

    let mut commands = Vec::new();
    let gear = ui.add_sized(
        [34.0, 30.0],
        egui::Button::new(egui::RichText::new("\u{2699}").size(15.0)).fill(BG_RAISED),
    );
    if gear.clicked() {
        commands.push(AppCommand::OpenSettings);
    }
    if !settings.open {
        return commands;
    }

    let modal = egui::Modal::new(egui::Id::new("settings_modal"))
        .backdrop_color(egui::Color32::from_black_alpha(180))
        .frame(
            egui::Frame::default()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0_f32, BORDER))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(14, 12)),
        )
        .show(ui.ctx(), |ui| {
            let mut close_requested = false;
            ui.set_width(SETTINGS_MODAL_W);
            ui.set_min_height(SETTINGS_MODAL_H);
            ui.set_max_height(SETTINGS_MODAL_H);

            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new(i18n.text("settings-heading").as_ref()).size(15.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_sized(
                            [30.0, 26.0],
                            egui::Button::new(egui::RichText::new("X").size(14.0).strong()),
                        )
                        .clicked()
                    {
                        close_requested = true;
                    }
                });
            });
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for tab in SettingsTab::ALL {
                    if let Some(cmd) = settings_tab_button(ui, i18n, tab, settings.tab) {
                        commands.push(cmd);
                    }
                }
            });
            ui.add_space(4.0);
            ui.separator();

            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), SETTINGS_BODY_H),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("settings_content")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            if let Some(email) = &user.email {
                                if settings.tab == SettingsTab::Account {
                                    ui.label(
                                        egui::RichText::new(email)
                                            .size(12.0)
                                            .color(egui::Color32::WHITE),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "OpenNOW-Vita {}",
                                            env!("CARGO_PKG_VERSION")
                                        ))
                                        .size(10.0)
                                        .color(TEXT_DIM),
                                    );
                                    ui.add_space(6.0);
                                    ui.separator();
                                }
                            }

                            if settings.tab == SettingsTab::Controls {
                                for cmd in controls_settings_panel(ui, i18n, settings, regions) {
                                    commands.push(cmd);
                                }
                            } else if settings.tab == SettingsTab::Account {
                                ui.label(egui::RichText::new("OpenNOW no captura ni envía evidencia.").size(11.0).color(TEXT_DIM));
                                ui.label(egui::RichText::new("El diagnóstico del sistema vive en Vita Lab.").size(11.0).color(TEXT_DIM));
                            } else {
                                let row_count = settings.tab.row_count();
                                for row in 0..row_count {
                                    let Some(info) =
                                        crate::app::settings_menu::row_info(settings.tab, row)
                                    else {
                                        continue;
                                    };
                                    let focused = settings.focus == row;
                                    let expanded = settings.expanded == Some(row);
                                    if let Some(cmd) = settings_item(
                                        ui,
                                        i18n,
                                        settings.tab,
                                        row,
                                        &info,
                                        focused,
                                        expanded,
                                        settings.option_focus,
                                        regions,
                                        false,
                                    ) {
                                        commands.push(cmd);
                                    }
                                    ui.separator();
                                }
                            }
                        });
                },
            );

            close_requested
        });

    if modal.inner || modal.should_close() {
        commands.push(AppCommand::CloseSettings);
    }
    commands
}

pub(crate) fn controls_settings_panel(
    ui: &mut egui::Ui,
    i18n: &I18n,
    settings: SettingsView,
    regions: &RegionsView<'_>,
) -> Vec<AppCommand> {
    use crate::app::settings_menu::SettingsTab;

    let mut commands = Vec::new();
    let tab = SettingsTab::Controls;
    let gap = 10.0;
    let half = ((ui.available_width() - gap) / 2.0).max(120.0);

    ui.add_space(2.0);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(half);
            if let Some(info) = crate::app::settings_menu::row_info(tab, 0) {
                rear_touch_diagram(ui, 92.0, None);
                ui.add_space(4.0);
                if let Some(cmd) =
                    settings_chip_choice(ui, i18n, tab, 0, &info, settings.focus == 0)
                {
                    commands.push(cmd);
                }
            }
        });
        ui.add_space(gap);
        ui.vertical(|ui| {
            ui.set_width(half);
            if let Some(info) = crate::app::settings_menu::row_info(tab, 1) {
                front_stick_zones_diagram(ui, 92.0);
                ui.add_space(4.0);
                if let Some(cmd) =
                    settings_chip_choice(ui, i18n, tab, 1, &info, settings.focus == 1)
                {
                    commands.push(cmd);
                }
            }
        });
    });

    ui.add_space(6.0);
    ui.separator();

    for row in 2..tab.row_count() {
        let Some(info) = crate::app::settings_menu::row_info(tab, row) else {
            continue;
        };
        if let Some(cmd) = settings_item(
            ui,
            i18n,
            tab,
            row,
            &info,
            settings.focus == row,
            settings.expanded == Some(row),
            settings.option_focus,
            regions,
            false,
        ) {
            commands.push(cmd);
        }
        ui.separator();
    }

    commands
}

pub(crate) fn settings_chip_choice(
    ui: &mut egui::Ui,
    i18n: &I18n,
    tab: crate::app::settings_menu::SettingsTab,
    row: usize,
    info: &crate::app::settings_menu::RowInfo,
    focused: bool,
) -> Option<AppCommand> {
    let mut command = None;
    ui.add_space(4.0);
    let current = crate::app::settings_menu::current_option_index(tab, row, &[], i18n.locale());
    let count = crate::app::settings_menu::option_count(tab, row, 0);

    let block = ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(i18n.text(info.label_key).as_ref())
                .size(12.5)
                .strong(),
        );
        if let Some(desc_key) = info.desc_key {
            ui.label(
                egui::RichText::new(i18n.text(desc_key).as_ref())
                    .size(9.5)
                    .color(TEXT_DIM),
            );
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for option in 0..count {
                let label = crate::app::settings_menu::option_label(tab, row, option, i18n, &[]);
                let selected = option == current;
                let fill = if selected {
                    ACCENT.gamma_multiply(0.35)
                } else {
                    BG_RAISED
                };
                let text = egui::RichText::new(label).size(11.0).color(if selected {
                    egui::Color32::WHITE
                } else {
                    TEXT_DIM
                });
                if ui
                    .add(
                        egui::Button::new(text)
                            .fill(fill)
                            .min_size(egui::vec2(0.0, 28.0)),
                    )
                    .clicked()
                {
                    command = Some(AppCommand::ChooseSettingsOption(row, option));
                }
            }
        });
    });
    if focused {
        ui.painter().rect_stroke(
            block.response.rect.expand(3.0),
            4.0,
            egui::Stroke::new(1.5_f32, ACCENT),
            egui::StrokeKind::Outside,
        );
    }

    command
}
pub(crate) fn settings_tab_button(
    ui: &mut egui::Ui,
    i18n: &I18n,
    tab: crate::app::settings_menu::SettingsTab,
    current: crate::app::settings_menu::SettingsTab,
) -> Option<AppCommand> {
    use crate::app::settings_menu::SettingsTab;

    let icon = match tab {
        SettingsTab::Stream => StreamIcon::Globe,
        SettingsTab::Controls => StreamIcon::Controls,
        SettingsTab::App => StreamIcon::Monitor,
        SettingsTab::Account => StreamIcon::Person,
    };
    let active = tab == current;
    let label = i18n.text(tab.label_key());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(122.0, 28.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if active {
            painter.rect_filled(rect, 5.0, ACCENT.gamma_multiply(0.14));
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(rect.min.x + 8.0, rect.max.y - 3.0),
                    egui::vec2(rect.width() - 16.0, 2.0),
                ),
                1.0,
                ACCENT,
            );
        }
        let icon_rect = egui::Rect::from_center_size(
            egui::pos2(rect.min.x + 16.0, rect.center().y),
            egui::vec2(13.0, 13.0),
        );
        paint_stream_icon(
            painter,
            icon_rect,
            icon,
            if active { ACCENT } else { TEXT_DIM },
        );
        painter.text(
            egui::pos2(rect.min.x + 30.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label.as_ref(),
            egui::FontId::proportional(12.0),
            if active {
                egui::Color32::WHITE
            } else {
                TEXT_DIM
            },
        );
    }
    response
        .clicked()
        .then_some(AppCommand::SetSettingsTab(tab))
}

pub(crate) fn settings_item(
    ui: &mut egui::Ui,
    i18n: &I18n,
    tab: crate::app::settings_menu::SettingsTab,
    row: usize,
    info: &crate::app::settings_menu::RowInfo,
    focused: bool,
    expanded: bool,
    option_focus: usize,
    regions: &RegionsView<'_>,
    show_touch_diagrams: bool,
) -> Option<AppCommand> {
    use crate::app::settings_menu::RowKind;

    let mut command = None;
    ui.add_space(4.0);

    if matches!(info.kind, RowKind::Region)
        && regions.list.is_empty()
        && !regions.busy
        && regions.error.is_none()
    {
        command = Some(AppCommand::LoadRegions);
    }

    let control_width = if matches!(info.kind, RowKind::Region) {
        200.0
    } else {
        160.0
    };
    let header_response = ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - control_width).max(80.0));
            ui.label(
                egui::RichText::new(i18n.text(info.label_key).as_ref())
                    .size(12.5)
                    .strong(),
            );
            if let Some(desc_key) = info.desc_key {
                ui.label(
                    egui::RichText::new(i18n.text(desc_key).as_ref())
                        .size(9.5)
                        .color(TEXT_DIM),
                );
            }
        });
        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| match info.kind {
                RowKind::Toggle(on) => {
                    let game_only = info.label_key == "settings-game-profile-heading";
                    let can_toggle =
                        !game_only || crate::gfn::stream_prefs::active_game().is_some();
                    let mut value = on;
                    let response =
                        ui.add_enabled(can_toggle, egui::Checkbox::without_text(&mut value));
                    if response.changed() && can_toggle {
                        command = Some(AppCommand::ChooseSettingsOption(row, 0));
                    }
                }
                RowKind::Choice | RowKind::Region => {
                    let summary = if info.kind == RowKind::Region && regions.busy {
                        i18n.text(if regions.measuring {
                            "settings-region-measuring"
                        } else {
                            "settings-region-loading"
                        })
                        .to_string()
                    } else {
                        crate::app::settings_menu::current_summary(
                            tab,
                            row,
                            i18n,
                            regions.list,
                            i18n.locale(),
                        )
                    };
                    let button =
                        egui::Button::new(egui::RichText::new(format!("{summary}   ")).size(11.0))
                            .fill(BG_RAISED)
                            .min_size(egui::vec2(150.0, 28.0));
                    let button_response = ui.add(button);
                    if button_response.clicked() {
                        command = Some(AppCommand::ExpandSettingsRow(if expanded {
                            None
                        } else {
                            Some(row)
                        }));
                    }
                    let chevron_rect = egui::Rect::from_center_size(
                        egui::pos2(
                            button_response.rect.max.x - 14.0,
                            button_response.rect.center().y,
                        ),
                        egui::vec2(12.0, 12.0),
                    );
                    paint_stream_icon(
                        ui.painter(),
                        chevron_rect,
                        StreamIcon::ChevronDown,
                        TEXT_DIM,
                    );
                    if info.kind == RowKind::Region {
                        let test_btn =
                            ui.add_sized([28.0, 28.0], egui::Button::new("").fill(BG_RAISED));
                        if test_btn.clicked() {
                            command = Some(if regions.list.is_empty() {
                                AppCommand::LoadRegions
                            } else {
                                AppCommand::TestRegionLatency
                            });
                        }
                        paint_stream_icon(
                            ui.painter(),
                            test_btn.rect.shrink(7.0),
                            StreamIcon::Signal,
                            ACCENT,
                        );
                    }
                }
            },
        );
    });

    if focused {
        ui.painter().rect_stroke(
            header_response.response.rect.expand(3.0),
            4.0,
            egui::Stroke::new(1.5_f32, ACCENT),
            egui::StrokeKind::Outside,
        );
    }

    if let RowKind::Region = info.kind {
        if let Some(error) = regions.error {
            ui.label(egui::RichText::new(error).size(10.0).color(DANGER));
        }
    }

    if expanded {
        let option_count = crate::app::settings_menu::option_count(tab, row, regions.list.len());
        egui::Frame::default()
            .fill(BG_DEEP)
            .stroke(egui::Stroke::new(1.0_f32, BORDER))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
                let current_index = crate::app::settings_menu::current_option_index(
                    tab,
                    row,
                    regions.list,
                    i18n.locale(),
                );
                for option in 0..option_count {
                    let label = crate::app::settings_menu::option_label(
                        tab,
                        row,
                        option,
                        i18n,
                        regions.list,
                    );
                    let is_current = option == current_index;
                    let is_focused = option == option_focus;
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 26.0),
                        egui::Sense::click(),
                    );
                    if ui.is_rect_visible(rect) {
                        if is_focused {
                            ui.painter()
                                .rect_filled(rect, 4.0, ACCENT.gamma_multiply(0.18));
                        }
                        ui.painter().text(
                            egui::pos2(rect.min.x + 8.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &label,
                            egui::FontId::proportional(11.5),
                            if is_current {
                                ACCENT
                            } else {
                                egui::Color32::WHITE
                            },
                        );
                        if is_current {
                            let check_rect = egui::Rect::from_center_size(
                                egui::pos2(rect.max.x - 14.0, rect.center().y),
                                egui::vec2(12.0, 12.0),
                            );
                            paint_stream_icon(ui.painter(), check_rect, StreamIcon::Check, ACCENT);
                        }
                        if info.kind == RowKind::Region && option > 0 {
                            if let Some(best) = regions.list.iter().filter_map(|r| r.ping_ms).min()
                            {
                                if regions.list.get(option - 1).and_then(|r| r.ping_ms)
                                    == Some(best)
                                {
                                    ui.painter().text(
                                        egui::pos2(rect.max.x - 46.0, rect.center().y),
                                        egui::Align2::RIGHT_CENTER,
                                        i18n.text("settings-region-best"),
                                        egui::FontId::proportional(8.5),
                                        ACCENT,
                                    );
                                }
                            }
                        }
                    }
                    if response.clicked() {
                        command = Some(AppCommand::ChooseSettingsOption(row, option));
                    }
                }
            });
    }

    if show_touch_diagrams {
        if info.label_key == "settings-rear-touch-mode-heading" {
            ui.add_space(8.0);
            rear_touch_diagram(ui, 120.0, None);
            ui.add_space(6.0);
        }
        if info.label_key == "settings-stick-zones-heading" {
            ui.add_space(8.0);
            front_stick_zones_diagram(ui, 120.0);
            ui.add_space(6.0);
        }
    }

    command
}

/// One setting: a heading with its choices laid out across the row rather than stacked.
///
/// Horizontal is what keeps the modal short - stacked, four settings came to twenty-odd rows.
pub(crate) fn settings_row<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    i18n: &I18n,
    heading_key: &'static str,
    candidates: impl Iterator<Item = T>,
    current: T,
    label: impl Fn(T) -> String,
) -> Option<T> {
    let mut chosen = None;
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(i18n.text(heading_key).as_ref())
            .size(10.0)
            .color(TEXT_DIM),
    );
    ui.horizontal_wrapped(|ui| {
        for candidate in candidates {
            if ui
                .selectable_label(candidate == current, label(candidate))
                .clicked()
            {
                chosen = Some(candidate);
            }
        }
    });
    chosen
}

