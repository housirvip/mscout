pub mod page_table;
pub mod vmware;
pub mod windows_guest;

#[cfg(target_os = "windows")]
pub mod hyperv;

use crate::platform::PlatformError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("page not present at {0:#x}")]
    PageNotPresent(u64),
    #[error("guest RAM not found in VM process")]
    GuestRamNotFound,
    #[error("kernel base not found")]
    KernelNotFound,
    #[error("process not found in guest: {0}")]
    GuestProcessNotFound(String),
    #[error("platform: {0}")]
    Platform(#[from] PlatformError),
    #[error("vm error: {0}")]
    Other(String),
}

/// Trait for reading guest physical memory from the host.
pub trait PhysicalMemoryReader: Send + Sync {
    fn read_phys(&self, gpa: u64, buf: &mut [u8]) -> Result<usize, VmError>;
    fn write_phys(&self, gpa: u64, data: &[u8]) -> Result<usize, VmError>;
    fn guest_ram_size(&self) -> usize;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInfo {
    pub id: String,
    pub name: String,
    pub vm_type: String,
    pub pid: u32,
}
