use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use mscout_core::platform::{NativeProcess, ProcessMemory};
use serde_json::json;

use crate::output::Output;
use crate::session::{self, SessionFile};
use crate::state::CliState;

/// Attach to a process (subcommand mode: creates session file).
pub fn attach(pid: u32, session_path: Option<&Path>, out: &Output) -> Result<()> {
    // Verify process exists by attaching
    let process = NativeProcess::attach(pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let name = {
        let processes = NativeProcess::list_processes().unwrap_or_default();
        processes
            .iter()
            .find(|p| p.pid == pid)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("pid:{pid}"))
    };
    drop(process);

    let session_file = SessionFile {
        version: 1,
        pid,
        process_name: name.clone(),
        vm_pid: None,
        guest_pid: None,
        scan_state: None,
        frozen_pids: Vec::new(),
        table: None,
    };
    let saved_path = session::save_session(&session_file, session_path)?;

    if out.json {
        out.success("attach", json!({"pid": pid, "name": name, "session": saved_path.display().to_string()}));
    } else {
        println!("Attached to {} (PID {})", name, pid);
        println!("Session: {}", saved_path.display());
    }
    Ok(())
}

/// Detach from current process (removes session, stops freeze daemons).
pub fn detach(session_path: Option<&Path>, out: &Output) -> Result<()> {
    let (session, path) = session::load_session(session_path)?;

    // Kill freeze daemons (best-effort cleanup)
    for daemon_pid in &session.frozen_pids {
        #[cfg(unix)]
        {
            match i32::try_from(*daemon_pid) {
                Ok(pid) => {
                    // SAFETY: sending SIGTERM to a PID we own from a prior freeze command.
                    let ret = unsafe { libc::kill(pid, libc::SIGTERM) };
                    if ret != 0 {
                        // Process may have already exited — ignore.
                    }
                }
                Err(_) => {} // PID exceeds i32::MAX — skip
            }
        }
    }

    session::remove_session(&path)?;

    if out.json {
        out.success("detach", json!({"pid": session.pid}));
    } else {
        println!("Detached from {} (PID {})", session.process_name, session.pid);
    }
    Ok(())
}

/// Attach in REPL mode (stores state in-memory).
pub fn attach_repl(state: &mut CliState, pid: u32, out: &Output) -> Result<()> {
    let process = NativeProcess::attach(pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let name = {
        let processes = NativeProcess::list_processes().unwrap_or_default();
        processes
            .iter()
            .find(|p| p.pid == pid)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("pid:{pid}"))
    };

    state.process = Some(Arc::new(process));
    state.pid = Some(pid);
    state.process_name = Some(name.clone());
    state.scan_session = None;
    state.freeze_manager = None;

    if out.json {
        out.success("attach", json!({"pid": pid, "name": name}));
    } else {
        println!("Attached to {} (PID {})", name, pid);
    }
    Ok(())
}

/// Detach in REPL mode.
pub fn detach_repl(state: &mut CliState, out: &Output) -> Result<()> {
    let pid = state.pid.ok_or_else(|| anyhow::anyhow!("Not attached to any process"))?;
    let name = state.process_name.clone().unwrap_or_default();

    if let Some(mut fm) = state.freeze_manager.take() {
        fm.stop();
    }
    state.process = None;
    state.scan_session = None;
    state.pid = None;
    state.process_name = None;

    if out.json {
        out.success("detach", json!({"pid": pid}));
    } else {
        println!("Detached from {} (PID {})", name, pid);
    }
    Ok(())
}
