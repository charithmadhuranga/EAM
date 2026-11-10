//! ESPHome Automation Manager - Desktop Application Entry Point

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod app;
mod commands;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("esphome_automation_manager=debug".parse().unwrap()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            // Initialize application state
            app::initialize(handle)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::device::get_devices,
            commands::device::discover_devices,
            commands::runtime::get_runtime_status,
            commands::runtime::get_workspace_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
