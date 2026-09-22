#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod args;
mod assets_image;
mod banners;
mod config;
mod download;
mod events;
mod install;
mod logger;
mod manifest;
mod path_utils;
mod process_wait;
mod relaunch;
mod ui;
mod updater;
mod verify;
mod zip_extract;

use eframe::egui;
use std::sync::mpsc;

fn window_icon() -> Option<egui::IconData> {
    let (width, height, rgba) = assets_image::decode_rgba(assets_image::DEVELOPER_LOGO_PNG)?;
    Some(egui::IconData { rgba, width: width as u32, height: height as u32 })
}

fn main() -> eframe::Result<()> {
    let launch = args::LaunchArgs::parse(std::env::args().skip(1));
    let logger = logger::Logger::new();
    logger.configure(&launch.app_id, &launch.current_version);
    logger.info(&format!("Argumentos: {}", launch.to_dictionary()));

    let (tx, rx) = mpsc::channel::<events::Event>();

    {
        let tx = tx.clone();
        let logger = logger.clone();
        let launch = launch.clone();
        std::thread::spawn(move || updater::run(launch, tx, logger));
    }
    {
        let tx = tx.clone();
        let logger = logger.clone();
        let app_id = launch.app_id.clone();
        std::thread::spawn(move || banners::load_banners(app_id, tx, logger));
    }
    drop(tx);

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("GGUpdater")
        .with_inner_size([800.0, 700.0])
        .with_min_inner_size([640.0, 560.0]);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "GGUpdater",
        options,
        Box::new(move |cc| Ok(Box::new(ui::UpdaterApp::new(cc, launch, rx, logger)))),
    )
}
