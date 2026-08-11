use serde::{Deserialize, Serialize};

use super::page_table::PageTableWalker;
use super::{PhysicalMemoryReader, VmError};

/// EPROCESS structure offsets for Windows 10 21H2+ / Windows 11.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EprocessOffsets {
    pub active_process_links: usize,
    pub unique_process_id: usize,
    pub image_file_name: usize,
    pub directory_table_base: usize,
}

impl Default for EprocessOffsets {
    fn default() -> Self {
        Self {
            active_process_links: 0x448,
            unique_process_id: 0x440,
            image_file_name: 0x5A8,
            directory_table_base: 0x028,
        }
    }
}

/// Information about a process inside the guest OS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cr3: u64,
    pub eprocess_va: u64,
}

/// Windows guest OS introspection engine.
pub struct WindowsGuest {
    pub kernel_base_pa: u64,
    pub kernel_base_va: u64,
    pub kernel_cr3: u64,
    offsets: EprocessOffsets,
}

impl WindowsGuest {
    /// Scan guest physical memory to find the Windows kernel base (ntoskrnl).
    /// This scans for MZ/PE signatures with kernel-specific section names.
    pub fn find_kernel(reader: &dyn PhysicalMemoryReader) -> Result<Self, VmError> {
        let ram_size = reader.guest_ram_size() as u64;
        let mut buf = [0u8; 0x1000];

        // Scan from 1MB upward (kernel is loaded above 1MB)
        let mut gpa = 0x100000u64;
        while gpa < ram_size {
            if reader.read_phys(gpa, &mut buf[..2]).is_err() {
                gpa += 0x1000;
                continue;
            }
            // Check for MZ header
            if buf[0] != b'M' || buf[1] != b'Z' {
                gpa += 0x1000;
                continue;
            }
            // Read full first page
            if reader.read_phys(gpa, &mut buf).is_err() {
                gpa += 0x1000;
                continue;
            }
            // Validate PE offset
            let pe_offset =
                u32::from_le_bytes([buf[0x3C], buf[0x3D], buf[0x3E], buf[0x3F]]) as usize;
            if pe_offset >= 0xF00 || pe_offset + 4 > buf.len() {
                gpa += 0x1000;
                continue;
            }
            // Check PE signature
            if &buf[pe_offset..pe_offset + 4] != b"PE\x00\x00" {
                gpa += 0x1000;
                continue;
            }
            // Look for kernel-specific section names in the page
            let page_str = String::from_utf8_lossy(&buf[pe_offset..]);
            if page_str.contains("PAGELK") || page_str.contains(".text") {
                // Extract ImageBase from PE optional header (PE64 format)
                // Optional header starts at pe_offset + 0x18
                // ImageBase is at optional header + 0x18 (offset 0x30 from PE sig)
                let image_base_offset = pe_offset + 0x30;
                if image_base_offset + 8 > buf.len() {
                    gpa += 0x1000;
                    continue;
                }
                let kernel_va = u64::from_le_bytes(
                    buf[image_base_offset..image_base_offset + 8]
                        .try_into()
                        .unwrap(),
                );

                let kernel_cr3 = Self::find_kernel_cr3(reader)?;
                return Ok(Self {
                    kernel_base_pa: gpa,
                    kernel_base_va: kernel_va,
                    kernel_cr3,
                    offsets: EprocessOffsets::default(),
                });
            }
            gpa += 0x1000;
        }
        Err(VmError::KernelNotFound)
    }

    /// Heuristic: scan low memory for a valid PML4 that looks like a kernel page table.
    /// A kernel PML4 typically has many entries in the upper half (indices 256-511).
    fn find_kernel_cr3(reader: &dyn PhysicalMemoryReader) -> Result<u64, VmError> {
        let ram_size = reader.guest_ram_size() as u64;
        let search_limit = ram_size.min(0x1000000); // First 16MB
        let mut buf = [0u8; 8];

        let mut candidate_pa = 0x1000u64;
        while candidate_pa < search_limit {
            // Read the first PML4 entry to quickly reject
            if reader.read_phys(candidate_pa, &mut buf).is_err() {
                candidate_pa += 0x1000;
                continue;
            }
            let entry = u64::from_le_bytes(buf);
            // A valid PML4E should have present bit set and point within RAM
            if entry & 1 == 0 {
                candidate_pa += 0x1000;
                continue;
            }
            let pointed = entry & 0x000F_FFFF_FFFF_F000;
            if pointed == 0 || pointed >= ram_size {
                candidate_pa += 0x1000;
                continue;
            }

            // Count populated entries in the upper half (kernel space)
            let mut high_count = 0u32;
            for i in 256u64..512 {
                if reader.read_phys(candidate_pa + i * 8, &mut buf).is_err() {
                    break;
                }
                let e = u64::from_le_bytes(buf);
                if e & 1 != 0 {
                    high_count += 1;
                }
            }
            // Kernel PML4 typically has >10 populated high-half entries
            if high_count > 10 {
                return Ok(candidate_pa);
            }
            candidate_pa += 0x1000;
        }
        Err(VmError::Other("Could not find kernel CR3".into()))
    }

    /// Enumerate guest processes by scanning physical memory for EPROCESS structures.
    /// Looks for the characteristic pattern of DirectoryTableBase + UniqueProcessId + ImageFileName.
    pub fn enumerate_processes(
        &self,
        reader: &dyn PhysicalMemoryReader,
        _walker: &PageTableWalker,
    ) -> Result<Vec<GuestProcessInfo>, VmError> {
        let ram_size = reader.guest_ram_size() as u64;
        let mut processes = Vec::new();
        let mut buf = [0u8; 0x600];

        let mut pa = 0x1000u64;
        while pa < ram_size {
            if reader.read_phys(pa, &mut buf).is_err() {
                pa += 0x1000;
                continue;
            }

            // Check DirectoryTableBase (must be page-aligned, non-zero, within RAM)
            let dtb_offset = self.offsets.directory_table_base;
            if dtb_offset + 8 > buf.len() {
                pa += 0x1000;
                continue;
            }
            let dtb = u64::from_le_bytes(buf[dtb_offset..dtb_offset + 8].try_into().unwrap());
            if dtb == 0 || dtb & 0xFFF != 0 || dtb >= ram_size {
                pa += 0x1000;
                continue;
            }

            // Check UniqueProcessId (reasonable PID range)
            let pid_offset = self.offsets.unique_process_id;
            if pid_offset + 8 > buf.len() {
                pa += 0x1000;
                continue;
            }
            let pid = u64::from_le_bytes(buf[pid_offset..pid_offset + 8].try_into().unwrap());
            if pid == 0 || pid > 0xFFFF {
                pa += 0x1000;
                continue;
            }

            // Check ImageFileName (up to 15 bytes of printable ASCII)
            let name_offset = self.offsets.image_file_name;
            if name_offset + 15 > buf.len() {
                pa += 0x1000;
                continue;
            }
            let name_bytes = &buf[name_offset..name_offset + 15];
            let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(15);
            if name_end == 0 {
                pa += 0x1000;
                continue;
            }
            let name_slice = &name_bytes[..name_end];
            if !name_slice.iter().all(|&b| b >= 0x20 && b <= 0x7E) {
                pa += 0x1000;
                continue;
            }
            let name = String::from_utf8_lossy(name_slice).into_owned();

            // Deduplicate by PID + name
            if !processes
                .iter()
                .any(|p: &GuestProcessInfo| p.pid == pid as u32 && p.name == name)
            {
                processes.push(GuestProcessInfo {
                    pid: pid as u32,
                    name,
                    cr3: dtb,
                    eprocess_va: 0, // Not easily determined without walking VA
                });
            }

            pa += 0x1000;
        }

        Ok(processes)
    }
}
