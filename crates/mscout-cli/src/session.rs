use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mscout_core::{
    platform::MemoryRegion,
    scanner::ValueType,
    table::CheatTable,
};
use serde::{Deserialize, Serialize};

/// Serializable session file for subcommand mode persistence.
#[derive(Serialize, Deserialize)]
pub struct SessionFile {
    pub version: u32,
    pub pid: u32,
    pub process_name: String,
    pub vm_pid: Option<u32>,
    pub guest_pid: Option<u32>,
    pub scan_state: Option<ScanState>,
    pub frozen_pids: Vec<u32>,
    pub table: Option<CheatTable>,
}

/// Serializable scan state.
#[derive(Serialize, Deserialize)]
pub struct ScanState {
    pub value_type: ValueType,
    pub alignment: usize,
    pub addresses: Vec<usize>,
    pub previous_values: Vec<u8>,
    pub history: Vec<(Vec<usize>, Vec<u8>)>,
    pub regions: Vec<MemoryRegion>,
}

/// Get the session directory.
fn session_dir() -> PathBuf {
    let tmp = std::env::temp_dir();
    tmp.join("mscout")
}

/// Get the canonical session file path for a PID.
pub fn session_path_for_pid(pid: u32) -> PathBuf {
    session_dir().join(format!("session-{pid}.bin"))
}

/// Find the most recent active session file.
pub fn find_active_session() -> Option<PathBuf> {
    let dir = session_dir();
    if !dir.exists() {
        return None;
    }
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with("session-") && n.ends_with(".bin"))
                .unwrap_or(false)
        })
        .collect();

    entries.sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok())));
    entries.first().map(|e| e.path())
}

/// Load session from path or auto-detect.
pub fn load_session(path: Option<&Path>) -> Result<(SessionFile, PathBuf)> {
    let path = match path {
        Some(p) => p.to_path_buf(),
        None => find_active_session()
            .ok_or_else(|| anyhow::anyhow!("No active session. Run `mscout attach <pid>` first."))?,
    };
    let data = fs::read(&path)
        .with_context(|| format!("Failed to read session file: {}", path.display()))?;
    let session: SessionFile = bincode::deserialize(&data)
        .with_context(|| "Failed to deserialize session file (corrupt?)")?;
    Ok((session, path))
}

/// Save session atomically (write to .tmp then rename).
pub fn save_session(session: &SessionFile, path: Option<&Path>) -> Result<PathBuf> {
    let path = match path {
        Some(p) => p.to_path_buf(),
        None => session_path_for_pid(session.pid),
    };

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = path.with_extension("bin.tmp");
    let data = bincode::serialize(session)?;
    let mut file = fs::File::create(&tmp_path)?;
    file.write_all(&data)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp_path, &path)?;
    Ok(path)
}

/// Remove session file.
pub fn remove_session(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
