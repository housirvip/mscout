use std::sync::Arc;

use parking_lot::Mutex;

use mem_scanner_core::{
    platform::ProcessMemory,
    vm::{
        vmware::{VmwareConnector, VmwareProcess},
        VmInfo,
    },
};
use serde::Serialize;
use tauri::State;

use crate::AppState;

#[derive(Serialize)]
pub struct GuestProcess {
    pub pid: u32,
    pub name: String,
}

#[tauri::command]
pub fn list_vms() -> Result<Vec<VmInfo>, String> {
    #[allow(unused_mut)]
    let mut vms = VmwareConnector::detect_vms().map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        use mem_scanner_core::vm::hyperv::HyperVConnector;
        if let Ok(mut hyperv_vms) = HyperVConnector::detect_vms() {
            vms.append(&mut hyperv_vms);
        }
    }

    Ok(vms)
}

#[tauri::command]
pub fn attach_vm(pid: u32) -> Result<Vec<GuestProcess>, String> {
    let connector = VmwareConnector::attach(pid).map_err(|e| e.to_string())?;
    let guest_processes =
        VmwareProcess::list_guest_processes(&connector).map_err(|e| e.to_string())?;

    let result: Vec<GuestProcess> = guest_processes
        .iter()
        .map(|p| GuestProcess {
            pid: p.pid,
            name: p.name.clone(),
        })
        .collect();

    Ok(result)
}

#[tauri::command]
pub fn attach_vm_process(
    vm_pid: u32,
    guest_pid: u32,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let connector = VmwareConnector::attach(vm_pid).map_err(|e| e.to_string())?;
    let connector = Arc::new(connector);

    let guest_processes =
        VmwareProcess::list_guest_processes(&connector).map_err(|e| e.to_string())?;

    let target = guest_processes
        .iter()
        .find(|p| p.pid == guest_pid)
        .ok_or_else(|| format!("Guest process {} not found", guest_pid))?;

    let vm_process = VmwareProcess::new(connector, target);

    let mut app_state = state.lock();
    app_state.process = Some(Arc::new(vm_process) as Arc<dyn ProcessMemory + Send + Sync>);
    Ok(())
}
