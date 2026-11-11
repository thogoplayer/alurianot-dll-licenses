// Release builds: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod gitops;
mod handoff;
mod keys;
mod licenses;
mod settings;
mod worker;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_licenses,
            commands::pull_latest,
            commands::create_license,
            commands::set_revoked,
            commands::set_features,
            commands::set_expires,
            commands::delete_license,
            commands::retry_push,
            commands::pending_push,
            commands::config_text,
            commands::prepare_handoff,
            commands::open_key_folder,
            commands::token_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AlurianOT License Manager");
}
