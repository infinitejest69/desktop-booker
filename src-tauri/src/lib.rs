//! Tauri shell. Keep commands thin: validate input, call into the
//! `booker-*` crates, return serialisable results.

mod commands;
mod error;

use booker_store::SqliteStore;
use tauri::{Manager, RunEvent};

/// How many automatic on-exit backups to keep (Phase 7 adds scheduled ones).
const BACKUPS_KEPT: usize = 10;

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
            commands::list_rate_rules,
            commands::save_rate_rule,
            commands::delete_rate_rule,
            commands::list_board_options,
            commands::save_board_option,
            commands::delete_board_option,
            commands::search_guests,
            commands::quote_room,
            commands::free_rooms,
            commands::chart,
            commands::get_booking,
            commands::save_booking,
            commands::set_booking_status,
            commands::search_bookings,
            commands::seed_demo_data,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        // `run` takes a closure called for every app event; we only care about exit.
        .run(|app, event| {
            if let RunEvent::Exit = event {
                backup_on_exit(app);
            }
        });
}

/// Copies the database into `<app data>/backups/`, keeping the newest few.
fn backup_on_exit(app: &tauri::AppHandle) {
    let (Some(store), Ok(dir)) = (app.try_state::<SqliteStore>(), app.path().app_data_dir()) else {
        return;
    };
    let dir = dir.join("backups");
    match tauri::async_runtime::block_on(store.backup_with_rotation(&dir, BACKUPS_KEPT)) {
        Ok(path) => println!("[desktop-booker] backup: {}", path.display()),
        Err(e) => eprintln!("[desktop-booker] backup failed: {e}"),
    }
}
