use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText, TextureHandle, TextureOptions, Vec2};

use crate::args::LaunchArgs;
use crate::assets_image;
use crate::browser;
use crate::events::{BannerImage, Event, State, StateData};
use crate::logger::Logger;
use crate::relaunch;

const COLOR_BG: Color32 = Color32::from_rgb(0x14, 0x16, 0x23);
const COLOR_PANEL: Color32 = Color32::from_rgb(0x1d, 0x21, 0x33);
const COLOR_ACCENT: Color32 = Color32::from_rgb(0x4f, 0x8c, 0xff);
const COLOR_ERROR: Color32 = Color32::from_rgb(0xff, 0x5c, 0x7a);
const COLOR_TEXT: Color32 = Color32::from_rgb(0xe6, 0xe9, 0xf5);
const COLOR_MUTED: Color32 = Color32::from_rgb(0x8b, 0x93, 0xad);

const ROTATION_INTERVAL: f64 = 3.0;
const FADE_DURATION: f32 = 0.25;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FadePhase {
    Out,
    In,
}

struct Fade {
    next: usize,
    phase: FadePhase,
    start: Instant,
}

#[derive(Clone)]
struct Banner {
    texture: TextureHandle,
    link: Option<String>,
}

pub struct UpdaterApp {
    state: State,
    status: String,
    status_color: Color32,
    version_text: String,
    progress: f32,
    percent: String,
    show_ok: bool,
    show_retry: bool,
    banners: Vec<Banner>,
    banner_index: usize,
    banner_count: Option<usize>,
    last_rotation: Instant,
    fade_alpha: f32,
    fade: Option<Fade>,
    logo: Option<TextureHandle>,
    rx: Receiver<Event>,
    logger: Logger,
    args: LaunchArgs,
}

impl UpdaterApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        args: LaunchArgs,
        rx: Receiver<Event>,
        logger: Logger,
    ) -> Self {
        apply_theme(&cc.egui_ctx);

        let logo = assets_image::to_color_image(assets_image::DEVELOPER_LOGO_PNG)
            .map(|image| cc.egui_ctx.load_texture("developer_logo", image, TextureOptions::LINEAR));

        Self {
            state: State::Starting,
            status: "Starting updater...".to_string(),
            status_color: COLOR_TEXT,
            version_text: String::new(),
            progress: 0.0,
            percent: "0%".to_string(),
            show_ok: false,
            show_retry: false,
            banners: Vec::new(),
            banner_index: 0,
            banner_count: None,
            last_rotation: Instant::now(),
            fade_alpha: 1.0,
            fade: None,
            logo,
            rx,
            logger,
            args,
        }
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::State(state, data) => self.apply_state(state, data),
                Event::Progress { ratio } => {
                    self.progress = ratio;
                    self.percent = format_percent(ratio);
                }
                Event::Versions { current, target } => {
                    self.version_text = version_text(&current, &target);
                }
                Event::BannersPartial(images) | Event::BannersReady(images) => {
                    self.load_banners(ctx, images);
                    if self.state == State::Preview {
                        self.refresh_preview_message();
                    }
                }
            }
        }
    }

    fn apply_state(&mut self, state: State, data: StateData) {
        self.state = state;
        self.status = data.message.clone();

        match state {
            State::Starting | State::LoadingManifest | State::LoadingBanners => {
                self.progress = 0.0;
                self.percent = "0%".to_string();
                self.show_ok = false;
                self.show_retry = false;
                self.status_color = COLOR_TEXT;
            }
            State::Downloading | State::Verifying | State::Extracting | State::Installing => {
                self.show_ok = false;
                self.show_retry = false;
            }
            State::Completed => {
                self.progress = 1.0;
                self.percent = "100%".to_string();
                self.show_ok = true;
                self.show_retry = false;
                self.status_color = COLOR_TEXT;
                self.status = format!("✓ {}", self.status);
                if !data.up_to_date {
                    self.version_text = version_text(&data.current, &data.target);
                }
            }
            State::Error => {
                self.show_ok = false;
                self.show_retry = true;
                self.status_color = COLOR_ERROR;
                self.status = format!("⚠ {}", self.status);
            }
            State::Preview => {
                self.progress = 0.0;
                self.percent = String::new();
                self.show_ok = false;
                self.show_retry = false;
                self.status_color = COLOR_MUTED;
                self.version_text = "Preview de interfaz".to_string();
                self.refresh_preview_message();
            }
        }
    }

    fn refresh_preview_message(&mut self) {
        let mut message = "Modo preview (sin --app)".to_string();
        if let Some(count) = self.banner_count {
            if count == 0 {
                message.push_str("\nNo hay banners en caché. Configura BANNERS_MANIFEST_URL.");
            } else {
                message.push_str(&format!("\n{count} banners cargados."));
            }
        }
        self.status = message;
    }

    fn load_banners(&mut self, ctx: &egui::Context, images: Vec<BannerImage>) {
        self.banners.clear();
        for (index, banner) in images.into_iter().enumerate() {
            self.banners.push(Banner {
                texture: ctx.load_texture(format!("banner_{index}"), banner.image, TextureOptions::LINEAR),
                link: banner.link,
            });
        }
        self.banner_index = 0;
        self.banner_count = Some(self.banners.len());
        self.last_rotation = Instant::now();
        self.fade_alpha = 1.0;
        self.fade = None;
    }

    fn start_transition(&mut self, next: usize) {
        if self.banners.len() <= 1 || next == self.banner_index || self.fade.is_some() {
            return;
        }
        self.fade = Some(Fade { next, phase: FadePhase::Out, start: Instant::now() });
    }

    fn tick_animation(&mut self, ctx: &egui::Context) {
        if self.fade.is_none()
            && self.banners.len() > 1
            && self.last_rotation.elapsed().as_secs_f64() >= ROTATION_INTERVAL
        {
            let next = (self.banner_index + 1) % self.banners.len();
            self.start_transition(next);
        }

        let active = self.fade.as_ref().map(|fade| (fade.phase, fade.next, fade.start));
        if let Some((phase, next, start)) = active {
            let progress = (start.elapsed().as_secs_f32() / FADE_DURATION).clamp(0.0, 1.0);
            match phase {
                FadePhase::Out => {
                    if progress >= 1.0 {
                        self.banner_index = next;
                        self.fade = Some(Fade { next, phase: FadePhase::In, start: Instant::now() });
                        self.fade_alpha = 0.0;
                    } else {
                        self.fade_alpha = 1.0 - smoothstep(progress);
                    }
                }
                FadePhase::In => {
                    if progress >= 1.0 {
                        self.fade = None;
                        self.fade_alpha = 1.0;
                        self.last_rotation = Instant::now();
                    } else {
                        self.fade_alpha = smoothstep(progress);
                    }
                }
            }
        }

        if self.fade.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else if self.banners.len() > 1 {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn draw_logo(&self, ui: &mut egui::Ui) {
        if let Some(logo) = &self.logo {
            let size = logo.size_vec2();
            let height = 40.0;
            let width = if size.y > 0.0 { height * size.x / size.y } else { height };
            ui.vertical_centered(|ui| {
                ui.add(egui::Image::new(logo).fit_to_exact_size(Vec2::new(width, height)));
            });
        }
    }

    fn draw_banner(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(COLOR_PANEL)
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
                ui.set_min_height((ui.available_height() - 8.0).max(120.0));
                ui.vertical(|ui| {
                    let available = ui.available_size();
                    let banner_height = (available.y - 26.0).max(100.0);
                    let banner = self.banners.get(self.banner_index).cloned();
                    let logger = self.logger.clone();
                    let alpha = (self.fade_alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
                    let tint = Color32::from_white_alpha(alpha);
                    ui.allocate_ui(Vec2::new(available.x, banner_height), |ui| {
                        let Some(banner) = banner else {
                            return;
                        };
                        let size = fit_size(banner.texture.size_vec2(), ui.available_size());
                        let response = ui
                            .vertical_centered(|ui| {
                                ui.add(
                                    egui::Image::new(&banner.texture)
                                        .fit_to_exact_size(size)
                                        .tint(tint)
                                        .sense(egui::Sense::click()),
                                )
                            })
                            .inner;
                        if let Some(link) = &banner.link {
                            if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                                && !browser::open_url(link)
                            {
                                logger.warn(&format!("No se pudo abrir el enlace del banner: {link}"));
                            }
                        }
                    });

                    let count = self.banners.len();
                    if count > 1 {
                        let current = self.banner_index;
                        let mut selected = None;
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            let button = 24.0;
                            let gap = 2.0;
                            let total = count as f32 * button + (count as f32 - 1.0) * gap;
                            ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                            for index in 0..count {
                                let is_selected = index == current;
                                let text = if is_selected { "●" } else { "○" };
                                let color = if is_selected { COLOR_ACCENT } else { COLOR_MUTED };
                                let response = ui.add(
                                    egui::Button::new(RichText::new(text).color(color))
                                        .frame(false)
                                        .min_size(Vec2::new(button, button)),
                                );
                                if response.clicked() {
                                    selected = Some(index);
                                }
                            }
                        });
                        if let Some(index) = selected {
                            self.start_transition(index);
                        }
                    }
                });
            });
    }

    fn draw_info(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::Frame::new()
            .fill(COLOR_PANEL)
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let status = self.status.clone();
                let status_color = self.status_color;
                ui.vertical_centered(|ui| {
                    ui.set_min_height(40.0);
                    ui.label(RichText::new(status).color(status_color).size(15.0));
                });

                ui.add_space(6.0);
                let version_text = self.version_text.clone();
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(version_text).color(COLOR_MUTED));
                });

                ui.add_space(6.0);
                ui.add(egui::ProgressBar::new(self.progress).desired_height(18.0).text(""));

                ui.add_space(6.0);
                let percent = self.percent.clone();
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(percent).color(COLOR_MUTED));
                });

                ui.add_space(6.0);
                let show_ok = self.show_ok;
                let show_retry = self.show_retry;
                let mut pressed_ok = false;
                let mut pressed_close = false;
                ui.horizontal(|ui| {
                    let count = show_ok as u8 + show_retry as u8;
                    if count > 0 {
                        let button = 110.0;
                        let gap = 12.0;
                        let total = count as f32 * button + (count as f32 - 1.0) * gap;
                        ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                        if show_ok && ui.add(egui::Button::new("OK").min_size(Vec2::new(button, 32.0))).clicked() {
                            pressed_ok = true;
                        }
                        if show_retry
                            && ui
                                .add(egui::Button::new("Cerrar").min_size(Vec2::new(button, 32.0)))
                                .clicked()
                        {
                            pressed_close = true;
                        }
                    }
                });

                if pressed_ok {
                    relaunch::relaunch(&self.args.target_root, &self.args.executable, &self.logger);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if pressed_close {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
    }
}

impl eframe::App for UpdaterApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.drain_events(&ctx);
        self.tick_animation(&ctx);

        // Logo anclado arriba: el panel de info (abajo) siempre queda visible y el
        // carrusel ocupa el espacio restante, sin poder empujar los botones fuera.
        egui::Panel::top("ggupdater_logo")
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(COLOR_BG).inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 12,
                bottom: 0,
            }))
            .show(ui, |ui| self.draw_logo(ui));

        egui::Panel::bottom("ggupdater_info")
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(COLOR_BG).inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 10,
                bottom: 12,
            }))
            .show(ui, |ui| self.draw_info(ui, &ctx));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(COLOR_BG).inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 10,
                bottom: 0,
            }))
            .show(ui, |ui| self.draw_banner(ui));
    }
}

fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = COLOR_BG;
    visuals.panel_fill = COLOR_BG;
    visuals.override_text_color = Some(COLOR_TEXT);
    visuals.selection.bg_fill = COLOR_ACCENT;
    ctx.set_visuals(visuals);
}

fn version_text(current: &str, target: &str) -> String {
    let left = if current.is_empty() { "?" } else { current };
    if target.is_empty() {
        format!("Versión actual {left}")
    } else {
        format!("{left} → {target}")
    }
}

fn format_percent(ratio: f32) -> String {
    format!("{}%", (ratio * 100.0).round() as i32)
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn fit_size(original: Vec2, available: Vec2) -> Vec2 {
    if original.x <= 0.0 || original.y <= 0.0 || available.x <= 0.0 || available.y <= 0.0 {
        return Vec2::ZERO;
    }
    let scale = (available.x / original.x).min(available.y / original.y);
    original * scale
}
