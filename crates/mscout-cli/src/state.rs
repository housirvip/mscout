use std::sync::Arc;

use mscout_core::{
    freeze::FreezeManager,
    platform::ProcessMemory,
    scanner::ScanSession,
};

/// In-process state for the CLI, mirrors Tauri's AppState.
pub struct CliState {
    pub process: Option<Arc<dyn ProcessMemory + Send + Sync>>,
    pub scan_session: Option<ScanSession>,
    pub freeze_manager: Option<FreezeManager>,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

impl CliState {
    pub fn new() -> Self {
        Self {
            process: None,
            scan_session: None,
            freeze_manager: None,
            pid: None,
            process_name: None,
        }
    }
}

impl Default for CliState {
    fn default() -> Self {
        Self::new()
    }
}
