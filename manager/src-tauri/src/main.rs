// Release builds: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gitops;
mod handoff;
mod keys;
mod licenses;
mod worker;

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running AlurianOT License Manager");
}
