//! Tauri shell. Keep commands thin: validate input, call into the
//! `booker-*` crates, return serialisable results.

mod commands;
mod error;
mod session;

use booker_store::{BackupRepo, EventRepo, SettingsRepo, SqliteStore};
use session::Session;
use tauri::{Manager, RunEvent};

/// Where the live database file is, for restoring a backup over it.
pub struct DbPath(pub std::path::PathBuf);

/// The folder backups go to: the chosen one, or `<app data>/backups`.
pub fn backup_dir(app: &tauri::AppHandle, folder: &str) -> std::path::PathBuf {
    if folder.trim().is_empty() {
        app.path()
            .app_data_dir()
            .map(|d| d.join("backups"))
            .unwrap_or_else(|_| std::path::PathBuf::from("backups"))
    } else {
        std::path::PathBuf::from(folder.trim())
    }
}

/// Takes a backup if the last one is overdue. Errors are logged, not fatal.
fn backup_if_due(app: &tauri::AppHandle) {
    let Some(store) = app.try_state::<SqliteStore>() else {
        return;
    };
    let run = async {
        let settings = store.get_backup_settings().await?;
        let dir = backup_dir(app, &settings.folder);
        let last = booker_store::list_backups(&dir)?
            .first()
            .map(|b| b.taken_at);
        if booker_core::backup_overdue(last, chrono::Local::now().naive_local()) {
            let made = store.backup_now(&dir, &settings).await?;
            println!("[desktop-booker] daily backup: {}", made.name);
        }
        Ok::<(), booker_store::StoreError>(())
    };
    if let Err(e) = tauri::async_runtime::block_on(run) {
        eprintln!("[desktop-booker] scheduled backup failed: {e}");
    }
}

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
            app.manage(DbPath(db_path.clone()));
            // A daily backup while the app is open: check now, then every 30 minutes.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    backup_if_due(&handle);
                    std::thread::sleep(std::time::Duration::from_secs(30 * 60));
                }
            });
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
            commands::list_housekeeping,
            commands::set_housekeeping,
            commands::occupancy_report,
            commands::revenue_report,
            commands::report_csv,
            commands::guest_profile,
            commands::erase_guest,
            commands::backup_status,
            commands::save_backup_settings,
            commands::backup_now,
            commands::restore_backup,
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
    let Some(store) = app.try_state::<SqliteStore>() else {
        return;
    };
    let run = async {
        let settings = store.get_backup_settings().await?;
        let target = backup_dir(app, &settings.folder);
        store.backup_now(&target, &settings).await
    };
    match tauri::async_runtime::block_on(run) {
        Ok(made) => println!("[desktop-booker] backup on exit: {}", made.name),
        Err(e) => eprintln!("[desktop-booker] backup failed: {e}"),
    }
}
