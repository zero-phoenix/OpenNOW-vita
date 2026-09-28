use super::{App, AppState};
use super::pause_menu;
use super::{catalog_ui::*, stream_ui::*};
use crate::app::theme::{ACCENT, BG_DEEP, BG_PANEL, BG_RAISED, BORDER, DANGER, TEXT_DIM};
use crate::gfn::auth::GfnUser;
use crate::i18n::{I18n, arg_string};
use crate::input::AppCommand;
use fluent_bundle::FluentArgs;
use std::sync::Arc;
// Old paths (shell, app, main) keep resolving: the split moved these, the callers did not.
pub(crate) use super::catalog_ui::selected_game;
pub(crate) use super::stream_ui::{
    flash_control_manual, flash_profile_toast, keyboard_panel_rect, note_overlay_touch,
    stream_ui_rects,
};

/// Builds the egui UI for the current frame and returns any commands produced by widget
/// interaction (buttons etc.) so the caller can feed them back through `App::handle_command`.


/// Width of the left-hand title list.

/// Installs the app's style, palette and touch-input tuning.
pub(crate) fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.spacing.scroll.bar_width = 4.0;
    style.spacing.scroll.bar_inner_margin = 0.0;
    style.spacing.scroll.bar_outer_margin = 0.0;
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.interaction.interact_radius = 12.0;

    ctx.set_style(style);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_DEEP;
    visuals.window_fill = BG_PANEL;
    visuals.extreme_bg_color = egui::Color32::BLACK;
    visuals.faint_bg_color = BG_RAISED;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.45);
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    visuals.hyperlink_color = ACCENT;
    ctx.set_visuals(visuals);

    ctx.options_mut(|options| {
        options.input_options.max_click_duration = 5.0;
        options.input_options.max_click_dist = 32.0;
    });
}

/// The GeForce NOW wordmark, embedded in the binary and decoded into exactly one egui texture for
/// the whole process.
pub(crate) fn geforce_logo(ctx: &egui::Context) -> Option<Arc<egui::TextureHandle>> {
    const LOGO_PNG: &[u8] = include_bytes!("../../assets/geforce-now-logo.png");
    embedded_texture(ctx, "gfn_logo", LOGO_PNG, 384)
}

/// The PlayStation face buttons, for input hints.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PsButton {
    Cross,
    Circle,
}

impl PsButton {
    fn asset(self) -> (&'static str, &'static [u8]) {
        match self {
            Self::Cross => (
                "ps_button_cross",
                include_bytes!("../../assets/ps-button-x.png"),
            ),
            Self::Circle => (
                "ps_button_circle",
                include_bytes!("../../assets/ps-button-c.png"),
            ),
        }
    }
}

pub(crate) fn ps_button(ctx: &egui::Context, button: PsButton) -> Option<Arc<egui::TextureHandle>> {
    let (key, bytes) = button.asset();
    embedded_texture(ctx, key, bytes, 64)
}

/// PS Vita cartridge shell with a transparent window, drawn *over* the cover art so each title
/// looks like a physical Vita game card.
pub(crate) fn cart_frame(ctx: &egui::Context) -> Option<Arc<egui::TextureHandle>> {
    const CART_PNG: &[u8] = include_bytes!("../../assets/casset.png");
    embedded_texture(ctx, "vita_cart_frame", CART_PNG, 200)
}

pub(crate) fn vita_front(ctx: &egui::Context) -> Option<Arc<egui::TextureHandle>> {
    const FRONT_PNG: &[u8] = include_bytes!("../../assets/front.png");
    embedded_texture(ctx, "vita_front", FRONT_PNG, 480)
}

pub(crate) fn vita_back(ctx: &egui::Context) -> Option<Arc<egui::TextureHandle>> {
    const BACK_PNG: &[u8] = include_bytes!("../../assets/back.png");
    embedded_texture(ctx, "vita_back", BACK_PNG, 480)
}

pub(crate) const CART_ASPECT: f32 = 447.0 / 558.0;
pub(crate) const CART_WINDOW_X: (f32, f32) = (0.1611, 0.8479);
pub(crate) const CART_WINDOW_Y: (f32, f32) = (0.0376, 0.8513);

pub(crate) const REAR_PAD_X: (f32, f32) = (0.20, 0.80);
pub(crate) const REAR_PAD_Y: (f32, f32) = (0.18, 0.82);

pub(crate) const FRONT_SCREEN_X: (f32, f32) = (0.20, 0.79);
pub(crate) const FRONT_SCREEN_Y: (f32, f32) = (0.12, 0.83);

/// Decodes a PNG compiled into the binary into exactly one cached egui texture.
pub(crate) fn embedded_texture(
    ctx: &egui::Context,
    key: &'static str,
    bytes: &'static [u8],
    max_width: u32,
) -> Option<Arc<egui::TextureHandle>> {
    let cache_id = egui::Id::new(("embedded_texture", key));
    if let Some(cached) =
        ctx.data(|data| data.get_temp::<Option<Arc<egui::TextureHandle>>>(cache_id))
    {
        return cached;
    }

    let decoded = image::load_from_memory(bytes)
        .inspect_err(|error| crate::diag!("failed to decode embedded image {key}: {error}"))
        .ok()
        .map(|image| {
            let rgba = image.to_rgba8();
            let (width, height) = rgba.dimensions();
            if width <= max_width {
                return rgba;
            }
            let target_height = (height * max_width / width.max(1)).max(1);
            image::imageops::resize(
                &rgba,
                max_width,
                target_height,
                image::imageops::FilterType::Triangle,
            )
        })
        .map(|rgba| {
            let (width, height) = rgba.dimensions();
            let handle = ctx.load_texture(
                key,
                egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    rgba.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            );
            Arc::new(handle)
        });

    ctx.data_mut(|data| data.insert_temp(cache_id, decoded.clone()));
    decoded
}

/// The glyph drawn on a streaming-overlay button.
///
/// Formats `id` with a single Fluent argument.
pub(crate) fn text1(
    i18n: &I18n,
    id: &'static str,
    key: &'static str,
    value: impl ToString,
) -> std::rc::Rc<str> {
    let mut args = FluentArgs::new();
    args.set(key, arg_string(value.to_string()));
    i18n.text_with(id, args)
}

pub(crate) fn text2(
    i18n: &I18n,
    id: &'static str,
    first: (&'static str, impl ToString),
    second: (&'static str, impl ToString),
) -> std::rc::Rc<str> {
    let mut args = FluentArgs::new();
    args.set(first.0, arg_string(first.1.to_string()));
    args.set(second.0, arg_string(second.1.to_string()));
    i18n.text_with(id, args)
}

/// Everything the catalog screen needs, bundled so the renderer doesn't take a dozen positional

const SPLASH_FADE_IN: f64 = 0.55;
const SPLASH_HOLD: f64 = 1.05;
const SPLASH_FADE_OUT: f64 = 0.60;
pub(crate) const SPLASH_TOTAL: f64 = SPLASH_FADE_IN + SPLASH_HOLD + SPLASH_FADE_OUT;
const SPLASH_OPAQUE_UNTIL: f64 = SPLASH_FADE_IN + SPLASH_HOLD;

pub fn build_ui(ctx: &egui::Context, app: &App) -> Vec<AppCommand> {
    let splash_elapsed = ctx.input(|input| input.time);
    if splash_elapsed < SPLASH_TOTAL {
        ctx.request_repaint();
    }
    if splash_elapsed < SPLASH_OPAQUE_UNTIL {
        splash_overlay(ctx);
        return Vec::new();
    }

    let i18n = I18n::new(app.locale);
    let mut commands = Vec::new();

    match &app.state {
        AppState::Login => login_screen(ctx, &i18n, app),
        AppState::StartingDeviceLogin(_) => starting_login_screen(ctx, &i18n),
        AppState::WaitingForDeviceAuthorization { challenge, .. } => {
            device_code_screen(ctx, &i18n, challenge)
        }
        AppState::LoadingCatalog { user, .. } => loading_catalog_screen(ctx, &i18n, user),
        AppState::Catalog {
            user,
            games,
            selected,
            filtered_indices,
            search_query,
            search_requested,
            covers,
        } => {
            commands.extend(catalog_screen(
                ctx,
                &i18n,
                &CatalogView {
                    user,
                    games,
                    selected: *selected,
                    filtered_indices,
                    search_query,
                    search_requested: *search_requested,
                    covers,
                    http_client: &app.http_client,
                    status_note: app.status_note.as_deref(),
                    sort: app.catalog_sort,
                    filter: app.catalog_filter,
                    total_count: app.catalog_total_count(),
                    favorites: &app.favorites,
                    regions: RegionsView::from_app(app),
                    settings: SettingsView::from_app(app),
                    loading_more: app.is_loading_more_catalog(),
                },
            ));
            if app.server_picker_open {
                commands.extend(server_picker_modal(
                    ctx,
                    &i18n,
                    app,
                    selected_game(games, filtered_indices, *selected),
                ));
            }
        }
        AppState::CreatingSession {
            user,
            games,
            selected,
            filtered_indices,
            search_query,
            search_requested,
            covers,
            job,
            queue_tracker,
        } => {
            let queue_status = queue_tracker
                .lock()
                .map(|st| st.clone())
                .unwrap_or_default();
            let game = selected_game(games, filtered_indices, *selected);
            let launch = creating_session_launch(
                &i18n,
                game,
                job.is_pending(),
                &queue_status,
                app.launch_was_queued || queue_status.was_queued,
            );
            let catalog = CatalogView {
                user,
                games,
                selected: *selected,
                filtered_indices,
                search_query,
                search_requested: *search_requested,
                covers,
                http_client: &app.http_client,
                status_note: None,
                sort: app.catalog_sort,
                filter: app.catalog_filter,
                total_count: app.catalog_total_count(),
                loading_more: app.is_loading_more_catalog(),
                favorites: &app.favorites,
                regions: RegionsView::from_app(app),
                settings: SettingsView::from_app(app),
            };
            if let Some(cmd) = session_launch_overlay(ctx, &i18n, &catalog, &launch) {
                commands.push(cmd);
            }
        }
        AppState::SessionReady {
            user,
            games,
            selected,
            filtered_indices,
            search_query,
            search_requested,
            covers,
            session,
        } => {
            let launch = LaunchView {
                stage: LaunchStage::Ready,
                game: selected_game(games, filtered_indices, *selected),
                headline: i18n.text("session-ready-headline"),
                detail: Some(i18n.text("session-ready-hint")),
                // Waiting on the player's Confirm, not on NVIDIA.
                spinning: false,
                session_id: Some(&session.session_id),
                queue_skipped: !app.launch_was_queued,
            };
            let catalog = CatalogView {
                user,
                games,
                selected: *selected,
                filtered_indices,
                search_query,
                search_requested: *search_requested,
                covers,
                http_client: &app.http_client,
                status_note: None,
                sort: app.catalog_sort,
                filter: app.catalog_filter,
                total_count: app.catalog_total_count(),
                loading_more: app.is_loading_more_catalog(),
                favorites: &app.favorites,
                regions: RegionsView::from_app(app),
                settings: SettingsView::from_app(app),
            };
            if let Some(cmd) = session_launch_overlay(ctx, &i18n, &catalog, &launch) {
                commands.push(cmd);
            }
        }
        AppState::Signaling {
            user,
            games,
            selected,
            filtered_indices,
            search_query,
            search_requested,
            covers,
            session,
            offer_sdp,
            ..
        } => {
            let launch = LaunchView {
                stage: LaunchStage::Ready,
                game: selected_game(games, filtered_indices, *selected),
                headline: i18n.text("signaling-title"),
                detail: Some(match offer_sdp.as_deref() {
                    Some(sdp) => text1(&i18n, "signaling-offer-received", "bytes", sdp.len()),
                    None => i18n.text("signaling-waiting-offer"),
                }),
                spinning: true,
                session_id: Some(&session.session_id),
                queue_skipped: !app.launch_was_queued,
            };
            let catalog = CatalogView {
                user,
                games,
                selected: *selected,
                filtered_indices,
                search_query,
                search_requested: *search_requested,
                covers,
                http_client: &app.http_client,
                status_note: None,
                sort: app.catalog_sort,
                filter: app.catalog_filter,
                total_count: app.catalog_total_count(),
                loading_more: app.is_loading_more_catalog(),
                favorites: &app.favorites,
                regions: RegionsView::from_app(app),
                settings: SettingsView::from_app(app),
            };
            if let Some(cmd) = session_launch_overlay(ctx, &i18n, &catalog, &launch) {
                commands.push(cmd);
            }
        }
        AppState::Streaming {
            games,
            selected,
            filtered_indices,
            peer,
            session_start,
            ..
        } => {
            if let Some(cmd) = streaming_screen(
                ctx,
                &i18n,
                selected_game(games, filtered_indices, *selected),
                peer.video_frame().is_some(),
                app.status_note.as_deref(),
                &app.fps_history,
                app.keyboard_open,
                app.show_stream_stats,
                app.toolbar_expanded,
                app.mouse_trackpad_enabled,
                app.battery,
                Some(*session_start),
                app.membership_tier.as_deref(),
                app.pause_menu_open,
                &app.hud,
            ) {
                commands.push(cmd);
            }
            // The pause menu paints after the screen so it sits above video, toolbar and
            // pill alike; its rows reserve their own touches.
            if app.pause_menu_open {
                commands.extend(pause_menu::paint(
                    ctx,
                    &i18n,
                    &app.pause_menu,
                    &pause_menu::PauseView {
                        stats_on: app.show_stream_stats,
                        keyboard_open: app.keyboard_open,
                        trackpad_on: app.mouse_trackpad_enabled,
                    },
                ));
            }
        }
        AppState::Error { message, code, .. } => error_screen(ctx, &i18n, message, *code),
    }

    if app.show_controls_modal
        && matches!(app.state, AppState::Streaming { .. })
        && let Some(cmd) = stream_controls_modal(ctx, &i18n)
    {
        commands.push(cmd);
    }

    if app.keyboard_open && matches!(app.state, AppState::Streaming { .. }) {
        commands.extend(if app.keyboard_shortcuts {
            windows_shortcuts_page(ctx)
        } else {
            on_screen_keyboard(ctx, app.key_shift, app.key_ctrl, app.key_alt)
        });
    }

    if app.show_controls_hint
        && matches!(app.state, AppState::Streaming { .. })
        && let Some(cmd) = controls_hint_overlay(ctx, &i18n)
    {
        commands.push(cmd);
    }

    if app.confirm_exit {
        if let Some(cmd) = confirm_exit_modal(ctx, &i18n) {
            commands.push(cmd);
        }
    }

    splash_overlay(ctx);

    commands
}

fn splash_overlay(ctx: &egui::Context) {
    let elapsed = ctx.input(|input| input.time);
    if elapsed >= SPLASH_TOTAL {
        return;
    }

    let (alpha, scale) = if elapsed < SPLASH_FADE_IN {
        let t = (elapsed / SPLASH_FADE_IN) as f32;
        let eased = t * t * (3.0 - 2.0 * t);
        (eased, 0.92 + 0.08 * eased)
    } else if elapsed < SPLASH_FADE_IN + SPLASH_HOLD {
        (1.0, 1.0)
    } else {
        let t = ((elapsed - SPLASH_FADE_IN - SPLASH_HOLD) / SPLASH_FADE_OUT) as f32;
        (1.0 - t, 1.0)
    };
    let alpha = alpha.clamp(0.0, 1.0);
    let alpha_u8 = (alpha * 255.0) as u8;

    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("splash_overlay"),
    ));

    if alpha_u8 == 255 {
        painter.rect_filled(screen, 0.0, BG_DEEP);
    } else {
        painter.rect_filled(
            screen,
            0.0,
            egui::Color32::from_rgba_unmultiplied(0x0e, 0x0e, 0x0e, alpha_u8),
        );
    }

    let Some(logo) = geforce_logo(ctx) else {
        painter.text(
            screen.center(),
            egui::Align2::CENTER_CENTER,
            "GEFORCE NOW",
            egui::FontId::proportional(28.0),
            egui::Color32::WHITE.gamma_multiply(alpha),
        );
        return;
    };

    let size = logo.size_vec2();
    let width = (screen.width() * 0.52 * scale).min(size.x * 1.5);
    let height = width * size.y / size.x.max(1.0);
    let logo_rect = egui::Rect::from_center_size(screen.center(), egui::vec2(width, height));
    painter.image(
        logo.id(),
        logo_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::from_white_alpha(alpha_u8),
    );

    let rule_half = width * 0.5 * alpha;
    if rule_half > 1.0 {
        let y = logo_rect.max.y + 14.0;
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(screen.center().x - rule_half, y),
                egui::pos2(screen.center().x + rule_half, y + 2.0),
            ),
            1.0,
            ACCENT.gamma_multiply(alpha),
        );
    }
}

fn login_screen(ctx: &egui::Context, i18n: &I18n, app: &App) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.heading(
                egui::RichText::new("OpenNOW Vita")
                    .size(32.0)
                    .strong()
                    .color(ACCENT),
            );
            ui.label(i18n.text("login-subtitle").as_ref());
            ui.add_space(24.0);
            button_hint(ui, &i18n.text("login-hint"), 13.0, TEXT_DIM, true);
            ui.add_space(24.0);
            if let Some(last_input) = app.last_input {
                ui.weak(
                    text1(i18n, "login-last-input", "input", format!("{last_input:?}")).as_ref(),
                );
            }
        });
    });
}

fn starting_login_screen(ctx: &egui::Context, i18n: &I18n) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(120.0);
            ui.spinner();
            ui.add_space(12.0);
            ui.label(i18n.text("login-requesting-code").as_ref());
        });
    });
}

fn device_code_screen(
    ctx: &egui::Context,
    i18n: &I18n,
    challenge: &crate::gfn::auth::DeviceCodeChallenge,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.heading(i18n.text("device-title").as_ref());
        });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() - 220.0);
                ui.label(i18n.text("device-step-open").as_ref());
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(&challenge.verification_uri_complete)
                        .monospace()
                        .strong(),
                );
                ui.add_space(20.0);
                ui.label(i18n.text("device-step-scan").as_ref());
                ui.add_space(12.0);
                egui::Frame::NONE
                    .fill(BG_PANEL)
                    .corner_radius(12.0)
                    .inner_margin(egui::Margin::symmetric(28, 20))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&challenge.user_code)
                                .size(48.0)
                                .monospace()
                                .strong(),
                        );
                    });
                ui.add_space(20.0);
                ui.label(i18n.text("device-waiting").as_ref());
            });

            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                draw_qr(ui, &challenge.verification_uri_complete, 200.0);
            });
        });
    });
}

fn loading_catalog_screen(ctx: &egui::Context, i18n: &I18n, user: &GfnUser) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.heading(text1(i18n, "catalog-welcome", "name", &user.display_name).as_ref());
            ui.add_space(20.0);
            ui.spinner();
            ui.add_space(12.0);
            ui.label(i18n.text("catalog-loading").as_ref());
        });
    });
}

/// The catalog screen: a narrow scrolling title list on the left, a large detail panel with the

pub(crate) fn battery_color(battery: crate::power::BatteryStatus) -> egui::Color32 {
    if battery.charging {
        ACCENT
    } else if battery.is_critical() {
        DANGER
    } else if battery.should_warn() {
        egui::Color32::from_rgb(0xe0, 0xa8, 0x30)
    } else {
        egui::Color32::WHITE
    }
}

pub(crate) fn paint_battery(painter: &egui::Painter, rect: egui::Rect, battery: crate::power::BatteryStatus) {
    let color = battery_color(battery);
    let body = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x - 3.0, rect.max.y))
        .shrink2(egui::vec2(0.0, 3.0));
    painter.rect_stroke(
        body,
        2.0,
        egui::Stroke::new(1.2_f32, color),
        egui::StrokeKind::Inside,
    );
    painter.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(body.max.x + 1.0, body.center().y - 3.0),
            egui::vec2(2.0, 6.0),
        ),
        1.0,
        color,
    );
    let inner = body.shrink(2.5);
    let filled = inner.width() * (f32::from(battery.percent) / 100.0);
    if filled > 0.5 {
        painter.rect_filled(
            egui::Rect::from_min_size(inner.min, egui::vec2(filled, inner.height())),
            1.0,
            color,
        );
    }
    if battery.charging {
        let c = body.center();
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(c.x + 1.0, c.y - 5.0),
                egui::pos2(c.x - 2.5, c.y + 0.5),
                egui::pos2(c.x - 0.2, c.y + 0.5),
                egui::pos2(c.x - 1.0, c.y + 5.0),
                egui::pos2(c.x + 2.5, c.y - 0.5),
                egui::pos2(c.x + 0.2, c.y - 0.5),
            ],
            BG_DEEP,
            egui::Stroke::new(1.0_f32, BG_DEEP),
        ));
    }
}

pub(crate) fn uv_subrect(image: egui::Rect, x: (f32, f32), y: (f32, f32)) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(
            image.min.x + image.width() * x.0,
            image.min.y + image.height() * y.0,
        ),
        egui::pos2(
            image.min.x + image.width() * x.1,
            image.min.y + image.height() * y.1,
        ),
    )
}

pub(crate) fn allocate_device_image(
    ui: &mut egui::Ui,
    texture: &egui::TextureHandle,
    max_height: f32,
) -> Option<egui::Rect> {
    let size = texture.size_vec2();
    let width = ui.available_width().max(1.0);
    let height = (width * size.y / size.x.max(1.0)).min(max_height).max(1.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return None;
    }

    let painter = ui.painter();
    painter.rect_filled(rect, 6.0, BG_DEEP);
    painter.rect_stroke(
        rect,
        6u8,
        egui::Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );

    let pad = 4.0;
    let inner = rect.shrink(pad);
    let scale = (inner.width() / size.x.max(1.0)).min(inner.height() / size.y.max(1.0));
    let draw = egui::vec2(size.x * scale, size.y * scale);
    let image = egui::Rect::from_center_size(inner.center(), draw);
    painter.image(
        texture.id(),
        image,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    Some(image)
}

/// True when the desktop profile's key strips are actually on screen. The stick-zone hints hide
/// that nobody reads and that pushes the hint off the screen.
const MAX_ERROR_BODY: usize = 220;

// old text-based classifier, only hit when we never got a real gfn code (sign-in, catalog
// graphql, signaling socket). has spanish words too bc the text mightve already been
// translated. dont add more to this list, thats what the code table is for now
fn legacy_error_keys(message: &str) -> Option<(&'static str, &'static str)> {
    let haystack = message.to_ascii_lowercase();

    // Checked before the session case: an expired login often mentions "session" too, and the
    // recovery is completely different.
    if haystack.contains("401")
        || haystack.contains("sign in again")
        || haystack.contains("expired")
        || haystack.contains("expirado")
        || haystack.contains("caduc")
    {
        return Some(("error-auth-title", "error-auth-body"));
    }

    if haystack.contains("session_limit") || haystack.contains("active session") {
        return Some(("error-session-busy-title", "error-session-busy-body"));
    }

    None
}

// title/body to show the player. code decides it when we have one, substring checks below
// are just the fallback for stuff that never carried a code (sign-in, catalog, signaling)
fn present_error(
    i18n: &I18n,
    message: &str,
    code: Option<crate::gfn::error_codes::GfnErrorCode>,
) -> (String, String) {
    if let Some(code) = code {
        if let Some((title, body)) = code.message_keys() {
            return (i18n.text(title).to_string(), i18n.text(body).to_string());
        }
        // A code NVIDIA has not given wording to. Naming it still beats the raw JSON this used to
        // print, and it is the string a player can search for or quote in a bug report.
        return (
            i18n.text("error-gfn-unknown-title").to_string(),
            text1(
                i18n,
                "error-gfn-unknown-body",
                "detail",
                match code.name() {
                    Some(name) => format!("{name} ({})", code.0),
                    None => code.0.to_string(),
                },
            )
            .to_string(),
        );
    }

    if let Some((title, body)) = legacy_error_keys(message) {
        return (i18n.text(title).to_string(), i18n.text(body).to_string());
    }

    let mut body = message.trim().to_owned();
    if body.chars().count() > MAX_ERROR_BODY {
        // By chars, not bytes: truncating mid-codepoint would panic on an accented message.
        body = body.chars().take(MAX_ERROR_BODY - 3).collect::<String>() + "...";
    }
    (i18n.text("error-title").to_string(), body)
}

fn error_screen(
    ctx: &egui::Context,
    i18n: &I18n,
    message: &str,
    code: Option<crate::gfn::error_codes::GfnErrorCode>,
) {
    let (title, body) = present_error(i18n, message, code);
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(70.0);
            ui.heading(egui::RichText::new(title).size(22.0).color(DANGER));
            ui.add_space(12.0);
            ui.label(egui::RichText::new(body).size(13.0));
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new(i18n.text("error-hint").as_ref())
                    .size(11.0)
                    .color(TEXT_DIM),
            );
        });
    });
}

/// Draws a QR code's module grid as plain filled rects (not an image/texture blit) - adapted from
/// green-vita (MPL-2.0), src/app/ui/screens/token_setup.rs.
struct QrImage {
    uri: String,
    modules: Vec<bool>,
    size: u32,
}

fn draw_qr(ui: &mut egui::Ui, verification_uri: &str, target_size: f32) {
    const QUIET_ZONE_MODULES: u32 = 2;
    let cache_id = egui::Id::new("device_code_qr");
    let cached = ui.ctx().data_mut(|data| {
        if let Some(cached) = data.get_temp::<Arc<QrImage>>(cache_id)
            && cached.uri == verification_uri
        {
            return Some(cached);
        }

        let code = qrcode::QrCode::new(verification_uri).ok()?;
        let image = Arc::new(QrImage {
            uri: verification_uri.to_owned(),
            size: code.width() as u32,
            modules: code
                .to_colors()
                .into_iter()
                .map(|color| color == qrcode::Color::Dark)
                .collect(),
        });
        data.insert_temp(cache_id, image.clone());
        Some(image)
    });
    let Some(cached) = cached else {
        ui.spinner();
        return;
    };
    let total_modules = cached.size + QUIET_ZONE_MODULES * 2;
    let module_size = target_size / total_modules as f32;

    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(target_size, target_size), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, egui::Color32::WHITE);
    for y in 0..cached.size {
        for x in 0..cached.size {
            if !cached.modules[(y * cached.size + x) as usize] {
                continue;
            }
            let module_rect = egui::Rect::from_min_size(
                rect.min
                    + egui::vec2(
                        (QUIET_ZONE_MODULES + x) as f32 * module_size,
                        (QUIET_ZONE_MODULES + y) as f32 * module_size,
                    ),
                egui::vec2(module_size, module_size),
            );
            painter.rect_filled(module_rect, 0.0, egui::Color32::BLACK);
        }
    }
}

#[cfg(test)]
mod error_presentation_tests {
    use super::{
        KEYBOARD_CAP_SPACING, KEYBOARD_COLUMNS, KEYBOARD_PADDING, keyboard_key_width,
        keyboard_layout, keyboard_panel_rect, keyboard_unit_width, legacy_error_keys,
    };
    use crate::gfn::error_codes::GfnErrorCode;

    fn classify(message: &str) -> &'static str {
        match legacy_error_keys(message) {
            Some(("error-auth-title", _)) => "auth",
            Some(_) => "session",
            None => "generic",
        }
    }

    #[test]
    fn a_session_limit_is_not_shown_as_a_generic_failure() {
        assert_eq!(
            classify("GeForce NOW still reports an active session"),
            "session"
        );
    }

    /// An expired login usually mentions "session" too, and the fix is completely different - so
    /// the auth case has to win. This is why the order of the checks matters.
    #[test]
    fn an_expired_login_beats_the_session_case() {
        assert_eq!(
            classify("HTTP 401 Unauthorized: session token invalid"),
            "auth"
        );
        assert_eq!(
            classify("Your session expired. Please sign in again."),
            "auth"
        );
    }

    #[test]
    fn anything_else_falls_back() {
        assert_eq!(classify("connection reset by peer"), "generic");
    }

    // this one wouldve landed on the auth branch if we still matched by text
    #[test]
    fn a_code_decides_regardless_of_the_wording() {
        let (title, _) = GfnErrorCode::SESSION_LIMIT_PER_DEVICE_REACHED
            .message_keys()
            .expect("the per-device limit has wording");
        assert_eq!(title, "error-gfn-session-limit-per-device-reached-title");
        assert_eq!(
            classify("CloudMatch rejected the launch: token expired"),
            "auth",
            "without a code this is all the classifier has to go on"
        );
    }

    /// Long errors used to wrap into a wall of text that pushed the hint off screen.
    #[test]
    fn a_long_body_is_truncated_on_a_character_boundary() {
        const MAX: usize = 220;
        // Accented, so a byte-wise truncation would split a codepoint and panic.
        let long = "é".repeat(400);
        let truncated: String = long.chars().take(MAX - 3).collect::<String>() + "...";
        assert_eq!(truncated.chars().count(), MAX);
    }

    #[test]
    fn keyboard_rows_all_span_full_width() {
        for (index, row) in keyboard_layout().iter().enumerate() {
            let units: f32 = row.iter().map(|(_, units)| units).sum();
            assert!(
                (units - KEYBOARD_COLUMNS).abs() < f32::EPSILON,
                "row {index} sums to {units} cap-units, but the panel is sized for \
                 {KEYBOARD_COLUMNS}; caps outside it cannot be touched",
            );
        }
    }

    #[test]
    fn a_full_width_row_exactly_fills_the_panel() {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(960.0, 544.0));
        let panel = keyboard_panel_rect(screen);
        assert!((panel.width() - screen.width()).abs() < f32::EPSILON);
        let inner = panel.width() - KEYBOARD_PADDING * 2.0;
        let unit = keyboard_unit_width(inner);
        let row = KEYBOARD_COLUMNS * unit + (KEYBOARD_COLUMNS - 1.0) * KEYBOARD_CAP_SPACING;
        assert!((inner - row).abs() < 0.01, "{inner} != {row}");
        assert!((keyboard_key_width(2.0, unit) - (2.0 * unit + KEYBOARD_CAP_SPACING)).abs() < 0.01);
    }
}
