use super::traits::*;
use std::fs::{self, File, OpenOptions};
use std::os::unix::io::AsRawFd;

pub struct LinuxProcess {
    pid: u32,
    mem_file: File,
}

impl ProcessMemory for LinuxProcess {
    fn list_processes() -> Result<Vec<ProcessInfo>, PlatformError> {
        let mut processes = Vec::new();

        let entries = fs::read_dir("/proc").map_err(|e| PlatformError::Other(e.to_string()))?;

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Only numeric directory names are PIDs
            if !name_str.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            let pid: u32 = match name_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Read the process command name
            let comm_path = format!("/proc/{}/comm", pid);
            let comm = match fs::read_to_string(&comm_path) {
                Ok(c) => c.trim_end_matches('\n').to_string(),
                Err(_) => continue, // Skip if unreadable (kernel threads, permission denied)
            };

            processes.push(ProcessInfo { pid, name: comm });
        }

        Ok(processes)
    }

    fn attach(pid: u32) -> Result<Self, PlatformError> {
        let mem_path = format!("/proc/{}/mem", pid);

        let mem_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&mem_path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => PlatformError::ProcessNotFound(pid),
                std::io::ErrorKind::PermissionDenied => PlatformError::PermissionDenied(pid),
                _ => PlatformError::Other(format!("failed to open {}: {}", mem_path, e)),
            })?;

        Ok(LinuxProcess { pid, mem_file })
    }

    fn read(&self, address: usize, buf: &mut [u8]) -> Result<usize, PlatformError> {
        let fd = self.mem_file.as_raw_fd();
        let result = unsafe {
            libc::pread64(
                fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
                address as i64,
            )
        };

        if result < 0 {
            let err = std::io::Error::last_os_error();
            return Err(PlatformError::ReadFailed {
                address,
                reason: err.to_string(),
            });
        }

        Ok(result as usize)
    }

    fn write(&self, address: usize, data: &[u8]) -> Result<usize, PlatformError> {
        let fd = self.mem_file.as_raw_fd();
        let result = unsafe {
            libc::pwrite64(
                fd,
                data.as_ptr() as *const libc::c_void,
                data.len(),
                address as i64,
            )
        };

        if result < 0 {
            let err = std::io::Error::last_os_error();
            return Err(PlatformError::WriteFailed {
                address,
                reason: err.to_string(),
            });
        }

        Ok(result as usize)
    }

    fn regions(&self) -> Result<Vec<MemoryRegion>, PlatformError> {
        let maps_path = format!("/proc/{}/maps", self.pid);
        let content = fs::read_to_string(&maps_path).map_err(|e| {
            PlatformError::Other(format!("failed to read {}: {}", maps_path, e))
        })?;

        let mut regions = Vec::new();

        for line in content.lines() {
            if line.is_empty() {
                continue;
            }

            // Format: start-end perms offset dev inode pathname
            // Example: 7f8c0a000000-7f8c0a021000 rw-p 00000000 00:00 0  [heap]
            let mut parts = line.splitn(6, char::is_whitespace);

            let addr_range = match parts.next() {
                Some(a) => a,
                None => continue,
            };

            let perms = match parts.next() {
                Some(p) => p,
                None => continue,
            };

            // Skip offset, dev, inode (parts 3, 4, 5)
            let _offset = parts.next();
            let _dev = parts.next();
            let _inode = parts.next();

            // Pathname is the remainder (may be empty or have leading whitespace)
            let pathname = parts.next().unwrap_or("").trim().to_string();

            // Parse address range "start-end"
            let mut addr_parts = addr_range.split('-');
            let start_str = match addr_parts.next() {
                Some(s) => s,
                None => continue,
            };
            let end_str = match addr_parts.next() {
                Some(s) => s,
                None => continue,
            };

            let start = match usize::from_str_radix(start_str, 16) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let end = match usize::from_str_radix(end_str, 16) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let readable = perms.contains('r');
            let writable = perms.contains('w');
            let executable = perms.contains('x');

            regions.push(MemoryRegion {
                base: start,
                size: end - start,
                readable,
                writable,
                executable,
                info: pathname,
            });
        }

        Ok(regions)
    }

    fn pid(&self) -> u32 {
        self.pid
    }
}
