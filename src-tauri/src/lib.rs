//! Tauri shell. Keep commands thin: validate input, call into the
//! `booker-*` crates, return serialisable results.

mod commands;
mod error;

use booker_store::SqliteStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // e.g. ~/.local/share/com.desktopbooker.app/booker.db on Linux
            let db_path = app.path().app_data_dir()?.join("booker.db");
            // `setup` isn't async, so wait for the database to open here.
            let store = tauri::async_runtime::block_on(SqliteStore::open(&db_path))?;
            println!("[desktop-booker] database: {}", db_path.display());
            app.manage(store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::get_settings,
            commands::save_settings,
            commands::list_floors,
            commands::save_floor,
            commands::delete_floor,
            commands::list_room_types,
            commands::save_room_type,
            commands::delete_room_type,
            commands::list_spaces,
            commands::save_space,
            commands::delete_space,
            commands::seed_demo_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
