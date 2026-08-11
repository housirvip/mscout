use parking_lot::Mutex;

use mem_scanner_core::pointer::{PointerChain, PointerScanner, resolve_pointer};
use tauri::State;

use crate::AppState;

#[tauri::command]
pub fn pointer_scan(
    target_address: usize,
    max_depth: usize,
    max_offset: usize,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<PointerChain>, String> {
    let app_state = state.lock();
    let process = app_state
        .process
        .as_ref()
        .ok_or_else(|| "No process attached".to_string())?;
    let regions = process.regions().map_err(|e| e.to_string())?;
    let scanner = PointerScanner::new(max_depth, max_offset);
    scanner
        .scan(process.as_ref(), target_address, &regions)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resolve_pointer_cmd(
    chain: PointerChain,
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<usize>, String> {
    let app_state = state.lock();
    let process = app_state
        .process
        .as_ref()
        .ok_or_else(|| "No process attached".to_string())?;
    Ok(resolve_pointer(process.as_ref(), &chain))
}
