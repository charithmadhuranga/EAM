//! ESPHome Automation Manager - Desktop Application Entry Point

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod app;
mod commands;

use tauri::Manager as _;

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

            // Bootstrap the persistent runtime (DB, registry, connection manager).
            let state = tauri::async_runtime::block_on(app::AppState::bootstrap(&handle))?;
            handle.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::device::get_devices,
            commands::device::register_device,
            commands::device::register_manual,
            commands::device::register_discovered,
            commands::device::unregister_device,
            commands::device::connect_device,
            commands::device::disconnect_device,
            commands::device::scan_devices,
            commands::device::probe_device,
            commands::device::get_entities,
            commands::device::get_entity_states,
            commands::device::control_switch,
            commands::device::set_number,
            commands::runtime::get_runtime_status,
            commands::runtime::get_workspace_info,
        ])
        .on_window_event(|window, event| {
            // Flush and stop the runtime when the window closes.
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(state) = window.try_state::<app::AppState>() {
                    tauri::async_runtime::block_on(state.shutdown());
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
