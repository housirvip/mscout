use parking_lot::Mutex;

use mem_scanner_core::{
    freeze::FreezeManager,
    scanner::ScanValue,
};
use tauri::State;

use crate::AppState;

#[tauri::command]
pub fn add_frozen(
    address: usize,
    value: ScanValue,
    label: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let mut app_state = state.lock();

    // Create freeze manager if it doesn't exist
    if app_state.freeze_manager.is_none() {
        let process = app_state
            .process
            .as_ref()
            .ok_or_else(|| "No process attached".to_string())?;
        app_state.freeze_manager = Some(FreezeManager::start(process.clone()));
    }

    let fm = app_state.freeze_manager.as_ref().unwrap();
    fm.add(address, value, label);

    Ok(())
}

#[tauri::command]
pub fn remove_frozen(address: usize, state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let app_state = state.lock();
    let fm = app_state
        .freeze_manager
        .as_ref()
        .ok_or_else(|| "Freeze manager not active".to_string())?;

    fm.remove(address);
    Ok(())
}

#[tauri::command]
pub fn toggle_frozen(
    address: usize,
    enabled: bool,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let app_state = state.lock();
    let fm = app_state
        .freeze_manager
        .as_ref()
        .ok_or_else(|| "Freeze manager not active".to_string())?;

    fm.set_enabled(address, enabled);
    Ok(())
}

#[tauri::command]
pub fn list_frozen(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<mem_scanner_core::freeze::FrozenEntry>, String> {
    let app_state = state.lock();
    match &app_state.freeze_manager {
        Some(fm) => Ok(fm.list()),
        None => Ok(Vec::new()),
    }
}
