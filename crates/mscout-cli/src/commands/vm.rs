use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use mscout_core::vm::vmware::{VmwareConnector, VmwareProcess};
use serde_json::json;

use crate::output::Output;
use crate::session::{self, SessionFile};
use crate::state::CliState;

pub fn list_vms(out: &Output) -> Result<()> {
    let vms = VmwareConnector::detect_vms()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    #[cfg(target_os = "windows")]
    {
        use mscout_core::vm::hyperv::HyperVConnector;
        if let Ok(mut hyperv_vms) = HyperVConnector::detect_vms() {
            vms.append(&mut hyperv_vms);
        }
    }

    if out.json {
        let data: Vec<_> = vms.iter().map(|v| {
            json!({"id": v.id, "name": v.name, "type": v.vm_type, "pid": v.pid})
        }).collect();
        out.success("vm-list", data);
    } else {
        if vms.is_empty() {
            println!("No virtual machines detected.");
        } else {
            let rows: Vec<Vec<String>> = vms.iter().map(|v| {
                vec![v.id.clone(), v.name.clone(), v.vm_type.clone(), v.pid.to_string()]
            }).collect();
            out.print_table(&["ID", "Name", "Type", "PID"], rows);
        }
    }
    Ok(())
}

pub fn attach_vm(vm_pid: u32, out: &Output) -> Result<()> {
    let connector = VmwareConnector::attach(vm_pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let guest_processes = VmwareProcess::list_guest_processes(&connector)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if out.json {
        let data: Vec<_> = guest_processes.iter().map(|p| {
            json!({"pid": p.pid, "name": p.name})
        }).collect();
        out.success("vm-attach", json!({"vm_pid": vm_pid, "guests": data}));
    } else {
        println!("VM (PID {}) guest processes:", vm_pid);
        let rows: Vec<Vec<String>> = guest_processes.iter().map(|p| {
            vec![p.pid.to_string(), p.name.clone()]
        }).collect();
        out.print_table(&["Guest PID", "Name"], rows);
    }
    Ok(())
}

pub fn attach_vm_process(
    vm_pid: u32,
    guest_pid: u32,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let connector = VmwareConnector::attach(vm_pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let connector = Arc::new(connector);

    let guest_processes = VmwareProcess::list_guest_processes(&connector)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let target = guest_processes.iter()
        .find(|p| p.pid == guest_pid)
        .ok_or_else(|| anyhow::anyhow!("Guest process {} not found", guest_pid))?;

    let _vm_process = VmwareProcess::new(connector, target);
    let name = target.name.clone();

    // Save session
    let session_file = SessionFile {
        version: 1,
        pid: guest_pid,
        process_name: name.clone(),
        vm_pid: Some(vm_pid),
        guest_pid: Some(guest_pid),
        scan_state: None,
        frozen_pids: Vec::new(),
        table: None,
    };
    session::save_session(&session_file, session_path)?;

    if out.json {
        out.success("vm-attach-process", json!({"vm_pid": vm_pid, "guest_pid": guest_pid, "name": name}));
    } else {
        println!("Attached to VM guest process: {} (PID {})", name, guest_pid);
    }
    Ok(())
}

// ─── REPL ──────────────────────────────────────────────────────────────────

pub fn attach_vm_process_repl(
    state: &mut CliState,
    vm_pid: u32,
    guest_pid: u32,
    out: &Output,
) -> Result<()> {
    let connector = VmwareConnector::attach(vm_pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let connector = Arc::new(connector);

    let guest_processes = VmwareProcess::list_guest_processes(&connector)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let target = guest_processes.iter()
        .find(|p| p.pid == guest_pid)
        .ok_or_else(|| anyhow::anyhow!("Guest process {} not found", guest_pid))?;

    let name = target.name.clone();
    let vm_process = VmwareProcess::new(connector, target);

    state.process = Some(Arc::new(vm_process));
    state.pid = Some(guest_pid);
    state.process_name = Some(name.clone());
    state.scan_session = None;
    state.freeze_manager = None;

    if out.json {
        out.success("vm-attach-process", json!({"vm_pid": vm_pid, "guest_pid": guest_pid, "name": name}));
    } else {
        println!("Attached to VM guest: {} (PID {})", name, guest_pid);
    }
    Ok(())
}
