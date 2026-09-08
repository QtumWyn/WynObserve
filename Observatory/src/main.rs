mod analysis;
mod app;
mod config;
mod extended_analysis;
mod fleet;
mod hub;
mod live;
mod mock;
mod model;
mod normalize;
mod theme;
mod ui;
mod runtime_config;

use app::ObservatoryApp;
use eframe::egui;

fn main() -> eframe::Result {
    let config =
        runtime_config::ObservatoryConfig::load()
            .expect(
                "failed to load Observatory configuration"
            );

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WynCommand // Observatory")
            .with_inner_size([1580.0, 980.0])
            .with_min_inner_size([1080.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "WynCommand Observatory",
        native_options,
        Box::new(move |cc| {
            Ok(
                Box::new(
                    ObservatoryApp::new(
                        cc,
                        config.clone(),
                    )
                )
            )
        }),
    )
}
