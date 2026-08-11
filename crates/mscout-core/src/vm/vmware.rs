use std::sync::Arc;

use super::page_table::PageTableWalker;
use super::windows_guest::{GuestProcessInfo, WindowsGuest};
use super::{PhysicalMemoryReader, VmError, VmInfo};
use crate::platform::{MemoryRegion, NativeProcess, PlatformError, ProcessInfo, ProcessMemory};

/// Connector that reads guest physical memory from a VMware vmware-vmx host process.
pub struct VmwareConnector {
    host_process: NativeProcess,
    guest_ram_base: usize,
    guest_ram_size: usize,
}

impl VmwareConnector {
    /// Detect running VMware VMs by finding vmware-vmx processes.
    pub fn detect_vms() -> Result<Vec<VmInfo>, VmError> {
        let processes = NativeProcess::list_processes().map_err(VmError::Platform)?;
        let vms: Vec<VmInfo> = processes
            .iter()
            .filter(|p| {
                let name_lower = p.name.to_lowercase();
                name_lower.contains("vmware-vmx")
            })
            .map(|p| VmInfo {
                id: format!("vmware-{}", p.pid),
                name: format!("VMware VM (PID {})", p.pid),
                vm_type: "vmware".to_string(),
                pid: p.pid,
            })
            .collect();
        Ok(vms)
    }

    /// Attach to a VMware VM process and locate guest RAM.
    /// Guest RAM is identified as the largest anonymous readable+writable region (>= 256MB).
    pub fn attach(pid: u32) -> Result<Self, VmError> {
        let host_process = NativeProcess::attach(pid).map_err(VmError::Platform)?;
        let regions = host_process.regions().map_err(VmError::Platform)?;

        // Find guest RAM: largest anonymous RW region
        let guest_region = regions
            .iter()
            .filter(|r| r.readable && r.writable && !r.executable)
            .filter(|r| r.info.is_empty() || r.info == "[anon]")
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

impl PhysicalMemoryReader for VmwareConnector {
    fn read_phys(&self, gpa: u64, buf: &mut [u8]) -> Result<usize, VmError> {
        if gpa as usize + buf.len() > self.guest_ram_size {
            return Err(VmError::Other(format!(
                "GPA {:#x} out of range (ram_size={:#x})",
                gpa, self.guest_ram_size
            )));
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

/// A guest process accessed through VMware's host process memory.
/// Implements `ProcessMemory` so the standard scan/freeze/read/write commands work transparently.
pub struct VmwareProcess {
    connector: Arc<VmwareConnector>,
    walker: PageTableWalker,
    target_cr3: u64,
    target_pid: u32,
    #[allow(dead_code)]
    target_name: String,
}

impl VmwareProcess {
    pub fn new(connector: Arc<VmwareConnector>, info: &GuestProcessInfo) -> Self {
        Self {
            connector,
            walker: PageTableWalker::new(),
            target_cr3: info.cr3,
            target_pid: info.pid,
            target_name: info.name.clone(),
        }
    }

    /// Enumerate guest processes. Finds the kernel then walks EPROCESS list.
    pub fn list_guest_processes(
        connector: &VmwareConnector,
    ) -> Result<Vec<GuestProcessInfo>, VmError> {
        let guest = WindowsGuest::find_kernel(connector)?;
        let walker = PageTableWalker::new();
        guest.enumerate_processes(connector, &walker)
    }
}

impl ProcessMemory for VmwareProcess {
    fn list_processes() -> Result<Vec<ProcessInfo>, PlatformError> {
        Err(PlatformError::Other(
            "Use VmwareProcess::list_guest_processes instead".into(),
        ))
    }

    fn attach(_pid: u32) -> Result<Self, PlatformError> {
        Err(PlatformError::Other(
            "Use VmwareProcess::new instead".into(),
        ))
    }

    fn read(&self, address: usize, buf: &mut [u8]) -> Result<usize, PlatformError> {
        let mut total_read = 0;
        let mut remaining = buf.len();
        let mut current_addr = address as u64;

        while remaining > 0 {
            let page_offset = (current_addr & 0xFFF) as usize;
            let chunk_size = remaining.min(0x1000 - page_offset);

            let gpa = self
                .walker
                .translate(self.connector.as_ref(), self.target_cr3, current_addr)
                .map_err(|e| PlatformError::ReadFailed {
                    address: current_addr as usize,
                    reason: e.to_string(),
                })?;

            let read = self
                .connector
                .read_phys(gpa, &mut buf[total_read..total_read + chunk_size])
                .map_err(|e| PlatformError::ReadFailed {
                    address: current_addr as usize,
                    reason: e.to_string(),
                })?;

            total_read += read;
            remaining -= chunk_size;
            current_addr += chunk_size as u64;

            if read < chunk_size {
                break;
            }
        }
        Ok(total_read)
    }

    fn write(&self, address: usize, data: &[u8]) -> Result<usize, PlatformError> {
        let mut total_written = 0;
        let mut remaining = data.len();
        let mut current_addr = address as u64;

        while remaining > 0 {
            let page_offset = (current_addr & 0xFFF) as usize;
            let chunk_size = remaining.min(0x1000 - page_offset);

            let gpa = self
                .walker
                .translate(self.connector.as_ref(), self.target_cr3, current_addr)
                .map_err(|e| PlatformError::WriteFailed {
                    address: current_addr as usize,
                    reason: e.to_string(),
                })?;

            let written = self
                .connector
                .write_phys(gpa, &data[total_written..total_written + chunk_size])
                .map_err(|e| PlatformError::WriteFailed {
                    address: current_addr as usize,
                    reason: e.to_string(),
                })?;

            total_written += written;
            remaining -= chunk_size;
            current_addr += chunk_size as u64;

            if written < chunk_size {
                break;
            }
        }
        Ok(total_written)
    }

    fn regions(&self) -> Result<Vec<MemoryRegion>, PlatformError> {
        // Return a simplified region covering typical Windows userspace.
        // A full implementation would walk the guest page tables to find all present pages.
        Ok(vec![MemoryRegion {
            base: 0x10000,
            size: 0x7FFE_0000,
            readable: true,
            writable: true,
            executable: false,
            info: String::new(),
        }])
    }

    fn pid(&self) -> u32 {
        self.target_pid
    }
}
