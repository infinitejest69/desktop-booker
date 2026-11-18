//! Tauri shell. Keep commands thin: validate input, call into the
//! `booker-*` crates, return serialisable results.

/// Returns the app version from the core crate (smoke test of the
/// frontend → Rust → workspace crate wiring).
#[tauri::command]
fn app_version() -> String {
    booker_core::version().to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![app_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
