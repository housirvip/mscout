use rayon::prelude::*;

use super::results::{ScanResult, ScanResultSet};
use super::value::{ScanCondition, ScanValue, ValueType};
use crate::platform::{MemoryRegion, PlatformError, ProcessMemory};

/// Block size for reading memory (64 KB).
const READ_BLOCK_SIZE: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("platform error: {0}")]
    Platform(#[from] PlatformError),
    #[error("no scan in progress")]
    NoScanInProgress,
    #[error("invalid scan parameters: {0}")]
    InvalidParams(String),
}

pub struct ScanSession {
    pub results: ScanResultSet,
    pub value_type: ValueType,
    pub alignment: usize,
    pub regions: Vec<MemoryRegion>,
}

impl ScanSession {
    /// Perform the first scan: read all regions, find matches at alignment boundaries.
    pub fn first_scan(
        process: &dyn ProcessMemory,
        value_type: ValueType,
        condition: ScanCondition,
        value: Option<ScanValue>,
        value2: Option<ScanValue>,
        regions: &[MemoryRegion],
        alignment: usize,
    ) -> Result<Self, ScanError> {
        let val_size = value_type.size();
        if val_size == 0 && value.is_none() {
            return Err(ScanError::InvalidParams(
                "ByteArray scan requires a value".into(),
            ));
        }

        let effective_size = if val_size > 0 {
            val_size
        } else {
            value.as_ref().map(|v| v.to_bytes().len()).unwrap_or(1)
        };

        let effective_alignment = if alignment == 0 {
            if val_size > 0 {
                val_size
            } else {
                1
            }
        } else {
            alignment
        };

        // Scan each region in parallel using rayon
        let scan_results: Vec<(Vec<usize>, Vec<u8>)> = regions
            .par_iter()
            .filter(|r| r.readable && r.size >= effective_size)
            .filter_map(|region| {
                let mut region_addresses = Vec::new();
                let mut region_values = Vec::new();
                let mut buf = vec![0u8; READ_BLOCK_SIZE];

                let mut offset = 0usize;
                while offset < region.size {
                    let to_read = (region.size - offset).min(READ_BLOCK_SIZE);
                    let addr = region.base + offset;
                    let read_buf = &mut buf[..to_read];

                    let bytes_read = match process.read(addr, read_buf) {
                        Ok(n) => n,
                        Err(_) => {
                            offset += to_read;
                            continue;
                        }
                    };

                    if bytes_read < effective_size {
                        offset += to_read;
                        continue;
                    }

                    // Scan within the block
                    let mut pos = 0;
                    while pos + effective_size <= bytes_read {
                        let chunk = &read_buf[pos..pos + effective_size];
                        let matched = match condition {
                            ScanCondition::Unknown => true,
                            ScanCondition::Exact
                            | ScanCondition::GreaterThan
                            | ScanCondition::LessThan => {
                                if let Some(val) = &value {
                                    val.matches(chunk, condition, None)
                                } else {
                                    false
                                }
                            }
                            ScanCondition::Between => {
                                if let (Some(v1), Some(v2)) = (&value, &value2) {
                                    v1.matches(chunk, ScanCondition::GreaterThan, None)
                                        || v1.matches(chunk, ScanCondition::Exact, None)
                                        || (v2.matches(chunk, ScanCondition::LessThan, None)
                                            || v2.matches(chunk, ScanCondition::Exact, None))
                                } else {
                                    false
                                }
                            }
                            _ => {
                                // Changed/Unchanged/Increased/Decreased don't apply to first scan
                                // Treat as Unknown for first scan
                                true
                            }
                        };

                        if matched {
                            region_addresses.push(addr + pos);
                            region_values.extend_from_slice(chunk);
                        }

                        pos += effective_alignment;
                    }

                    offset += to_read;
                }

                if region_addresses.is_empty() {
                    None
                } else {
                    Some((region_addresses, region_values))
                }
            })
            .collect();

        // Merge results
        let mut result_set = ScanResultSet::new(value_type);
        for (addrs, vals) in scan_results {
            result_set.addresses.extend(addrs);
            result_set.previous_values.extend(vals);
        }

        Ok(Self {
            results: result_set,
            value_type,
            alignment: effective_alignment,
            regions: regions.to_vec(),
        })
    }

    /// Perform a next scan: re-read values at stored addresses, filter by condition.
    /// Returns the new match count.
    pub fn next_scan(
        &mut self,
        process: &dyn ProcessMemory,
        condition: ScanCondition,
        value: Option<ScanValue>,
        _value2: Option<ScanValue>,
    ) -> Result<usize, ScanError> {
        if self.results.is_empty() {
            return Err(ScanError::NoScanInProgress);
        }

        // Save current state for undo
        self.results.push_history();

        let val_size = self.value_type.size().max(
            value
                .as_ref()
                .map(|v| v.to_bytes().len())
                .unwrap_or(1),
        );

        let mut new_addresses = Vec::new();
        let mut new_values = Vec::new();
        let mut read_buf = vec![0u8; val_size];

        for (i, &addr) in self.results.addresses.iter().enumerate() {
            let prev_start = i * val_size;
            let prev_end = prev_start + val_size;
            let previous = if prev_end <= self.results.previous_values.len() {
                Some(&self.results.previous_values[prev_start..prev_end])
            } else {
                None
            };

            // Read current value
            match process.read(addr, &mut read_buf) {
                Ok(n) if n >= val_size => {}
                _ => continue, // Skip addresses we can't read anymore
            }

            let matched = if let Some(val) = &value {
                val.matches(&read_buf[..val_size], condition, previous)
            } else {
                // For conditions that don't need a target value (Changed, Unchanged, etc.)
                // Create a dummy ScanValue just to invoke the comparison logic
                match condition {
                    ScanCondition::Changed => previous.map_or(false, |p| &read_buf[..val_size] != p),
                    ScanCondition::Unchanged => {
                        previous.map_or(true, |p| &read_buf[..val_size] == p)
                    }
                    ScanCondition::Increased => {
                        if let Some(prev) = previous {
                            compare_raw_bytes(&read_buf[..val_size], prev, self.value_type)
                                == Some(std::cmp::Ordering::Greater)
                        } else {
                            false
                        }
                    }
                    ScanCondition::Decreased => {
                        if let Some(prev) = previous {
                            compare_raw_bytes(&read_buf[..val_size], prev, self.value_type)
                                == Some(std::cmp::Ordering::Less)
                        } else {
                            false
                        }
                    }
                    _ => true,
                }
            };

            if matched {
                new_addresses.push(addr);
                new_values.extend_from_slice(&read_buf[..val_size]);
            }
        }

        self.results.addresses = new_addresses;
        self.results.previous_values = new_values;

        Ok(self.results.len())
    }

    /// Undo the last next_scan operation.
    pub fn undo(&mut self) -> bool {
        self.results.pop_history()
    }

    /// Get results for display (paginated).
    /// Reads current values from the process.
    pub fn get_results(
        &self,
        process: &dyn ProcessMemory,
        offset: usize,
        count: usize,
    ) -> Result<Vec<ScanResult>, ScanError> {
        let total = self.results.len();
        if offset >= total {
            return Ok(Vec::new());
        }

        let end = (offset + count).min(total);
        let val_size = self.value_type.size().max(1);
        let mut results = Vec::with_capacity(end - offset);
        let mut read_buf = vec![0u8; val_size];

        for i in offset..end {
            let addr = self.results.addresses[i];

            // Read current value
            let current_str = match process.read(addr, &mut read_buf) {
                Ok(n) if n >= val_size => format_value(&read_buf[..val_size], self.value_type),
                _ => "???".to_string(),
            };

            // Get previous value
            let prev_start = i * val_size;
            let prev_end = prev_start + val_size;
            let prev_str = if prev_end <= self.results.previous_values.len() {
                format_value(
                    &self.results.previous_values[prev_start..prev_end],
                    self.value_type,
                )
            } else {
                "N/A".to_string()
            };

            results.push(ScanResult {
                address: format!("{:#x}", addr),
                value: current_str,
                previous_value: prev_str,
            });
        }

        Ok(results)
    }

    /// Returns the total number of matches.
    pub fn result_count(&self) -> usize {
        self.results.len()
    }
}

/// Format raw bytes as a human-readable string according to ValueType.
fn format_value(data: &[u8], vt: ValueType) -> String {
    macro_rules! fmt_val {
        ($ty:ty) => {{
            let size = std::mem::size_of::<$ty>();
            if data.len() >= size {
                let val = <$ty>::from_le_bytes(data[..size].try_into().unwrap());
                format!("{}", val)
            } else {
                "???".to_string()
            }
        }};
    }
    match vt {
        ValueType::I8 => fmt_val!(i8),
        ValueType::I16 => fmt_val!(i16),
        ValueType::I32 => fmt_val!(i32),
        ValueType::I64 => fmt_val!(i64),
        ValueType::U8 => fmt_val!(u8),
        ValueType::U16 => fmt_val!(u16),
        ValueType::U32 => fmt_val!(u32),
        ValueType::U64 => fmt_val!(u64),
        ValueType::F32 => fmt_val!(f32),
        ValueType::F64 => fmt_val!(f64),
        ValueType::ByteArray => {
            data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
        }
    }
}

/// Compare two raw byte buffers interpreted as the given value type.
fn compare_raw_bytes(a: &[u8], b: &[u8], vt: ValueType) -> Option<std::cmp::Ordering> {
    macro_rules! cmp_raw {
        ($ty:ty) => {{
            let size = std::mem::size_of::<$ty>();
            if a.len() >= size && b.len() >= size {
                let va = <$ty>::from_le_bytes(a[..size].try_into().unwrap());
                let vb = <$ty>::from_le_bytes(b[..size].try_into().unwrap());
                va.partial_cmp(&vb)
            } else {
                None
            }
        }};
    }
    match vt {
        ValueType::I8 => cmp_raw!(i8),
        ValueType::I16 => cmp_raw!(i16),
        ValueType::I32 => cmp_raw!(i32),
        ValueType::I64 => cmp_raw!(i64),
        ValueType::U8 => cmp_raw!(u8),
        ValueType::U16 => cmp_raw!(u16),
        ValueType::U32 => cmp_raw!(u32),
        ValueType::U64 => cmp_raw!(u64),
        ValueType::F32 => cmp_raw!(f32),
        ValueType::F64 => cmp_raw!(f64),
        ValueType::ByteArray => None,
    }
}
