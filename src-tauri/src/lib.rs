use parking_lot::Mutex;

use mem_scanner_core::{
    freeze::FreezeManager,
    platform::ProcessMemory,
    scanner::ScanSession,
};
use std::sync::Arc;

pub mod commands;

pub struct AppState {
    pub process: Option<Arc<dyn ProcessMemory + Send + Sync>>,
    pub scan_session: Option<ScanSession>,
    pub freeze_manager: Option<FreezeManager>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(AppState {
            process: None,
            scan_session: None,
            freeze_manager: None,
        }))
        .invoke_handler(tauri::generate_handler![
            commands::process::list_processes,
            commands::process::attach_process,
            commands::process::detach_process,
            commands::scan::first_scan,
            commands::scan::next_scan,
            commands::scan::undo_scan,
            commands::scan::get_scan_results,
            commands::memory::read_at,
            commands::memory::write_at,
            commands::freeze::add_frozen,
            commands::freeze::remove_frozen,
            commands::freeze::toggle_frozen,
            commands::freeze::list_frozen,
            commands::pointer::pointer_scan,
            commands::pointer::resolve_pointer_cmd,
            commands::table::save_table,
            commands::table::load_table,
            commands::vm::list_vms,
            commands::vm::attach_vm,
            commands::vm::attach_vm_process,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
