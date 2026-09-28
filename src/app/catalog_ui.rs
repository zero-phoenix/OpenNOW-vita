//! The library screens: catalog, server picker, sort/filter pickers, game detail, and the
//! session-launch overlay family. Split out of `ui.rs` (v0.7 F5) without behavioural change.

use super::settings_ui::settings_modal;
use super::stream_ui::{paint_heart, paint_stream_icon, StreamIcon};
use super::ui::{cart_frame, PsButton, CART_ASPECT, CART_WINDOW_X, CART_WINDOW_Y};
use super::ui::{geforce_logo, ps_button, text1, text2};
use super::{App, CatalogFilter, CatalogSort};
use super::theme::{ACCENT, BG_DEEP, BG_PANEL, BG_RAISED, BORDER, DANGER, TEXT_DIM};
use crate::gfn::auth::GfnUser;
use crate::gfn::catalog::GameSummary;
use crate::gfn::covers::{CoverSize, CoverSnapshot, CoverStore};
use crate::i18n::I18n;
use crate::input::AppCommand;
use reqwest::Client;
use std::sync::Arc;

pub(crate) const LIST_WIDTH: f32 = 250.0;
/// One list row, sized for a fingertip rather than a mouse cursor.
pub(crate) const ROW_HEIGHT: f32 = 30.0;
pub(crate) fn selected_game<'a>(
    games: &'a [GameSummary],
    filtered_indices: &[usize],
    selected: usize,
) -> Option<&'a GameSummary> {
    games.get(*filtered_indices.get(selected)?)
}

pub(crate) struct CatalogView<'a> {
    pub(crate) user: &'a GfnUser,
    pub(crate) games: &'a [GameSummary],
    pub(crate) selected: usize,
    pub(crate) filtered_indices: &'a [usize],
    pub(crate) search_query: &'a str,
    pub(crate) search_requested: bool,
    pub(crate) covers: &'a CoverStore,
    pub(crate) http_client: &'a Client,
    pub(crate) status_note: Option<&'a str>,
    pub(crate) sort: CatalogSort,
    pub(crate) filter: CatalogFilter,
    /// `pageInfo.totalCount` from the server - generally far more than we page in, so the header
    /// shows "N of M" to explain why the list stops where it does.
    pub(crate) total_count: Option<usize>,
    /// A background page fetch is in flight, i.e.
    pub(crate) loading_more: bool,
    /// Starred app ids. Held by the app rather than re-read here, because this is rebuilt on every
    /// repaint and the list lives on the memory card.
    pub(crate) favorites: &'a std::collections::BTreeSet<String>,
    pub(crate) regions: RegionsView<'a>,
    pub(crate) settings: SettingsView,
}

#[derive(Clone)]
pub(crate) struct SettingsView {
    pub(crate) open: bool,
    pub(crate) tab: crate::app::settings_menu::SettingsTab,
    pub(crate) focus: usize,
    pub(crate) expanded: Option<usize>,
    pub(crate) option_focus: usize,
}

impl SettingsView {
    pub(crate) fn from_app(app: &App) -> Self {
        Self {
            open: app.settings_open,
            tab: app.settings_tab,
            focus: app.settings_focus,
            expanded: app.settings_expanded,
            option_focus: app.settings_option_focus,
        }
    }
}

pub(crate) struct RegionsView<'a> {
    pub(crate) list: &'a [crate::gfn::regions::StreamRegion],
    pub(crate) busy: bool,
    pub(crate) measuring: bool,
    pub(crate) error: Option<&'a str>,
}

impl<'a> RegionsView<'a> {
    pub(crate) fn from_app(app: &'a App) -> Self {
        Self {
            list: &app.regions,
            busy: app.is_loading_regions(),
            measuring: app.regions_measuring,
            error: app.regions_error.as_deref(),
        }
    }
}
pub(crate) fn catalog_screen(ctx: &egui::Context, i18n: &I18n, view: &CatalogView<'_>) -> Vec<AppCommand> {
    let mut commands = Vec::new();

    egui::TopBottomPanel::top("catalog_header")
        .frame(
            egui::Frame::NONE
                .fill(BG_PANEL)
                .inner_margin(egui::Margin::symmetric(12, 8)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                match geforce_logo(ctx) {
                    Some(logo) => {
                        let size = logo.size_vec2();
                        let height = 24.0;
                        let width = height * size.x / size.y.max(1.0);
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                        ui.painter().image(
                            logo.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                    None => {
                        ui.label(
                            egui::RichText::new(i18n.text("catalog-library-title").as_ref())
                                .strong()
                                .size(20.0)
                                .color(ACCENT),
                        );
                    }
                }
                if let Some(total) = view.total_count {
                    ui.label(
                        egui::RichText::new("/")
                            .size(15.0)
                            .color(BORDER.gamma_multiply(3.0)),
                    );
                    let key = if view.loading_more {
                        "catalog-count-loading"
                    } else {
                        "catalog-count"
                    };
                    ui.label(
                        egui::RichText::new(
                            text2(
                                i18n,
                                key,
                                ("shown", view.filtered_indices.len()),
                                ("total", total),
                            )
                            .as_ref(),
                        )
                        .size(11.0)
                        .color(TEXT_DIM),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 15.0, ACCENT);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        view.user
                            .display_name
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_uppercase()
                            .to_string(),
                        egui::FontId::proportional(17.0),
                        BG_PANEL,
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(&view.user.display_name)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                    let (dot, _) =
                        ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot.center(), 4.0, ACCENT);
                    ui.add_space(10.0);
                    commands.extend(settings_modal(
                        ui,
                        i18n,
                        view.user,
                        &view.regions,
                        view.settings.clone(),
                    ));
                    ui.add_space(6.0);
                    if let Some(cmd) = sort_picker(ui, i18n, view.sort, view.games) {
                        commands.push(cmd);
                    }
                    ui.add_space(6.0);
                    if let Some(cmd) = filter_picker(ui, i18n, view.filter) {
                        commands.push(cmd);
                    }
                });
            });
        });

    egui::TopBottomPanel::bottom("catalog_footer")
        .frame(
            egui::Frame::NONE
                .fill(BG_PANEL)
                .inner_margin(egui::Margin::symmetric(12, 6)),
        )
        .show(ctx, |ui| {
            if let Some(note) = view.status_note {
                ui.label(
                    egui::RichText::new(note)
                        .italics()
                        .size(11.0)
                        .color(TEXT_DIM),
                );
            }
            button_hint(ui, &i18n.text("catalog-footer-hint"), 11.0, TEXT_DIM, false);
        });

    egui::SidePanel::left("catalog_list")
        .exact_width(LIST_WIDTH)
        .resizable(false)
        .frame(
            egui::Frame::NONE
                .fill(BG_DEEP)
                .inner_margin(egui::Margin::symmetric(8, 8)),
        )
        .show(ctx, |ui| {
            commands.extend(title_list(ui, i18n, view));
        });

    egui::CentralPanel::default()
        .frame(
            egui::Frame::NONE
                .fill(BG_DEEP)
                .inner_margin(egui::Margin::symmetric(12, 10)),
        )
        .show(ctx, |ui| {
            commands.extend(detail_panel(ctx, ui, i18n, view));
        });

    commands
}

/// First-run explainer for the buttons the Vita does not physically have.
pub(crate) fn ping_color(ms: u32) -> egui::Color32 {
    match ms {
        0..=40 => ACCENT,
        41..=80 => egui::Color32::from_rgb(0xe0, 0xa8, 0x30),
        _ => DANGER,
    }
}

pub(crate) fn queue_color(position: u32) -> egui::Color32 {
    match position {
        0..=9 => ACCENT,
        10..=24 => egui::Color32::from_rgb(0xe0, 0xa8, 0x30),
        _ => DANGER,
    }
}

pub(crate) fn format_wait(seconds: u64) -> String {
    if seconds >= 60 {
        format!("~{}m", seconds / 60)
    } else {
        format!("~{seconds}s")
    }
}

pub(crate) fn server_picker_modal(
    ctx: &egui::Context,
    i18n: &I18n,
    app: &App,
    game: Option<&GameSummary>,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    let regions = &app.regions;
    let queue = &app.queue_stats;
    let focus = app.server_picker_focus.index();

    let queue_for = |url: &str| {
        crate::gfn::queue_stats::server_code_from_url(url)
            .and_then(|code| queue.get(&code).copied())
    };
    let best_index = regions
        .iter()
        .enumerate()
        .filter_map(|(index, region)| region.ping_ms.map(|ping| (index, region, ping)))
        .min_by_key(|(_, region, ping)| {
            let depth = queue_for(&region.url).map_or(u32::MAX, |r| r.queue_position);
            (*ping, depth)
        })
        .map(|(index, _, _)| index);
    let closest_index = regions
        .iter()
        .enumerate()
        .filter_map(|(index, region)| region.ping_ms.map(|ping| (index, ping)))
        .min_by_key(|(_, ping)| *ping)
        .map(|(index, _)| index);

    egui::Modal::new(egui::Id::new("server_picker_modal"))
        .backdrop_color(egui::Color32::from_black_alpha(190))
        .frame(
            egui::Frame::default()
                .fill(BG_PANEL)
                .stroke(egui::Stroke::new(1.0_f32, BORDER))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(14, 12)),
        )
        .show(ctx, |ui| {
            ui.set_width(520.0);

            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new(i18n.text("server-picker-heading").as_ref()).size(15.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_sized(
                            [30.0, 26.0],
                            egui::Button::new(egui::RichText::new("X").size(14.0).strong()),
                        )
                        .clicked()
                    {
                        commands.push(AppCommand::CloseServerPicker);
                    }
                });
            });
            if let Some(game) = game {
                ui.label(egui::RichText::new(&game.title).size(10.5).color(TEXT_DIM));
            }
            ui.separator();

            if app.is_loading_regions() {
                ui.label(
                    egui::RichText::new(
                        i18n.text(if app.regions_measuring {
                            "settings-region-measuring"
                        } else {
                            "settings-region-loading"
                        })
                        .as_ref(),
                    )
                    .size(10.0)
                    .color(TEXT_DIM),
                );
            }
            if app.is_loading_queue_stats() {
                ui.label(
                    egui::RichText::new(i18n.text("server-picker-queue-loading").as_ref())
                        .size(10.0)
                        .color(TEXT_DIM),
                );
            }

            egui::ScrollArea::vertical()
                .id_salt("server_picker_list")
                .max_height(200.0)
                .show(ui, |ui| {
                    let auto_detail =
                        best_index
                            .and_then(|index| regions.get(index))
                            .map(|region| match region.ping_ms {
                                Some(ms) => format!("{} · {ms} ms", region.name),
                                None => region.name.clone(),
                            });
                    if server_picker_row(
                        ui,
                        &i18n.text("settings-region-auto"),
                        auto_detail.as_deref(),
                        None,
                        None,
                        focus == 0,
                        None,
                    ) {
                        commands.push(AppCommand::FocusServerPicker(0));
                        commands.push(AppCommand::LaunchOnServer(String::new()));
                    }

                    for (index, region) in regions.iter().enumerate() {
                        let row = index + 1;
                        let badge = if Some(index) == best_index {
                            Some(i18n.text("server-picker-auto-badge"))
                        } else if Some(index) == closest_index {
                            Some(i18n.text("server-picker-closest-badge"))
                        } else {
                            None
                        };
                        if server_picker_row(
                            ui,
                            &region.name,
                            None,
                            region.ping_ms,
                            queue_for(&region.url),
                            focus == row,
                            badge.as_deref(),
                        ) {
                            commands.push(AppCommand::FocusServerPicker(row));
                            commands.push(AppCommand::LaunchOnServer(region.url.clone()));
                        }
                    }
                });

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(i18n.text("server-picker-hint").as_ref())
                    .size(9.5)
                    .color(TEXT_DIM),
            );
            ui.separator();
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let launch = egui::Button::new(
                        egui::RichText::new(i18n.text("server-picker-launch").as_ref())
                            .size(12.0)
                            .strong()
                            .color(BG_DEEP),
                    )
                    .fill(ACCENT)
                    .min_size(egui::vec2(96.0, 28.0));
                    if ui.add(launch).clicked() {
                        commands.push(AppCommand::LaunchOnServer(match focus.checked_sub(1) {
                            None => String::new(),
                            Some(index) => regions
                                .get(index)
                                .map(|region| region.url.clone())
                                .unwrap_or_default(),
                        }));
                    }
                    ui.add_space(6.0);
                    let cancel = egui::Button::new(
                        egui::RichText::new(i18n.text("server-picker-cancel").as_ref()).size(12.0),
                    )
                    .fill(BG_RAISED)
                    .min_size(egui::vec2(84.0, 28.0));
                    if ui.add(cancel).clicked() {
                        commands.push(AppCommand::CloseServerPicker);
                    }
                    ui.add_space(6.0);
                    let refresh = ui.add_sized([28.0, 28.0], egui::Button::new("").fill(BG_RAISED));
                    if refresh.clicked() {
                        commands.push(AppCommand::LoadQueueStats);
                        commands.push(AppCommand::TestRegionLatency);
                    }
                    paint_stream_icon(
                        ui.painter(),
                        refresh.rect.shrink(7.0),
                        StreamIcon::Signal,
                        ACCENT,
                    );
                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(i18n.text("server-picker-powered-by").as_ref())
                                .size(9.5)
                                .color(TEXT_DIM),
                        )
                        .truncate(),
                    );
                });
            });
        });

    commands
}

pub(crate) fn server_picker_row(
    ui: &mut egui::Ui,
    name: &str,
    detail: Option<&str>,
    ping_ms: Option<u32>,
    queue: Option<crate::gfn::queue_stats::QueueReading>,
    focused: bool,
    badge: Option<&str>,
) -> bool {
    let height = if detail.is_some() { 34.0 } else { 26.0 };
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click(),
    );
    if !ui.is_rect_visible(rect) {
        return response.clicked();
    }

    let painter = ui.painter();
    if focused {
        painter.rect_filled(rect, 5.0, ACCENT.gamma_multiply(0.16));
        painter.rect_stroke(
            rect,
            5.0,
            egui::Stroke::new(1.0_f32, ACCENT),
            egui::StrokeKind::Inside,
        );
    }

    let text_x = rect.min.x + 8.0;
    let name_y = if detail.is_some() {
        rect.min.y + 11.0
    } else {
        rect.center().y
    };
    let name_end = painter.text(
        egui::pos2(text_x, name_y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(11.5),
        egui::Color32::WHITE,
    );
    if let Some(detail) = detail {
        painter.text(
            egui::pos2(text_x, rect.min.y + 24.0),
            egui::Align2::LEFT_CENTER,
            detail,
            egui::FontId::proportional(9.5),
            TEXT_DIM,
        );
    }
    if let Some(badge) = badge {
        painter.text(
            egui::pos2(name_end.max.x + 8.0, name_y),
            egui::Align2::LEFT_CENTER,
            badge,
            egui::FontId::proportional(8.5),
            ACCENT,
        );
    }

    let mut x = rect.max.x - 8.0;
    if let Some(seconds) = queue.and_then(|reading| reading.eta_seconds) {
        let drawn = painter.text(
            egui::pos2(x, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            format_wait(seconds),
            egui::FontId::proportional(10.0),
            TEXT_DIM,
        );
        x = drawn.min.x - 10.0;
    }
    if let Some(reading) = queue {
        let drawn = painter.text(
            egui::pos2(x, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            format!("Q:{}", reading.queue_position),
            egui::FontId::proportional(10.5),
            queue_color(reading.queue_position),
        );
        x = drawn.min.x - 10.0;
    }
    if let Some(ms) = ping_ms {
        painter.text(
            egui::pos2(x, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            format!("{ms} ms"),
            egui::FontId::proportional(10.5),
            ping_color(ms),
        );
    }

    response.clicked()
}
pub(crate) fn sort_picker(
    ui: &mut egui::Ui,
    i18n: &I18n,
    current: CatalogSort,
    games: &[GameSummary],
) -> Option<AppCommand> {
    let mut command = None;
    let label = text1(
        i18n,
        "catalog-sort-button",
        "sort",
        i18n.text(current.label_key()).as_ref(),
    );
    let response = ui.add_sized(
        [150.0, 30.0],
        egui::Button::new(label.as_ref()).fill(BG_RAISED),
    );
    let popup_id = ui.make_persistent_id("catalog_sort_popup");
    if response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }
    egui::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(170.0);
            for candidate in CatalogSort::ALL {
                let label = if candidate == CatalogSort::LastPlayed {
                    let count = games.iter().filter(|g| g.last_played.is_some()).count();
                    format!("{} ({count})", i18n.text(candidate.label_key()))
                } else {
                    i18n.text(candidate.label_key()).to_string()
                };
                if ui.selectable_label(candidate == current, label).clicked() {
                    command = Some(AppCommand::SetSort(candidate));
                }
            }
        },
    );
    command
}

// same as sort_picker but for my games / all games
pub(crate) fn filter_picker(ui: &mut egui::Ui, i18n: &I18n, current: CatalogFilter) -> Option<AppCommand> {
    let mut command = None;
    let label = text1(
        i18n,
        "catalog-filter-button",
        "filter",
        i18n.text(current.label_key()).as_ref(),
    );
    let response = ui.add_sized(
        [150.0, 30.0],
        egui::Button::new(label.as_ref()).fill(BG_RAISED),
    );
    let popup_id = ui.make_persistent_id("catalog_filter_popup");
    if response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }
    egui::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(170.0);
            for candidate in CatalogFilter::ALL {
                let label = i18n.text(candidate.label_key());
                if ui
                    .selectable_label(candidate == current, label.as_ref())
                    .clicked()
                {
                    command = Some(AppCommand::SetFilter(candidate));
                }
            }
        },
    );
    command
}

pub(crate) fn title_list(ui: &mut egui::Ui, i18n: &I18n, view: &CatalogView<'_>) -> Vec<AppCommand> {
    let mut commands = Vec::new();

    let mut query = view.search_query.to_owned();
    let hint = if view.search_query.is_empty() {
        format!(
            "{}  ({})",
            i18n.text("catalog-search-hint"),
            view.filtered_indices.len()
        )
    } else {
        i18n.text("catalog-search-hint").to_string()
    };
    // Clearing used to take two Back presses while the on-screen keyboard was up (one to dismiss
    // it, one to actually empty the field) with no visible way to do it in one tap. The × sits
    // inside the field itself, at its right edge, the same "inline clear icon" every search box
    // uses - reserving a separate widget slot for it (an earlier version of this fix) left a
    // visible seam between two disconnected-looking boxes instead of one search field.
    let show_clear = !view.search_query.is_empty();
    let response = ui.add(
        egui::TextEdit::singleline(&mut query)
            .hint_text(hint)
            .desired_width(ui.available_width())
            .margin(egui::vec2(8.0, 6.0)),
    );
    let mut cleared = false;
    if show_clear {
        const CLEAR_SIZE: f32 = 20.0;
        let clear_rect = egui::Rect::from_center_size(
            egui::pos2(
                response.rect.right() - CLEAR_SIZE / 2.0 - 6.0,
                response.rect.center().y,
            ),
            egui::vec2(CLEAR_SIZE, CLEAR_SIZE),
        );
        let clear_response = ui.interact(
            clear_rect,
            ui.id().with("clear_search"),
            egui::Sense::click(),
        );
        let color = if clear_response.hovered() {
            egui::Color32::WHITE
        } else {
            TEXT_DIM
        };
        ui.painter().text(
            clear_rect.center(),
            egui::Align2::CENTER_CENTER,
            "×",
            egui::FontId::proportional(16.0),
            color,
        );
        cleared = clear_response.clicked();
    }
    if view.search_requested && !response.has_focus() {
        response.request_focus();
    }
    if response.gained_focus() && !view.search_requested {
        commands.push(AppCommand::RequestSearch);
    }
    if response.changed() {
        commands.push(AppCommand::SetSearchQuery(query));
    }
    if cleared {
        commands.push(AppCommand::SetSearchQuery(String::new()));
        commands.push(AppCommand::CloseSearch);
    }
    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
    if enter_pressed || (view.search_requested && response.lost_focus()) {
        commands.push(AppCommand::CloseSearch);
    }

    ui.add_space(6.0);

    if view.filtered_indices.is_empty() {
        ui.add_space(20.0);
        ui.label(
            egui::RichText::new(
                if view.games.is_empty() {
                    i18n.text("catalog-no-games-api")
                } else {
                    i18n.text("catalog-no-match")
                }
                .as_ref(),
            )
            .size(12.0)
            .color(TEXT_DIM),
        );
        return commands;
    }

    let total = view.filtered_indices.len();
    let font_id = egui::FontId::proportional(12.0);

    let selected_id = egui::Id::new("catalog_list_last_scrolled_selected");
    let offset_id = egui::Id::new("catalog_list_scroll_offset");
    let selection_changed =
        ui.ctx().data(|d| d.get_temp::<usize>(selected_id)) != Some(view.selected);

    ui.scope(|ui| {
        // `show_rows` lays rows out on a `row_height + item_spacing.y` pitch, so the virtual row
        // geometry only lines up with what the rows actually occupy when the spacing is zero and the
        // gap is painted inside the row rect instead.
        ui.spacing_mut().item_spacing.y = 0.0;

        // Scrolling is driven from the selection index rather than from the selected row's
        // `Response`: once the cursor steps past the last visible row that row is outside
        // `row_range`, so it is never emitted, and a response-based `scroll_to_me` had nothing to
        // scroll to - the list stayed frozen while the highlight kept moving.
        let mut scroll_area = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .drag_to_scroll(false);
        if selection_changed {
            let viewport_height = ui.available_height();
            let row_top = view.selected as f32 * ROW_HEIGHT;
            let current = ui
                .ctx()
                .data(|d| d.get_temp::<f32>(offset_id))
                .unwrap_or(0.0);
            let offset = current
                .min(row_top)
                .max(row_top + ROW_HEIGHT - viewport_height)
                .max(0.0);
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        }

        let output = scroll_area.show_rows(ui, ROW_HEIGHT, total, |ui, row_range| {
            let painter = ui.painter().clone();
            for row in row_range {
                let Some(&game_index) = view.filtered_indices.get(row) else {
                    continue;
                };
                let game = &view.games[game_index];
                let is_selected = row == view.selected;

                let (row_rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), ROW_HEIGHT),
                    egui::Sense::click(),
                );
                let rect = row_rect.shrink2(egui::vec2(0.0, 1.5));
                if !ui.is_rect_visible(row_rect) {
                    if response.clicked() {
                        commands.push(AppCommand::SelectGame(row));
                    }
                    continue;
                }

                painter.rect_filled(rect, 6.0, if is_selected { BG_RAISED } else { BG_PANEL });
                if is_selected {
                    painter.rect_stroke(
                        rect,
                        6.0,
                        egui::Stroke::new(1.5_f32, ACCENT),
                        egui::StrokeKind::Inside,
                    );
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            rect.min + egui::vec2(2.0, 4.0),
                            egui::vec2(3.0, rect.height() - 8.0),
                        ),
                        1.5,
                        ACCENT,
                    );
                }

                let icon_size = ROW_HEIGHT - 11.0;
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x + 9.0, rect.center().y - icon_size / 2.0),
                    egui::vec2(icon_size, icon_size),
                );
                if !view.covers.is_requested(&game.app_id, CoverSize::Icon)
                    && let Some(url) = game.cover_url.clone()
                {
                    view.covers
                        .request_icon(view.http_client, ui.ctx(), game.app_id.clone(), url);
                }
                painter.rect_filled(icon_rect, 3.0, BG_DEEP);
                match view.covers.get_icon(&game.app_id) {
                    Some(CoverSnapshot::Ready(image)) => {
                        let tex = image.texture(ui.ctx(), || {
                            CoverStore::texture_key(&game.app_id, CoverSize::Icon)
                        });
                        let size = tex.size_vec2();
                        let src_aspect = size.x / size.y.max(1.0);
                        let uv = if src_aspect > 1.0 {
                            let inset = (1.0 - 1.0 / src_aspect) / 2.0;
                            egui::Rect::from_min_max(
                                egui::pos2(inset, 0.0),
                                egui::pos2(1.0 - inset, 1.0),
                            )
                        } else {
                            let inset = (1.0 - src_aspect) / 2.0;
                            egui::Rect::from_min_max(
                                egui::pos2(0.0, inset),
                                egui::pos2(1.0, 1.0 - inset),
                            )
                        };
                        painter.image(tex.id(), icon_rect, uv, egui::Color32::WHITE);
                    }
                    _ => {
                        painter.text(
                            icon_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            game.title.chars().next().unwrap_or('?').to_string(),
                            egui::FontId::proportional(11.0),
                            BORDER.gamma_multiply(3.0),
                        );
                    }
                }

                let text_color = if is_selected {
                    egui::Color32::WHITE
                } else {
                    TEXT_DIM
                };
                let text_x = icon_rect.max.x + 8.0;
                let mut job = egui::text::LayoutJob::single_section(
                    game.title.clone(),
                    egui::TextFormat::simple(font_id.clone(), text_color),
                );
                job.wrap = egui::text::TextWrapping::truncate_at_width(rect.max.x - text_x - 8.0);
                let galley = painter.layout_job(job);
                painter.galley(
                    egui::pos2(text_x, rect.center().y - galley.size().y / 2.0),
                    galley,
                    text_color,
                );

                // A small favourite marker only, not a button: starring happens in the detail
                // panel. A tap target per row meant 5829 of them competing with the row's own
                // click, for an action taken on one game at a time.
                if view.favorites.contains(&game.app_id) {
                    paint_heart(
                        &painter,
                        egui::Rect::from_center_size(
                            egui::pos2(rect.max.x - 14.0, rect.center().y),
                            egui::vec2(11.0, 11.0),
                        ),
                        true,
                        DANGER,
                    );
                }

                if response.clicked() {
                    commands.push(AppCommand::SelectGame(row));
                }
            }
        });

        ui.ctx().data_mut(|d| {
            d.insert_temp(offset_id, output.state.offset.y);
            d.insert_temp(selected_id, view.selected);
        });
    });

    commands
}

/// Right-hand detail panel: big cover, title, metadata and the PLAY button for whichever game the
/// list has highlighted.
pub(crate) fn detail_panel(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    i18n: &I18n,
    view: &CatalogView<'_>,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();

    let Some(game) = selected_game(view.games, view.filtered_indices, view.selected) else {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(i18n.text("detail-empty").as_ref())
                    .size(13.0)
                    .color(TEXT_DIM),
            );
        });
        return commands;
    };

    if !view.covers.is_requested(&game.app_id, CoverSize::Cover)
        && let Some(url) = game.cover_url.clone()
    {
        view.covers
            .request(view.http_client, ctx, game.app_id.clone(), url);
    }

    draw_panel_backdrop(ui, ctx, view.covers, game);

    let cart_height = 226.0;

    ui.horizontal(|ui| {
        draw_cover(ui, ctx, view.covers, game, cart_height, false);

        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            let mut favorite_toggled = false;
            // Height is pinned, not left to the layout. A bare `with_layout(right_to_left, ..)`
            // here claimed the whole remaining height of the panel and centred itself in it,
            // shoving the store badge, the app id and the PLAY button off the bottom.
            //
            // Right-to-left within that row so the heart takes its space first and the title
            // truncates into what is left; the other way round, a long title pushed the heart off
            // the edge.
            const TITLE_ROW_HEIGHT: f32 = 28.0;
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), TITLE_ROW_HEIGHT),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    let is_favorite = view.favorites.contains(&game.app_id);
                    let (heart_rect, heart_response) =
                        ui.allocate_exact_size(egui::vec2(28.0, 24.0), egui::Sense::click());
                    paint_heart(
                        &ui.painter().clone(),
                        egui::Rect::from_center_size(heart_rect.center(), egui::vec2(15.0, 15.0)),
                        is_favorite,
                        if is_favorite { DANGER } else { TEXT_DIM },
                    );
                    if heart_response.clicked() {
                        favorite_toggled = true;
                    }
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&game.title)
                                .size(19.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .truncate(),
                    );
                },
            );
            if favorite_toggled {
                commands.push(AppCommand::ToggleFavorite(game.app_id.clone()));
            }
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                if let Some(store) = game.store.as_deref() {
                    let (label, color) = store_badge(store);
                    egui::Frame::NONE
                        .fill(color)
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(6, 3))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(label)
                                    .size(10.0)
                                    .color(egui::Color32::WHITE),
                            );
                        });
                }
            });
            ui.add_space(4.0);

            let played = match &game.last_played {
                Some(date) => text1(i18n, "detail-last-played", "date", short_date(date)),
                None => i18n.text("detail-never-played"),
            };
            ui.label(
                egui::RichText::new(played.as_ref())
                    .size(11.0)
                    .color(TEXT_DIM),
            );
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(text1(i18n, "detail-app-id", "id", &game.app_id).as_ref())
                    .size(10.0)
                    .monospace()
                    .color(BORDER.gamma_multiply(3.0)),
            );

            ui.add_space(14.0);
            if play_button(ui, i18n) {
                commands.push(AppCommand::Input(crate::input::InputCommand::Confirm));
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(i18n.text("detail-press").as_ref())
                        .size(11.0)
                        .color(TEXT_DIM),
                );
                if let Some(glyph) = ps_button(ui.ctx(), PsButton::Cross) {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(15.0, 15.0), egui::Sense::hover());
                    ui.painter().image(
                        glyph.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                ui.label(
                    egui::RichText::new(i18n.text("detail-to-start").as_ref())
                        .size(11.0)
                        .color(TEXT_DIM),
                );
            });
        });
    });

    commands
}

/// The big green PLAY button, hand-painted so it can carry a vertical gradient - egui's `Button`
/// only does flat fills.
pub(crate) fn play_button(ui: &mut egui::Ui, i18n: &I18n) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(200.0, 44.0), egui::Sense::click());
    let painter = ui.painter();

    let boost = if response.is_pointer_button_down_on() {
        -18
    } else if response.hovered() {
        14
    } else {
        0
    };
    let shade = |base: egui::Color32, delta: i32| {
        let apply = |c: u8| (c as i32 + delta).clamp(0, 255) as u8;
        egui::Color32::from_rgb(apply(base.r()), apply(base.g()), apply(base.b()))
    };
    let top = shade(egui::Color32::from_rgb(0x9c, 0xd3, 0x2b), boost);
    let bottom = shade(egui::Color32::from_rgb(0x6a, 0xa8, 0x00), boost);

    let radius = rect.height() / 2.0;
    let mid = egui::Color32::from_rgb(
        ((top.r() as u16 + bottom.r() as u16) / 2) as u8,
        ((top.g() as u16 + bottom.g() as u16) / 2) as u8,
        ((top.b() as u16 + bottom.b() as u16) / 2) as u8,
    );
    painter.circle_filled(
        egui::pos2(rect.min.x + radius, rect.center().y),
        radius,
        mid,
    );
    painter.circle_filled(
        egui::pos2(rect.max.x - radius, rect.center().y),
        radius,
        mid,
    );

    let body = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + radius, rect.min.y),
        egui::pos2(rect.max.x - radius, rect.max.y),
    );
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(body.left_top(), top);
    mesh.colored_vertex(body.right_top(), top);
    mesh.colored_vertex(body.left_bottom(), bottom);
    mesh.colored_vertex(body.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(egui::Shape::Mesh(mesh.into()));

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        i18n.text("detail-play"),
        egui::FontId::proportional(18.0),
        egui::Color32::from_rgb(0x10, 0x1a, 0x00),
    );

    response.clicked()
}

/// How strongly the backdrop art shows through.
pub(crate) const BACKDROP_ALPHA: u8 = 58;

/// Paints the selected game's cover across the whole detail panel as a dimmed backdrop.
pub(crate) fn draw_panel_backdrop(
    ui: &egui::Ui,
    ctx: &egui::Context,
    covers: &CoverStore,
    game: &GameSummary,
) {
    let Some(CoverSnapshot::Ready(image)) = covers.get(&game.app_id) else {
        return;
    };
    let rect = ui.max_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }

    let tex = image.texture(ctx, || {
        CoverStore::texture_key(&game.app_id, CoverSize::Cover)
    });
    let tex_size = tex.size_vec2();
    let src_aspect = tex_size.x / tex_size.y.max(1.0);
    let dst_aspect = rect.width() / rect.height();
    let uv = if src_aspect > dst_aspect {
        let inset = (1.0 - dst_aspect / src_aspect) / 2.0;
        egui::Rect::from_min_max(egui::pos2(inset, 0.0), egui::pos2(1.0 - inset, 1.0))
    } else {
        let inset = (1.0 - src_aspect / dst_aspect) / 2.0;
        egui::Rect::from_min_max(egui::pos2(0.0, inset), egui::pos2(1.0, 1.0 - inset))
    };

    ui.painter().image(
        tex.id(),
        rect,
        uv,
        egui::Color32::from_white_alpha(BACKDROP_ALPHA),
    );
}

/// Trims an ISO-8601 timestamp down to its `YYYY-MM-DD` date part.
pub(crate) fn short_date(iso: &str) -> &str {
    iso.split('T').next().unwrap_or(iso)
}

/// Draws the cover art seated inside a PS Vita cartridge shell.
pub(crate) fn draw_cover(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    covers: &CoverStore,
    game: &GameSummary,
    cart_height: f32,
    // Stand in with the list thumbnail while the full-size cover is still downloading.
    icon_fallback: bool,
) {
    let cart_width = cart_height * CART_ASPECT;
    let (cart, _) =
        ui.allocate_exact_size(egui::vec2(cart_width, cart_height), egui::Sense::hover());
    let shell = cart_frame(ctx);

    let painter = ui.painter().clone();
    let rect = if shell.is_some() {
        egui::Rect::from_min_max(
            egui::pos2(
                cart.min.x + cart_width * CART_WINDOW_X.0,
                cart.min.y + cart_height * CART_WINDOW_Y.0,
            ),
            egui::pos2(
                cart.min.x + cart_width * CART_WINDOW_X.1,
                cart.min.y + cart_height * CART_WINDOW_Y.1,
            ),
        )
    } else {
        let inset = cart.shrink(6.0);
        painter.rect_stroke(
            inset,
            8.0,
            egui::Stroke::new(1.0_f32, BORDER.gamma_multiply(2.0)),
            egui::StrokeKind::Inside,
        );
        inset
    };
    painter.rect_filled(rect, 4.0, BG_DEEP);

    let paint_at = |size: CoverSize, image: &Arc<crate::gfn::covers::TitleImage>| {
        let tex = image.texture(ctx, || CoverStore::texture_key(&game.app_id, size));
        let tex_size = tex.size_vec2();
        let src_aspect = tex_size.x / tex_size.y.max(1.0);
        let slot_aspect = rect.width() / rect.height();
        let uv = if src_aspect > slot_aspect {
            let inset = (1.0 - slot_aspect / src_aspect) / 2.0;
            egui::Rect::from_min_max(egui::pos2(inset, 0.0), egui::pos2(1.0 - inset, 1.0))
        } else {
            let inset = (1.0 - src_aspect / slot_aspect) / 2.0;
            egui::Rect::from_min_max(egui::pos2(0.0, inset), egui::pos2(1.0, 1.0 - inset))
        };
        painter.image(tex.id(), rect, uv, egui::Color32::WHITE);
    };

    match covers.get(&game.app_id) {
        Some(CoverSnapshot::Ready(image)) => paint_at(CoverSize::Cover, &image),
        // The list thumbnail for this title is usually already decoded, so it stands in - soft,
        // but art immediately instead of a spinner, and it is replaced the moment the full cover
        // lands.
        other => match (icon_fallback, covers.get_icon(&game.app_id)) {
            (true, Some(CoverSnapshot::Ready(icon))) => paint_at(CoverSize::Icon, &icon),
            _ => match other {
                Some(CoverSnapshot::Loading) => {
                    ui.put(rect, egui::Spinner::new());
                }
                _ => {
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        game.title.chars().next().unwrap_or('?').to_string(),
                        egui::FontId::proportional(48.0),
                        TEXT_DIM,
                    );
                }
            },
        },
    }

    if let Some(shell) = shell {
        painter.image(
            shell.id(),
            cart,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
}

/// Short badge label + fill color for a GFN `appStore` value (`"STEAM"`, `"EPIC"`, ...).
pub(crate) fn store_badge(store: &str) -> (&'static str, egui::Color32) {
    match store.to_ascii_uppercase().as_str() {
        "STEAM" => ("Steam", egui::Color32::from_rgb(0x1b, 0x2a, 0x38)),
        "EPIC" | "EPIC_GAMES" => ("Epic", egui::Color32::from_rgb(0x2a, 0x2a, 0x2a)),
        "EA_APP" | "EA" | "ORIGIN" => ("EA", egui::Color32::from_rgb(0xc4, 0x2b, 0x1c)),
        "UBISOFT" | "UPLAY" => ("Ubisoft", egui::Color32::from_rgb(0x00, 0x69, 0xd2)),
        "BATTLENET" | "BATTLE_NET" => ("Battle.net", egui::Color32::from_rgb(0x00, 0x3f, 0x6b)),
        "XBOX" | "MICROSOFT_STORE" => ("Xbox", egui::Color32::from_rgb(0x10, 0x7c, 0x10)),
        "GOG" => ("GOG", egui::Color32::from_rgb(0x86, 0x2d, 0x59)),
        "RIOT" | "RIOT_GAMES" => ("Riot", egui::Color32::from_rgb(0xd1, 0x33, 0x22)),
        _ => ("Game", egui::Color32::from_rgb(0x44, 0x44, 0x44)),
    }
}

/// Header row shared by the session/streaming screens: a title on the left and a stop button on
/// the right.
/// Where the launch pipeline is, as the three dots the player sees. `Queue` is CloudMatch holding
/// us behind other users, `Setup` is the rig being provisioned, `Ready` covers the handoff to
/// signaling once a session exists.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaunchStage {
    Queue,
    Setup,
    Ready,
}

impl LaunchStage {
    fn index(self) -> usize {
        match self {
            Self::Queue => 0,
            Self::Setup => 1,
            Self::Ready => 2,
        }
    }
}

/// Everything the launch overlay needs that isn't the catalog behind it.
pub(crate) struct LaunchView<'a> {
    pub(crate) stage: LaunchStage,
    pub(crate) game: Option<&'a GameSummary>,
    /// Large line under the stepper.
    pub(crate) headline: std::rc::Rc<str>,
    /// Small line under the headline, if there's anything more specific to say.
    pub(crate) detail: Option<std::rc::Rc<str>>,
    /// False on the stages that are waiting on the player rather than on NVIDIA.
    pub(crate) spinning: bool,
    /// The launch never sat in NVIDIA's queue, so step 1 is drawn as skipped rather than as
    /// completed - marking it green claims the player waited through a queue that never existed.
    pub(crate) queue_skipped: bool,
    pub(crate) session_id: Option<&'a str>,
}

pub(crate) const LAUNCH_MODAL_WIDTH: f32 = 300.0;
pub(crate) const STEP_DOT_RADIUS: f32 = 13.0;

/// The whole "starting a session" flow as one modal over the still-visible library, rather than
/// three separate full-screen states - the player never loses sight of what they launched.
pub(crate) fn session_launch_overlay(
    ctx: &egui::Context,
    i18n: &I18n,
    catalog: &CatalogView<'_>,
    launch: &LaunchView<'_>,
) -> Option<AppCommand> {
    // Drawn purely as a backdrop: the modal takes the input layer, so the list underneath cannot
    // be interacted with and its commands are discarded.
    let _ = catalog_screen(ctx, i18n, catalog);

    let mut command = None;
    egui::Modal::new(egui::Id::new("session_launch_overlay"))
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

            launch_header(ui, i18n, catalog, launch.game);
            ui.add_space(12.0);
            launch_stepper(ui, i18n, launch.stage, launch.queue_skipped);
            ui.add_space(14.0);

            ui.vertical_centered(|ui| {
                if launch.spinning {
                    ui.add(egui::Spinner::new().size(20.0).color(ACCENT));
                    ui.add_space(8.0);
                }
                ui.label(
                    egui::RichText::new(launch.headline.as_ref())
                        .size(15.0)
                        .color(egui::Color32::WHITE),
                );
                if let Some(detail) = &launch.detail {
                    ui.add_space(3.0);
                    button_hint(ui, detail.as_ref(), 11.0, TEXT_DIM, true);
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);

            if ui
                .add_sized(
                    egui::vec2(ui.available_width(), 30.0),
                    egui::Button::new(
                        egui::RichText::new(i18n.text("session-cancel-button").as_ref())
                            .size(14.0)
                            .color(DANGER),
                    )
                    .fill(BG_RAISED),
                )
                .clicked()
            {
                command = Some(AppCommand::ToggleConfirmExit);
            }

            ui.add_space(5.0);
            button_hint(ui, &i18n.text("session-exit-hint"), 10.0, TEXT_DIM, true);
            // Only diagnostic worth keeping on screen: `status_note` is shared with every other
            // screen, so during a launch it still holds whatever the catalog last said.
            if let Some(id) = launch.session_id.filter(|id| !id.is_empty()) {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(id).size(8.0).color(BORDER));
                });
            }
        });
    command
}

/// One segment of a hint line: literal text, or a face-button glyph standing in for a marker.
pub(crate) enum HintSegment<'a> {
    Text(&'a str),
    Button(PsButton),
}

/// Renders a hint line, swapping the literal `(X)` / `(O)` markers in the translated string for
/// the real PlayStation face-button glyphs. The markers stay in the `.ftl` files so translators
/// can move them around inside the sentence, and a string may contain several.
pub(crate) fn button_hint(ui: &mut egui::Ui, text: &str, size: f32, color: egui::Color32, centered: bool) {
    const GAP: f32 = 4.0;
    let glyph_size = size + 3.0;
    let font = egui::FontId::proportional(size);

    let mut segments = Vec::new();
    let mut rest = text;
    loop {
        let next = [("(X)", PsButton::Cross), ("(O)", PsButton::Circle)]
            .into_iter()
            .filter_map(|(marker, button)| rest.find(marker).map(|at| (at, marker, button)))
            .min_by_key(|(at, _, _)| *at);
        let Some((at, marker, button)) = next else {
            if !rest.trim().is_empty() {
                segments.push(HintSegment::Text(rest.trim()));
            }
            break;
        };
        if !rest[..at].trim().is_empty() {
            segments.push(HintSegment::Text(rest[..at].trim()));
        }
        segments.push(HintSegment::Button(button));
        rest = &rest[at + marker.len()..];
    }

    let run_width: f32 = segments
        .iter()
        .map(|segment| match segment {
            HintSegment::Text(text) => ui.fonts(|fonts| {
                fonts
                    .layout_no_wrap((*text).to_owned(), font.clone(), color)
                    .size()
                    .x
            }),
            HintSegment::Button(_) => glyph_size,
        })
        .sum::<f32>()
        + GAP * segments.len().saturating_sub(1) as f32;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        if centered {
            ui.add_space(((ui.available_width() - run_width) / 2.0).max(0.0));
        }
        for segment in segments {
            match segment {
                HintSegment::Text(text) => {
                    ui.label(egui::RichText::new(text).size(size).color(color));
                }
                HintSegment::Button(button) => {
                    if let Some(glyph) = ps_button(ui.ctx(), button) {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(glyph_size, glyph_size),
                            egui::Sense::hover(),
                        );
                        ui.painter().image(
                            glyph.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }
        }
    });
}

/// Cover thumbnail + "Now loading" / title / storefront, mirroring the catalog's detail panel so
/// the overlay reads as the same title the player just picked.
pub(crate) fn launch_header(
    ui: &mut egui::Ui,
    i18n: &I18n,
    catalog: &CatalogView<'_>,
    game: Option<&GameSummary>,
) {
    ui.horizontal(|ui| {
        const HEADER_CART_HEIGHT: f32 = 76.0;
        match game {
            Some(game) => {
                // Same request + `draw_cover` path the detail panel uses, so the art, the loading
                // spinner and the initial-letter fallback all behave identically here.
                if !catalog.covers.is_requested(&game.app_id, CoverSize::Cover)
                    && let Some(url) = game.cover_url.clone()
                {
                    catalog
                        .covers
                        .request(catalog.http_client, ui.ctx(), game.app_id.clone(), url);
                }
                let ctx = ui.ctx().clone();
                draw_cover(ui, &ctx, catalog.covers, game, HEADER_CART_HEIGHT, true);
            }
            None => {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(HEADER_CART_HEIGHT * CART_ASPECT, HEADER_CART_HEIGHT),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(rect, 4.0, BG_DEEP);
            }
        }

        ui.add_space(10.0);
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(i18n.text("session-now-loading").as_ref())
                    .size(10.0)
                    .color(ACCENT),
            );
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(match game {
                    Some(game) => game.title.as_str(),
                    None => "",
                })
                .size(16.0)
                .color(egui::Color32::WHITE),
            );
            if let Some(store) = game.and_then(|game| game.store.as_deref()) {
                ui.add_space(2.0);
                ui.label(egui::RichText::new(store).size(10.0).color(TEXT_DIM));
            }
        });
    });
}

/// Three numbered dots joined by rails, filled up to `stage`.
pub(crate) fn launch_stepper(ui: &mut egui::Ui, i18n: &I18n, stage: LaunchStage, queue_skipped: bool) {
    const LABELS: [&str; 3] = [
        "session-step-queue",
        "session-step-setup",
        "session-step-ready",
    ];

    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(width, STEP_DOT_RADIUS * 2.0 + 18.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    let dot_y = rect.top() + STEP_DOT_RADIUS;
    // Inset by the radius so the outer dots sit fully inside `rect` rather than half-clipped.
    let first_x = rect.left() + STEP_DOT_RADIUS + 24.0;
    let last_x = rect.right() - STEP_DOT_RADIUS - 24.0;
    let gap = (last_x - first_x) / 2.0;

    for step in 0..3 {
        let x = first_x + gap * step as f32;
        let skipped = step == 0 && queue_skipped;
        let reached = step <= stage.index() && !skipped;
        let center = egui::pos2(x, dot_y);

        if step > 0 {
            painter.line_segment(
                [
                    egui::pos2(x - gap + STEP_DOT_RADIUS + 2.0, dot_y),
                    egui::pos2(x - STEP_DOT_RADIUS - 2.0, dot_y),
                ],
                egui::Stroke::new(2.0_f32, if reached { ACCENT } else { BORDER }),
            );
        }

        painter.circle_filled(
            center,
            STEP_DOT_RADIUS,
            if step == stage.index() {
                ACCENT
            } else {
                BG_RAISED
            },
        );
        if reached && step != stage.index() {
            painter.circle_stroke(center, STEP_DOT_RADIUS, egui::Stroke::new(1.5_f32, ACCENT));
        }
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            (step + 1).to_string(),
            egui::FontId::proportional(12.0),
            if step == stage.index() {
                BG_DEEP
            } else if reached {
                ACCENT
            } else {
                TEXT_DIM
            },
        );
        painter.text(
            egui::pos2(x, dot_y + STEP_DOT_RADIUS + 8.0),
            egui::Align2::CENTER_CENTER,
            i18n.text(LABELS[step]),
            egui::FontId::proportional(10.0),
            if reached {
                egui::Color32::WHITE
            } else {
                TEXT_DIM
            },
        );
    }
}

/// Turns the CloudMatch queue snapshot into the overlay's stage + wording.
pub(crate) fn creating_session_launch<'a>(
    i18n: &I18n,
    game: Option<&'a GameSummary>,
    is_polling: bool,
    queue_status: &crate::gfn::cloudmatch::QueueStatus,
    was_queued: bool,
) -> LaunchView<'a> {
    // Checked before the server-error case: a patch is reported as a 5xx but is not a failure, and
    // it can hold the launch for many minutes - long enough that silence reads as a hang.
    if queue_status.app_patching {
        return LaunchView {
            stage: LaunchStage::Setup,
            game,
            headline: i18n.text("session-app-patching"),
            detail: Some(i18n.text("session-app-patching-detail")),
            spinning: true,
            session_id: None,
            queue_skipped: !was_queued,
        };
    }

    if queue_status.has_video_ad {
        let percent = (queue_status.ad_progress_pct.clamp(0.0, 1.0) * 100.0).round() as u32;
        return LaunchView {
            stage: LaunchStage::Queue,
            game,
            headline: i18n.text("session-ad-playing"),
            detail: Some(text1(i18n, "session-ad-progress", "percent", percent)),
            spinning: true,
            session_id: None,
            queue_skipped: !was_queued,
        };
    }

    // A run of 5xx replies looks identical to a stalled launch from the outside, so it gets said
    // out loud rather than hidden behind the queue position.
    if queue_status.server_errors > 0 {
        return LaunchView {
            stage: LaunchStage::Setup,
            game,
            headline: i18n.text("session-server-busy"),
            detail: Some(text1(
                i18n,
                "session-server-busy-retry",
                "attempt",
                queue_status.server_errors,
            )),
            spinning: true,
            session_id: None,
            queue_skipped: !was_queued,
        };
    }

    let queued = queue_status.queue_position > 0;
    let mut detail = None;

    let headline = if queued {
        detail = if queue_status.eta_ms > 0 {
            let secs = (queue_status.eta_ms / 1000) % 60;
            let mins = queue_status.eta_ms / 60000;
            Some(if mins > 0 {
                text2(
                    i18n,
                    "session-eta-minutes",
                    ("minutes", mins),
                    ("seconds", secs),
                )
            } else {
                text1(i18n, "session-eta-seconds", "seconds", secs)
            })
        } else {
            Some(text1(
                i18n,
                "session-queue-live",
                "attempt",
                queue_status.attempt,
            ))
        };
        text1(
            i18n,
            "session-queue-position",
            "position",
            queue_status.queue_position,
        )
    } else {
        if is_polling && queue_status.attempt > 0 {
            detail = Some(text1(
                i18n,
                "session-connecting-attempt",
                "attempt",
                queue_status.attempt,
            ));
        } else if is_polling {
            detail = Some(i18n.text("session-waiting-ready"));
        }
        i18n.text("session-preparing-rig")
    };

    LaunchView {
        stage: if queued {
            LaunchStage::Queue
        } else {
            LaunchStage::Setup
        },
        game,
        headline,
        detail,
        spinning: true,
        session_id: None,
        queue_skipped: !was_queued,
    }
}
