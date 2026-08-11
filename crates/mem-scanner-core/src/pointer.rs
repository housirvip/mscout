use std::collections::HashMap;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::platform::{MemoryRegion, ProcessMemory};
use crate::scanner::ScanError;

/// Maximum number of results to return from a pointer scan.
const MAX_RESULTS: usize = 10_000;

/// Block size for reading memory regions during pointer map construction (1 MB).
const READ_BLOCK_SIZE: usize = 1024 * 1024;

/// Maximum total readable memory before aborting (4 GB).
const MAX_READABLE_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointerChain {
    pub base_address: usize,
    pub offsets: Vec<isize>,
    pub module: Option<String>,
}

pub struct PointerScanner {
    pub max_depth: usize,
    pub max_offset: usize,
}

impl Default for PointerScanner {
    fn default() -> Self {
        Self {
            max_depth: 5,
            max_offset: 4096,
        }
    }
}

impl PointerScanner {
    pub fn new(max_depth: usize, max_offset: usize) -> Self {
        Self {
            max_depth,
            max_offset,
        }
    }

    /// Perform a pointer scan starting from `target_address`.
    ///
    /// Builds a reverse pointer map from all readable regions, then performs a BFS
    /// from the target to find pointer chains whose base is in a static/module region.
    pub fn scan(
        &self,
        process: &dyn ProcessMemory,
        target_address: usize,
        regions: &[MemoryRegion],
    ) -> Result<Vec<PointerChain>, ScanError> {
        let readable_regions: Vec<&MemoryRegion> =
            regions.iter().filter(|r| r.readable).collect();

        // Check total readable memory doesn't exceed our cap
        let total_bytes: u64 = readable_regions.iter().map(|r| r.size as u64).sum();
        if total_bytes > MAX_READABLE_BYTES {
            return Err(ScanError::InvalidParams(
                "Too much memory to scan (>4GB). Enable region filtering.".into(),
            ));
        }

        // Build the pointer map: page_number -> Vec<(address_that_holds_pointer, pointer_value)>
        let pointer_map = self.build_pointer_map(process, &readable_regions, regions)?;

        // BFS from target
        let results = self.bfs_scan(&pointer_map, target_address, regions);

        Ok(results)
    }

    /// Build a map from page numbers to (address, value) pairs for all pointers found in memory.
    fn build_pointer_map(
        &self,
        process: &dyn ProcessMemory,
        readable_regions: &[&MemoryRegion],
        all_regions: &[MemoryRegion],
    ) -> Result<HashMap<usize, Vec<(usize, usize)>>, ScanError> {
        let ptr_size = std::mem::size_of::<usize>();

        // Precompute region bounds for fast "is this a valid pointer?" checking
        let region_bounds: Vec<(usize, usize)> = all_regions
            .iter()
            .filter(|r| r.readable)
            .map(|r| (r.base, r.base + r.size))
            .collect();

        // Read each region in parallel and collect pointer entries
        let per_region_maps: Vec<Vec<(usize, usize, usize)>> = readable_regions
            .par_iter()
            .filter_map(|region| {
                let mut entries = Vec::new();
                let mut offset = 0usize;
                let mut buf = vec![0u8; READ_BLOCK_SIZE];

                while offset < region.size {
                    let read_size = (region.size - offset).min(READ_BLOCK_SIZE);
                    let addr = region.base + offset;
                    let read_buf = &mut buf[..read_size];

                    match process.read(addr, read_buf) {
                        Ok(bytes_read) => {
                            // Scan for pointer-aligned values
                            let scan_end = if bytes_read >= ptr_size {
                                bytes_read - ptr_size + 1
                            } else {
                                0
                            };
                            let mut i = 0;
                            while i < scan_end {
                                let value = usize::from_le_bytes(
                                    read_buf[i..i + ptr_size].try_into().unwrap(),
                                );
                                // Check if value falls within any region
                                if is_valid_pointer(value, &region_bounds) {
                                    let page = value / 4096;
                                    let pointer_addr = addr + i;
                                    entries.push((page, pointer_addr, value));
                                }
                                i += ptr_size;
                            }
                        }
                        Err(_) => {
                            // Skip unreadable blocks
                        }
                    }
                    offset += read_size;
                }
                if entries.is_empty() {
                    None
                } else {
                    Some(entries)
                }
            })
            .collect();

        // Merge into a single HashMap
        let mut pointer_map: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
        for region_entries in per_region_maps {
            for (page, addr, value) in region_entries {
                pointer_map.entry(page).or_default().push((addr, value));
            }
        }

        Ok(pointer_map)
    }

    /// BFS from target address, looking for pointer chains that end in static regions.
    fn bfs_scan(
        &self,
        pointer_map: &HashMap<usize, Vec<(usize, usize)>>,
        target_address: usize,
        regions: &[MemoryRegion],
    ) -> Vec<PointerChain> {
        let mut results: Vec<PointerChain> = Vec::new();

        // BFS queue: (current_target, offsets_so_far)
        // offsets_so_far is built in reverse — from target back to base
        let mut queue: Vec<(usize, Vec<isize>)> = Vec::new();

        // Seed: find all pointers that point near the target
        let seeds = self.find_pointers_to(pointer_map, target_address);
        for (addr, offset) in seeds {
            let offsets = vec![offset];
            if self.is_static_address(addr, regions) {
                let module = self.get_module_name(addr, regions);
                results.push(PointerChain {
                    base_address: addr,
                    offsets,
                    module,
                });
                if results.len() >= MAX_RESULTS {
                    return results;
                }
            } else {
                queue.push((addr, offsets));
            }
        }

        // BFS: expand the queue level by level up to max_depth
        for _depth in 1..self.max_depth {
            if queue.is_empty() || results.len() >= MAX_RESULTS {
                break;
            }

            let mut next_queue: Vec<(usize, Vec<isize>)> = Vec::new();

            for (current_target, offsets_so_far) in &queue {
                let pointers = self.find_pointers_to(pointer_map, *current_target);
                for (addr, offset) in pointers {
                    let mut new_offsets = vec![offset];
                    new_offsets.extend_from_slice(offsets_so_far);

                    if self.is_static_address(addr, regions) {
                        let module = self.get_module_name(addr, regions);
                        results.push(PointerChain {
                            base_address: addr,
                            offsets: new_offsets,
                            module,
                        });
                        if results.len() >= MAX_RESULTS {
                            return results;
                        }
                    } else if offsets_so_far.len() < self.max_depth - 1 {
                        next_queue.push((addr, new_offsets));
                    }
                }
            }

            queue = next_queue;
        }

        results
    }

    /// Find all (address, offset) pairs where value at `address` points within
    /// `[target - max_offset, target + max_offset]`.
    fn find_pointers_to(
        &self,
        pointer_map: &HashMap<usize, Vec<(usize, usize)>>,
        target: usize,
    ) -> Vec<(usize, isize)> {
        let mut found = Vec::new();

        let low = target.saturating_sub(self.max_offset);
        let high = target.saturating_add(self.max_offset);
        let page_low = low / 4096;
        let page_high = high / 4096;

        for page in page_low..=page_high {
            if let Some(entries) = pointer_map.get(&page) {
                for &(addr, value) in entries {
                    if value >= low && value <= high {
                        let offset = target as isize - value as isize;
                        found.push((addr, offset));
                    }
                }
            }
        }

        found
    }

    /// Check if an address is in a "static" region (module/image).
    fn is_static_address(&self, address: usize, regions: &[MemoryRegion]) -> bool {
        for (i, region) in regions.iter().enumerate() {
            if address >= region.base && address < region.base + region.size {
                // First region is typically the main executable
                if i == 0 {
                    return true;
                }
                // Check if region info contains a module path
                let info = &region.info;
                if info.contains(".exe")
                    || info.contains(".dll")
                    || info.contains(".so")
                    || info.contains(".dylib")
                {
                    return true;
                }
                return false;
            }
        }
        false
    }

    /// Get the module name from the region info for a given address.
    fn get_module_name(&self, address: usize, regions: &[MemoryRegion]) -> Option<String> {
        for region in regions {
            if address >= region.base && address < region.base + region.size {
                if region.info.is_empty() {
                    return None;
                }
                return Some(region.info.clone());
            }
        }
        None
    }
}

/// Resolve a pointer chain: follow the chain of dereferences and offsets.
/// Returns the final resolved address, or None if any read fails.
pub fn resolve_pointer(process: &dyn ProcessMemory, chain: &PointerChain) -> Option<usize> {
    let ptr_size = std::mem::size_of::<usize>();
    let mut addr = chain.base_address;

    for &offset in &chain.offsets {
        let mut buf = [0u8; 8];
        let read_buf = &mut buf[..ptr_size];
        if process.read(addr, read_buf).is_err() {
            return None;
        }
        let ptr = usize::from_le_bytes(buf[..ptr_size].try_into().unwrap());
        addr = (ptr as isize + offset) as usize;
    }

    Some(addr)
}

/// Fast check if a value is a valid pointer (falls within any region bounds).
#[inline]
fn is_valid_pointer(value: usize, region_bounds: &[(usize, usize)]) -> bool {
    // Quick reject: null and very low addresses
    if value < 0x10000 {
        return false;
    }
    region_bounds
        .iter()
        .any(|&(start, end)| value >= start && value < end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PlatformError;

    /// A mock process for testing pointer scanning.
    struct MockProcess {
        memory: Vec<(usize, Vec<u8>)>,
        regions: Vec<MemoryRegion>,
    }

    impl MockProcess {
        fn new() -> Self {
            Self {
                memory: Vec::new(),
                regions: Vec::new(),
            }
        }

        fn add_region(&mut self, base: usize, data: Vec<u8>, info: &str) {
            let size = data.len();
            self.regions.push(MemoryRegion {
                base,
                size,
                readable: true,
                writable: true,
                executable: false,
                info: info.to_string(),
            });
            self.memory.push((base, data));
        }
    }

    impl ProcessMemory for MockProcess {
        fn list_processes() -> Result<Vec<crate::platform::ProcessInfo>, PlatformError> {
            Ok(vec![])
        }

        fn attach(_pid: u32) -> Result<Self, PlatformError> {
            Ok(MockProcess::new())
        }

        fn read(&self, address: usize, buf: &mut [u8]) -> Result<usize, PlatformError> {
            for (base, data) in &self.memory {
                let end = base + data.len();
                if address >= *base && address < end {
                    let offset = address - base;
                    let available = data.len() - offset;
                    let to_read = buf.len().min(available);
                    buf[..to_read].copy_from_slice(&data[offset..offset + to_read]);
                    return Ok(to_read);
                }
            }
            Err(PlatformError::ReadFailed {
                address,
                reason: "not mapped".into(),
            })
        }

        fn write(&self, _address: usize, _data: &[u8]) -> Result<usize, PlatformError> {
            Ok(0)
        }

        fn regions(&self) -> Result<Vec<MemoryRegion>, PlatformError> {
            Ok(self.regions.clone())
        }

        fn pid(&self) -> u32 {
            1
        }
    }

    #[test]
    fn test_resolve_pointer_simple() {
        let ptr_size = std::mem::size_of::<usize>();
        let mut process = MockProcess::new();

        // Set up: base_address (0x1000) contains pointer to 0x2000
        // target is at 0x2000 + 0x10 = 0x2010
        let mut data = vec![0u8; 64];
        let ptr_value: usize = 0x2000;
        data[0..ptr_size].copy_from_slice(&ptr_value.to_le_bytes());
        process.add_region(0x1000, data, "/app.exe");

        let mut target_data = vec![0u8; 64];
        target_data[0x10] = 0x42; // some data at target
        process.add_region(0x2000, target_data, "");

        let chain = PointerChain {
            base_address: 0x1000,
            offsets: vec![0x10],
            module: Some("/app.exe".into()),
        };

        let resolved = resolve_pointer(&process, &chain);
        assert_eq!(resolved, Some(0x2010));
    }

    #[test]
    fn test_resolve_pointer_multi_level() {
        let ptr_size = std::mem::size_of::<usize>();
        let mut process = MockProcess::new();

        // Chain: 0x1000 -> read ptr -> 0x2000 + offset 0x0 -> read ptr -> 0x3000 + offset 0x10
        let mut data1 = vec![0u8; 64];
        let ptr1: usize = 0x2000;
        data1[0..ptr_size].copy_from_slice(&ptr1.to_le_bytes());
        process.add_region(0x1000, data1, "/app.exe");

        let mut data2 = vec![0u8; 64];
        let ptr2: usize = 0x3000;
        data2[0..ptr_size].copy_from_slice(&ptr2.to_le_bytes());
        process.add_region(0x2000, data2, "");

        let data3 = vec![0xABu8; 64];
        process.add_region(0x3000, data3, "");

        let chain = PointerChain {
            base_address: 0x1000,
            offsets: vec![0, 0x10],
            module: Some("/app.exe".into()),
        };

        let resolved = resolve_pointer(&process, &chain);
        assert_eq!(resolved, Some(0x3010));
    }

    #[test]
    fn test_pointer_scan_finds_chain() {
        let ptr_size = std::mem::size_of::<usize>();
        let mut process = MockProcess::new();

        // Static region (module): at 0x10000, contains pointer to 0x20000
        let mut static_data = vec![0u8; 4096];
        let ptr_value: usize = 0x20000;
        static_data[0..ptr_size].copy_from_slice(&ptr_value.to_le_bytes());
        process.add_region(0x10000, static_data, "/game.exe");

        // Heap region: at 0x20000, target is at 0x20000 + 0x40 = 0x20040
        let heap_data = vec![0u8; 4096];
        process.add_region(0x20000, heap_data, "");

        let target = 0x20040;
        let scanner = PointerScanner::new(3, 4096);
        let regions = process.regions().unwrap();
        let results = scanner.scan(&process, target, &regions).unwrap();

        // Should find a chain: base=0x10000, offsets=[0x40]
        // Because 0x10000 holds value 0x20000, and target - 0x20000 = 0x40
        assert!(!results.is_empty());
        let found = results.iter().any(|chain| {
            chain.base_address == 0x10000 && chain.offsets == vec![0x40]
        });
        assert!(found, "Expected to find chain base=0x10000 offset=0x40, got: {:?}", results);
    }

    #[test]
    fn test_pointer_scan_respects_max_offset() {
        let ptr_size = std::mem::size_of::<usize>();
        let mut process = MockProcess::new();

        // Static region: pointer to 0x20000
        let mut static_data = vec![0u8; 4096];
        let ptr_value: usize = 0x20000;
        static_data[0..ptr_size].copy_from_slice(&ptr_value.to_le_bytes());
        process.add_region(0x10000, static_data, "/game.exe");

        let heap_data = vec![0u8; 8192];
        process.add_region(0x20000, heap_data, "");

        // Target is too far from the pointer value (offset = 0x2000 > max_offset=256)
        let target = 0x22000;
        let scanner = PointerScanner::new(3, 256);
        let regions = process.regions().unwrap();
        let results = scanner.scan(&process, target, &regions).unwrap();

        assert!(results.is_empty());
    }
}
