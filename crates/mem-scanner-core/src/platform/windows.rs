use super::traits::*;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Memory::{
    VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE, PAGE_EXECUTE_READ,
    PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ,
    PROCESS_VM_WRITE,
};

pub struct WindowsProcess {
    pid: u32,
    handle: HANDLE,
}

// SAFETY: The HANDLE is only used for memory read/write operations which are
// inherently thread-safe when targeting different addresses, and the Windows API
// functions used are documented as thread-safe.
unsafe impl Send for WindowsProcess {}
unsafe impl Sync for WindowsProcess {}

impl Drop for WindowsProcess {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

impl ProcessMemory for WindowsProcess {
    fn list_processes() -> Result<Vec<ProcessInfo>, PlatformError> {
        let mut processes = Vec::new();

        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .map_err(|e| PlatformError::Other(format!("CreateToolhelp32Snapshot failed: {}", e)))?;

            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    // Find null terminator in szExeFile
                    let name_len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);

                    processes.push(ProcessInfo {
                        pid: entry.th32ProcessID,
                        name,
                    });

                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
        }

        Ok(processes)
    }

    fn attach(pid: u32) -> Result<Self, PlatformError> {
        let access = PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_INFORMATION;

        let handle = unsafe { OpenProcess(access, false, pid) }.map_err(|e| {
            let code = e.code().0 as u32;
            // ERROR_ACCESS_DENIED = 5, ERROR_INVALID_PARAMETER = 87
            if code == 5 {
                PlatformError::PermissionDenied(pid)
            } else if code == 87 {
                PlatformError::ProcessNotFound(pid)
            } else {
                PlatformError::Other(format!("OpenProcess failed: {}", e))
            }
        })?;

        Ok(WindowsProcess { pid, handle })
    }

    fn read(&self, address: usize, buf: &mut [u8]) -> Result<usize, PlatformError> {
        let mut bytes_read: usize = 0;

        unsafe {
            ReadProcessMemory(
                self.handle,
                address as *const std::ffi::c_void,
                buf.as_mut_ptr() as *mut std::ffi::c_void,
                buf.len(),
                Some(&mut bytes_read),
            )
            .map_err(|e| PlatformError::ReadFailed {
                address,
                reason: e.to_string(),
            })?;
        }

        Ok(bytes_read)
    }

    fn write(&self, address: usize, data: &[u8]) -> Result<usize, PlatformError> {
        let mut bytes_written: usize = 0;

        unsafe {
            WriteProcessMemory(
                self.handle,
                address as *const std::ffi::c_void,
                data.as_ptr() as *const std::ffi::c_void,
                data.len(),
                Some(&mut bytes_written),
            )
            .map_err(|e| PlatformError::WriteFailed {
                address,
                reason: e.to_string(),
            })?;
        }

        Ok(bytes_written)
    }

    fn regions(&self) -> Result<Vec<MemoryRegion>, PlatformError> {
        let mut regions = Vec::new();
        let mut address: usize = 0;
        let mbi_size = std::mem::size_of::<MEMORY_BASIC_INFORMATION>();

        loop {
            let mut mbi = MEMORY_BASIC_INFORMATION::default();

            let result = unsafe {
                VirtualQueryEx(
                    self.handle,
                    Some(address as *const std::ffi::c_void),
                    &mut mbi,
                    mbi_size,
                )
            };

            if result == 0 {
                break;
            }

            if mbi.State == MEM_COMMIT {
                let protect = mbi.Protect;
                let readable = protect == PAGE_READONLY
                    || protect == PAGE_READWRITE
                    || protect == PAGE_EXECUTE_READ
                    || protect == PAGE_EXECUTE_READWRITE
                    || protect == PAGE_WRITECOPY
                    || protect == PAGE_EXECUTE_WRITECOPY;

                let writable = protect == PAGE_READWRITE
                    || protect == PAGE_EXECUTE_READWRITE
                    || protect == PAGE_WRITECOPY
                    || protect == PAGE_EXECUTE_WRITECOPY;

                let executable = protect == PAGE_EXECUTE
                    || protect == PAGE_EXECUTE_READ
                    || protect == PAGE_EXECUTE_READWRITE
                    || protect == PAGE_EXECUTE_WRITECOPY;

                regions.push(MemoryRegion {
                    base: mbi.BaseAddress as usize,
                    size: mbi.RegionSize,
                    readable,
                    writable,
                    executable,
                    info: String::new(),
                });
            }

            // Advance past this region
            address = mbi.BaseAddress as usize + mbi.RegionSize;

            // Guard against overflow on 32-bit or wrapping
            if address <= mbi.BaseAddress as usize {
                break;
            }
        }

        Ok(regions)
    }

    fn pid(&self) -> u32 {
        self.pid
    }
}
