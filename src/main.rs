mod app;

use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let demo = args.iter().any(|arg| arg == "--demo");
    let screenshot = args
        .windows(2)
        .find(|pair| pair[0] == "--screenshot")
        .map(|pair| PathBuf::from(&pair[1]));
    if screenshot.is_some() && !demo {
        eprintln!("--screenshot requires --demo to keep captures free of user data");
        std::process::exit(2);
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("OpenSSH Manager")
        .with_inner_size([1440.0, 920.0])
        .with_min_inner_size([1050.0, 700.0]);
    if let Ok(icon) = image::load_from_memory(include_bytes!("../assets/brand/logo.png")) {
        let rgba = icon.to_rgba8();
        viewport = viewport.with_icon(egui::IconData {
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
        });
    }
    eframe::run_native(
        "OpenSSH Manager",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(app::OpenSshApp::new(cc, demo, screenshot)))),
    )
}
