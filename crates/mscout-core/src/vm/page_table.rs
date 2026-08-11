use std::collections::HashMap;

use parking_lot::Mutex;

use super::{PhysicalMemoryReader, VmError};

/// x86-64 four-level page table walker with TLB cache.
pub struct PageTableWalker {
    tlb: Mutex<HashMap<u64, u64>>,
}

impl PageTableWalker {
    pub fn new() -> Self {
        Self {
            tlb: Mutex::new(HashMap::with_capacity(4096)),
        }
    }

    /// Translate a guest virtual address (GVA) to a guest physical address (GPA).
    pub fn translate(
        &self,
        reader: &dyn PhysicalMemoryReader,
        cr3: u64,
        gva: u64,
    ) -> Result<u64, VmError> {
        let page_key = gva & !0xFFF;
        {
            let cache = self.tlb.lock();
            if let Some(&cached_gpa_page) = cache.get(&page_key) {
                return Ok(cached_gpa_page | (gva & 0xFFF));
            }
        }
        let gpa = self.walk_tables(reader, cr3, gva)?;
        let gpa_page = gpa & !0xFFF;
        self.tlb.lock().insert(page_key, gpa_page);
        Ok(gpa)
    }

    /// Invalidate the entire TLB cache.
    pub fn invalidate(&self) {
        self.tlb.lock().clear();
    }

    fn walk_tables(
        &self,
        reader: &dyn PhysicalMemoryReader,
        cr3: u64,
        gva: u64,
    ) -> Result<u64, VmError> {
        let pml4_base = cr3 & 0x000F_FFFF_FFFF_F000;
        let pml4_idx = ((gva >> 39) & 0x1FF) as u64;
        let pdpt_idx = ((gva >> 30) & 0x1FF) as u64;
        let pd_idx = ((gva >> 21) & 0x1FF) as u64;
        let pt_idx = ((gva >> 12) & 0x1FF) as u64;
        let offset = gva & 0xFFF;

        // PML4
        let pml4e = self.read_entry(reader, pml4_base + pml4_idx * 8)?;
        if pml4e & 1 == 0 {
            return Err(VmError::PageNotPresent(gva));
        }

        // PDPT
        let pdpt_base = pml4e & 0x000F_FFFF_FFFF_F000;
        let pdpte = self.read_entry(reader, pdpt_base + pdpt_idx * 8)?;
        if pdpte & 1 == 0 {
            return Err(VmError::PageNotPresent(gva));
        }
        // 1GB huge page (PS bit set)
        if pdpte & 0x80 != 0 {
            return Ok((pdpte & 0x000F_FFFF_C000_0000) | (gva & 0x3FFF_FFFF));
        }

        // PD
        let pd_base = pdpte & 0x000F_FFFF_FFFF_F000;
        let pde = self.read_entry(reader, pd_base + pd_idx * 8)?;
        if pde & 1 == 0 {
            return Err(VmError::PageNotPresent(gva));
        }
        // 2MB large page (PS bit set)
        if pde & 0x80 != 0 {
            return Ok((pde & 0x000F_FFFF_FFE0_0000) | (gva & 0x1F_FFFF));
        }

        // PT
        let pt_base = pde & 0x000F_FFFF_FFFF_F000;
        let pte = self.read_entry(reader, pt_base + pt_idx * 8)?;
        if pte & 1 == 0 {
            return Err(VmError::PageNotPresent(gva));
        }

        Ok((pte & 0x000F_FFFF_FFFF_F000) | offset)
    }

    fn read_entry(&self, reader: &dyn PhysicalMemoryReader, addr: u64) -> Result<u64, VmError> {
        let mut buf = [0u8; 8];
        reader.read_phys(addr, &mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }
}

impl Default for PageTableWalker {
    fn default() -> Self {
        Self::new()
    }
}
