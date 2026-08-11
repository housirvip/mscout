use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRegion {
    pub base: usize,
    pub size: usize,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    /// e.g. "[heap]", mapped file name, or empty
    pub info: String,
}

pub trait ProcessMemory: Send + Sync {
    /// List all running processes
    fn list_processes() -> Result<Vec<ProcessInfo>, PlatformError>
    where
        Self: Sized;

    /// Attach to a process by PID. Returns a handle.
    fn attach(pid: u32) -> Result<Self, PlatformError>
    where
        Self: Sized;

    /// Read `buf.len()` bytes from `address`. Returns bytes actually read.
    fn read(&self, address: usize, buf: &mut [u8]) -> Result<usize, PlatformError>;

    /// Write `data` to `address`. Returns bytes written.
    fn write(&self, address: usize, data: &[u8]) -> Result<usize, PlatformError>;

    /// Enumerate readable memory regions of the attached process.
    fn regions(&self) -> Result<Vec<MemoryRegion>, PlatformError>;

    /// PID of the attached process
    fn pid(&self) -> u32;
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("permission denied (pid={0})")]
    PermissionDenied(u32),
    #[error("process not found (pid={0})")]
    ProcessNotFound(u32),
    #[error("read failed at {address:#x}: {reason}")]
    ReadFailed { address: usize, reason: String },
    #[error("write failed at {address:#x}: {reason}")]
    WriteFailed { address: usize, reason: String },
    #[error("platform error: {0}")]
    Other(String),
}
