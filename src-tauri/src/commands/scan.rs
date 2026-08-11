use parking_lot::Mutex;

use mem_scanner_core::{
    platform::MemoryRegion,
    scanner::{ScanCondition, ScanResult, ScanSession, ScanValue, ValueType},
};
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;

use crate::AppState;

#[derive(Deserialize)]
pub struct RegionFilter {
    pub writable_only: bool,
    pub skip_executable: bool,
    pub skip_mapped_files: bool,
}

#[derive(Clone, Serialize)]
pub struct ScanProgress {
    pub scanned_bytes: u64,
    pub total_bytes: u64,
    pub regions_done: u32,
    pub total_regions: u32,
}

#[derive(Serialize)]
pub struct ScanSummary {
    pub match_count: usize,
}

fn filter_regions(regions: Vec<MemoryRegion>, filter: &RegionFilter) -> Vec<MemoryRegion> {
    regions
        .into_iter()
        .filter(|r| {
            if filter.writable_only && !r.writable {
                return false;
            }
            if filter.skip_executable && r.executable {
                return false;
            }
            if filter.skip_mapped_files
                && !r.info.is_empty()
                && r.info != "[heap]"
                && r.info != "[stack]"
                && !r.info.starts_with('[')
            {
                return false;
            }
            true
        })
        .collect()
}

#[tauri::command]
pub fn first_scan(
    value_type: ValueType,
    condition: ScanCondition,
    value: Option<ScanValue>,
    value2: Option<ScanValue>,
    region_filter: Option<RegionFilter>,
    channel: Channel<ScanProgress>,
    state: State<'_, Mutex<AppState>>,
) -> Result<ScanSummary, String> {
    let mut app_state = state.lock();
    let process = app_state
        .process
        .as_ref()
        .ok_or_else(|| "No process attached".to_string())?;

    let regions = process.regions().map_err(|e| e.to_string())?;
    let regions = if let Some(filter) = &region_filter {
        filter_regions(regions, filter)
    } else {
        regions
    };

    let alignment = value_type.size().max(1);
    let total_bytes: u64 = regions.iter().map(|r| r.size as u64).sum();

    // Send initial progress
    let _ = channel.send(ScanProgress {
        scanned_bytes: 0,
        total_bytes,
        regions_done: 0,
        total_regions: regions.len() as u32,
    });

    let session = ScanSession::first_scan(
        process.as_ref(),
        value_type,
        condition,
        value,
        value2,
        &regions,
        alignment,
    )
    .map_err(|e| e.to_string())?;

    // Send completion progress
    let _ = channel.send(ScanProgress {
        scanned_bytes: total_bytes,
        total_bytes,
        regions_done: regions.len() as u32,
        total_regions: regions.len() as u32,
    });

    let match_count = session.result_count();
    app_state.scan_session = Some(session);

    Ok(ScanSummary { match_count })
}

#[tauri::command]
pub fn next_scan(
    condition: ScanCondition,
    value: Option<ScanValue>,
    value2: Option<ScanValue>,
    state: State<'_, Mutex<AppState>>,
) -> Result<ScanSummary, String> {
    let mut app_state = state.lock();
    let process = app_state
        .process
        .clone()
        .ok_or_else(|| "No process attached".to_string())?;

    let session = app_state
        .scan_session
        .as_mut()
        .ok_or_else(|| "No scan session active".to_string())?;

    session
        .next_scan(process.as_ref(), condition, value, value2)
        .map_err(|e| e.to_string())?;

    let match_count = session.result_count();
    Ok(ScanSummary { match_count })
}

#[tauri::command]
pub fn undo_scan(state: State<'_, Mutex<AppState>>) -> Result<ScanSummary, String> {
    let mut app_state = state.lock();
    let session = app_state
        .scan_session
        .as_mut()
        .ok_or_else(|| "No scan session active".to_string())?;

    session.undo();
    let match_count = session.result_count();
    Ok(ScanSummary { match_count })
}

#[tauri::command]
pub fn get_scan_results(
    offset: usize,
    count: usize,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<ScanResult>, String> {
    let app_state = state.lock();
    let process = app_state
        .process
        .as_ref()
        .ok_or_else(|| "No process attached".to_string())?;

    let session = app_state
        .scan_session
        .as_ref()
        .ok_or_else(|| "No scan session active".to_string())?;

    session
        .get_results(process.as_ref(), offset, count)
        .map_err(|e| e.to_string())
}
