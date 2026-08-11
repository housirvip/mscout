use parking_lot::Mutex;

use mscout_core::platform::{NativeProcess, ProcessInfo, ProcessMemory};
use std::sync::Arc;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub fn list_processes() -> Result<Vec<ProcessInfo>, String> {
    NativeProcess::list_processes().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn attach_process(pid: u32, state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let process = NativeProcess::attach(pid).map_err(|e| e.to_string())?;
    let mut app_state = state.lock();
    app_state.process = Some(Arc::new(process) as Arc<dyn ProcessMemory + Send + Sync>);
    Ok(())
}

#[tauri::command]
pub fn detach_process(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut app_state = state.lock();
    // Stop freeze manager if running
    if let Some(mut fm) = app_state.freeze_manager.take() {
        fm.stop();
    }
    app_state.scan_session = None;
    app_state.process = None;
    Ok(())
}
