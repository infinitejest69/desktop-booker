//! Tauri shell. Keep commands thin: validate input, call into the
//! `booker-*` crates, return serialisable results.

mod commands;
mod error;
mod session;

use booker_store::{EventRepo, SettingsRepo, SqliteStore};
use session::Session;
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
            let idle = tauri::async_runtime::block_on(store.get_settings())
                .map(|s| s.idle_lock_minutes)
                .unwrap_or(15);
            // Rooms held for events go back on sale once their release date comes.
            let today = chrono::Local::now().date_naive();
            match tauri::async_runtime::block_on(store.release_due_blocks(today)) {
                Ok(0) => {}
                Ok(n) => println!("[desktop-booker] released {n} held block room(s)"),
                Err(e) => eprintln!("[desktop-booker] couldn't release held rooms: {e}"),
            }
            app.manage(Session::new(idle));
            app.manage(store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::session_status,
            commands::create_first_admin,
            commands::login,
            commands::logout,
            commands::list_users,
            commands::save_user,
            commands::list_audit,
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
            commands::get_suite_rates,
            commands::save_suite_rates,
            commands::list_suite_bookings,
            commands::get_suite_booking,
            commands::save_suite_booking,
            commands::set_suite_booking_status,
            commands::list_events,
            commands::get_event,
            commands::save_event,
            commands::list_tax_rates,
            commands::save_tax_rate,
            commands::get_invoice_settings,
            commands::save_invoice_settings,
            commands::get_folio,
            commands::add_charge,
            commands::remove_charge,
            commands::add_payment,
            commands::issue_invoice,
            commands::get_invoice,
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
