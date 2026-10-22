mod window_placement;

use wyn_observatory_core::{FleetClient, FleetFeed};

#[tauri::command]
fn fleet_snapshot(client: tauri::State<'_, FleetClient>) -> FleetFeed {
    client.snapshot()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_filename(window_placement::STATE_FILENAME)
                .skip_initial_state("main")
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED,
                )
                .build(),
        )
        .setup(|app| {
            use tauri::Manager;
            let endpoint =
                wyn_observatory_core::configured_endpoint().map_err(std::io::Error::other)?;
            app.manage(FleetClient::connect(endpoint));
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            if let Some(window) = app.get_webview_window("main") {
                window_placement::restore_and_show(&window)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![fleet_snapshot])
        .run(tauri::generate_context!())
        .expect("error while running WynObserve");
}
