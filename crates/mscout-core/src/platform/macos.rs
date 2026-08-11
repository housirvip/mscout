use super::traits::*;
use mach2::kern_return::KERN_SUCCESS;
use mach2::port::mach_port_t;
use mach2::traps::mach_task_self;
use mach2::vm_types::{mach_vm_address_t, mach_vm_size_t};
use std::mem;
use std::ptr;

// libproc functions for process enumeration (available on macOS without extra deps)
extern "C" {
    fn proc_listallpids(buffer: *mut libc::c_void, buffersize: i32) -> i32;
    fn proc_name(pid: i32, buffer: *mut libc::c_void, buffersize: u32) -> i32;
    fn proc_regionfilename(pid: i32, address: u64, buffer: *mut libc::c_void, buffersize: u32) -> i32;
}

// mach_vm_* functions are not fully wrapped by mach2; declare them via extern.
extern "C" {
    fn mach_vm_read_overwrite(
        target_task: mach_port_t,
        address: mach_vm_address_t,
        size: mach_vm_size_t,
        data: mach_vm_address_t,
        out_size: *mut mach_vm_size_t,
    ) -> i32;

    fn mach_vm_write(
        target_task: mach_port_t,
        address: mach_vm_address_t,
        data: *const u8,
        data_cnt: u32,
    ) -> i32;

    fn mach_vm_region(
        target_task: mach_port_t,
        address: *mut mach_vm_address_t,
        size: *mut mach_vm_size_t,
        flavor: i32,
        info: *mut i32,
        info_cnt: *mut u32,
        object_name: *mut mach_port_t,
    ) -> i32;

    fn task_for_pid(
        target_tport: mach_port_t,
        pid: i32,
        task: *mut mach_port_t,
    ) -> i32;
}

/// VM_REGION_BASIC_INFO_64 flavor constant
const VM_REGION_BASIC_INFO_64: i32 = 9;

/// Size of vm_region_basic_info_64 in natural_t units
const VM_REGION_BASIC_INFO_COUNT_64: u32 = 9;

// VM protection bits
const VM_PROT_READ: i32 = 0x01;
const VM_PROT_WRITE: i32 = 0x02;
const VM_PROT_EXECUTE: i32 = 0x04;

#[repr(C)]
#[derive(Default)]
struct VmRegionBasicInfo64 {
    protection: i32,
    max_protection: i32,
    inheritance: u32,
    shared: u32,
    reserved: u32,
    offset: u64,
    behavior: i32,
    user_wired_count: u16,
}

pub struct MacOsProcess {
    pid: u32,
    task: mach_port_t,
}

// SAFETY: The mach port can be used from any thread.
unsafe impl Send for MacOsProcess {}
unsafe impl Sync for MacOsProcess {}

impl ProcessMemory for MacOsProcess {
    fn list_processes() -> Result<Vec<ProcessInfo>, PlatformError> {
        // Get number of processes
        let count = unsafe { proc_listallpids(ptr::null_mut(), 0) };
        if count <= 0 {
            return Err(PlatformError::Other("proc_listallpids failed".into()));
        }

        // Allocate buffer for PIDs (with some extra space for race conditions)
        let buf_count = (count as usize) * 2;
        let mut pids: Vec<i32> = vec![0i32; buf_count];
        let actual = unsafe {
            proc_listallpids(
                pids.as_mut_ptr() as *mut libc::c_void,
                (buf_count * mem::size_of::<i32>()) as i32,
            )
        };
        if actual <= 0 {
            return Err(PlatformError::Other("proc_listallpids failed".into()));
        }

        let mut result = Vec::with_capacity(actual as usize);
        let mut name_buf = [0u8; 256];

        for &pid in &pids[..actual as usize] {
            if pid <= 0 {
                continue;
            }
            let name_len = unsafe {
                proc_name(
                    pid,
                    name_buf.as_mut_ptr() as *mut libc::c_void,
                    name_buf.len() as u32,
                )
            };
            let name = if name_len > 0 {
                String::from_utf8_lossy(&name_buf[..name_len as usize]).into_owned()
            } else {
                String::new()
            };
            result.push(ProcessInfo {
                pid: pid as u32,
                name,
            });
        }

        Ok(result)
    }

    fn attach(pid: u32) -> Result<Self, PlatformError> {
        let mut task: mach_port_t = 0;
        let kr = unsafe { task_for_pid(mach_task_self(), pid as i32, &mut task) };
        if kr != KERN_SUCCESS {
            if kr == 5 {
                // KERN_FAILURE often means permission denied
                return Err(PlatformError::PermissionDenied(pid));
            }
            return Err(PlatformError::Other(format!(
                "task_for_pid failed with kern_return: {}",
                kr
            )));
        }
        Ok(Self { pid, task })
    }

    fn read(&self, address: usize, buf: &mut [u8]) -> Result<usize, PlatformError> {
        let mut out_size: mach_vm_size_t = 0;
        let kr = unsafe {
            mach_vm_read_overwrite(
                self.task,
                address as mach_vm_address_t,
                buf.len() as mach_vm_size_t,
                buf.as_mut_ptr() as mach_vm_address_t,
                &mut out_size,
            )
        };
        if kr != KERN_SUCCESS {
            return Err(PlatformError::ReadFailed {
                address,
                reason: format!("mach_vm_read_overwrite returned {}", kr),
            });
        }
        Ok(out_size as usize)
    }

    fn write(&self, address: usize, data: &[u8]) -> Result<usize, PlatformError> {
        let kr = unsafe {
            mach_vm_write(
                self.task,
                address as mach_vm_address_t,
                data.as_ptr(),
                data.len() as u32,
            )
        };
        if kr != KERN_SUCCESS {
            return Err(PlatformError::WriteFailed {
                address,
                reason: format!("mach_vm_write returned {}", kr),
            });
        }
        Ok(data.len())
    }

    fn regions(&self) -> Result<Vec<MemoryRegion>, PlatformError> {
        let mut regions = Vec::new();
        let mut address: mach_vm_address_t = 0;

        loop {
            let mut size: mach_vm_size_t = 0;
            let mut info = VmRegionBasicInfo64::default();
            let mut info_count = VM_REGION_BASIC_INFO_COUNT_64;
            let mut object_name: mach_port_t = 0;

            let kr = unsafe {
                mach_vm_region(
                    self.task,
                    &mut address,
                    &mut size,
                    VM_REGION_BASIC_INFO_64,
                    &mut info as *mut VmRegionBasicInfo64 as *mut i32,
                    &mut info_count,
                    &mut object_name,
                )
            };

            if kr != KERN_SUCCESS {
                // KERN_INVALID_ADDRESS (1) means we've enumerated all regions
                break;
            }

            let readable = (info.protection & VM_PROT_READ) != 0;
            let writable = (info.protection & VM_PROT_WRITE) != 0;
            let executable = (info.protection & VM_PROT_EXECUTE) != 0;

            // Resolve mapped file name
            let mut path_buf = [0u8; 1024];
            let path_len = unsafe {
                proc_regionfilename(
                    self.pid as i32,
                    address,
                    path_buf.as_mut_ptr() as *mut libc::c_void,
                    path_buf.len() as u32,
                )
            };
            let region_info = if path_len > 0 {
                let len = (path_len as usize).min(path_buf.len());
                String::from_utf8_lossy(&path_buf[..len]).into_owned()
            } else {
                String::new()
            };

            regions.push(MemoryRegion {
                base: address as usize,
                size: size as usize,
                readable,
                writable,
                executable,
                info: region_info,
            });

            address += size;
        }

        Ok(regions)
    }

    fn pid(&self) -> u32 {
        self.pid
    }
}
