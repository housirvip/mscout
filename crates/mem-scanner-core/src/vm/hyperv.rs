// Hyper-V connector — only available on Windows hosts.
// Targets vmwp.exe (Virtual Machine Worker Process).

#[cfg(target_os = "windows")]
use super::{PhysicalMemoryReader, VmError, VmInfo};
#[cfg(target_os = "windows")]
use crate::platform::{NativeProcess, ProcessMemory};

#[cfg(target_os = "windows")]
pub struct HyperVConnector {
    host_process: NativeProcess,
    guest_ram_base: usize,
    guest_ram_size: usize,
}

#[cfg(target_os = "windows")]
impl HyperVConnector {
    /// Detect running Hyper-V VMs by finding vmwp.exe processes.
    pub fn detect_vms() -> Result<Vec<VmInfo>, VmError> {
        let processes = NativeProcess::list_processes().map_err(VmError::Platform)?;
        let vms: Vec<VmInfo> = processes
            .iter()
            .filter(|p| {
                let name_lower = p.name.to_lowercase();
                name_lower == "vmwp.exe" || name_lower == "vmwp"
            })
            .map(|p| VmInfo {
                id: format!("hyperv-{}", p.pid),
                name: format!("Hyper-V VM (PID {})", p.pid),
                vm_type: "hyperv".to_string(),
                pid: p.pid,
            })
            .collect();
        Ok(vms)
    }

    /// Attach to a Hyper-V VM worker process and locate guest RAM.
    pub fn attach(pid: u32) -> Result<Self, VmError> {
        let host_process = NativeProcess::attach(pid).map_err(VmError::Platform)?;
        let regions = host_process.regions().map_err(VmError::Platform)?;

        let guest_region = regions
            .iter()
            .filter(|r| r.readable && r.writable && !r.executable && r.info.is_empty())
            .max_by_key(|r| r.size)
            .ok_or(VmError::GuestRamNotFound)?;

        if guest_region.size < 256 * 1024 * 1024 {
            return Err(VmError::GuestRamNotFound);
        }

        Ok(Self {
            host_process,
            guest_ram_base: guest_region.base,
            guest_ram_size: guest_region.size,
        })
    }
}

#[cfg(target_os = "windows")]
impl PhysicalMemoryReader for HyperVConnector {
    fn read_phys(&self, gpa: u64, buf: &mut [u8]) -> Result<usize, VmError> {
        if gpa as usize + buf.len() > self.guest_ram_size {
            return Err(VmError::Other("GPA out of range".into()));
        }
        let host_addr = self.guest_ram_base + gpa as usize;
        self.host_process
            .read(host_addr, buf)
            .map_err(VmError::Platform)
    }

    fn write_phys(&self, gpa: u64, data: &[u8]) -> Result<usize, VmError> {
        if gpa as usize + data.len() > self.guest_ram_size {
            return Err(VmError::Other("GPA out of range".into()));
        }
        let host_addr = self.guest_ram_base + gpa as usize;
        self.host_process
            .write(host_addr, data)
            .map_err(VmError::Platform)
    }

    fn guest_ram_size(&self) -> usize {
        self.guest_ram_size
    }
}
