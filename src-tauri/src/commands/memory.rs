use parking_lot::Mutex;

use mscout_core::platform::MemoryRegion;
use mscout_core::scanner::ScanValue;
use serde::Serialize;
use tauri::State;

use crate::AppState;

#[derive(Serialize)]
pub struct MemoryBytes {
    pub address: usize,
    pub bytes: Vec<u8>,
}

#[tauri::command]
pub fn read_at(
    address: usize,
    size: usize,
    state: State<'_, Mutex<AppState>>,
) -> Result<MemoryBytes, String> {
    let app_state = state.lock();
    let process = app_state
        .process
        .as_ref()
        .ok_or_else(|| "No process attached".to_string())?;

    let mut buf = vec![0u8; size];
    let bytes_read = process.read(address, &mut buf).map_err(|e| e.to_string())?;
    buf.truncate(bytes_read);

    Ok(MemoryBytes {
        address,
        bytes: buf,
    })
}

#[tauri::command]
pub fn write_at(
    address: usize,
    value: ScanValue,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let app_state = state.lock();
    let process = app_state
        .process
        .as_ref()
        .ok_or_else(|| "No process attached".to_string())?;

    let bytes = value.to_bytes();
    process.write(address, &bytes).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_regions(state: State<'_, Mutex<AppState>>) -> Result<Vec<MemoryRegion>, String> {
    let app_state = state.lock();
    let process = app_state
        .process
        .as_ref()
        .ok_or_else(|| "No process attached".to_string())?;
    process.regions().map_err(|e| e.to_string())
}
