#[cfg(test)]
mod tests {
    use crate::platform::{MemoryRegion, PlatformError, ProcessInfo, ProcessMemory};
    use crate::scanner::{ScanCondition, ScanSession, ScanValue, ValueType};
    use std::sync::Mutex;

    /// Mock process memory backed by a Vec<u8>
    struct MockProcess {
        data: Mutex<Vec<u8>>,
    }

    impl MockProcess {
        fn new(data: Vec<u8>) -> Self {
            Self {
                data: Mutex::new(data),
            }
        }
    }

    impl ProcessMemory for MockProcess {
        fn list_processes() -> Result<Vec<ProcessInfo>, PlatformError> {
            Ok(vec![])
        }

        fn attach(_pid: u32) -> Result<Self, PlatformError> {
            Ok(Self::new(vec![]))
        }

        fn read(&self, address: usize, buf: &mut [u8]) -> Result<usize, PlatformError> {
            let data = self.data.lock().unwrap();
            let end = (address + buf.len()).min(data.len());
            if address >= data.len() {
                return Err(PlatformError::ReadFailed {
                    address,
                    reason: "out of bounds".into(),
                });
            }
            let len = end - address;
            buf[..len].copy_from_slice(&data[address..end]);
            Ok(len)
        }

        fn write(&self, address: usize, data_in: &[u8]) -> Result<usize, PlatformError> {
            let mut data = self.data.lock().unwrap();
            let end = (address + data_in.len()).min(data.len());
            if address >= data.len() {
                return Err(PlatformError::WriteFailed {
                    address,
                    reason: "out of bounds".into(),
                });
            }
            let len = end - address;
            data[address..end].copy_from_slice(&data_in[..len]);
            Ok(len)
        }

        fn regions(&self) -> Result<Vec<MemoryRegion>, PlatformError> {
            let data = self.data.lock().unwrap();
            Ok(vec![MemoryRegion {
                base: 0,
                size: data.len(),
                readable: true,
                writable: true,
                executable: false,
                info: String::new(),
            }])
        }

        fn pid(&self) -> u32 {
            999
        }
    }

    #[test]
    fn test_first_scan_exact_i32() {
        // Create memory with known i32 values
        let mut data = vec![0u8; 256];
        // Place value 42 at offsets 16, 80, and 200
        data[16..20].copy_from_slice(&42i32.to_le_bytes());
        data[80..84].copy_from_slice(&42i32.to_le_bytes());
        data[200..204].copy_from_slice(&42i32.to_le_bytes());
        // Place other values
        data[0..4].copy_from_slice(&100i32.to_le_bytes());
        data[4..8].copy_from_slice(&999i32.to_le_bytes());

        let process = MockProcess::new(data);
        let regions = process.regions().unwrap();

        let session = ScanSession::first_scan(
            &process,
            ValueType::I32,
            ScanCondition::Exact,
            Some(ScanValue::I32(42)),
            None,
            &regions,
            4, // alignment
        )
        .unwrap();

        assert_eq!(session.results.len(), 3);
    }

    #[test]
    fn test_next_scan_changed() {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&10i32.to_le_bytes());
        data[4..8].copy_from_slice(&20i32.to_le_bytes());
        data[8..12].copy_from_slice(&30i32.to_le_bytes());

        let process = MockProcess::new(data);
        let regions = process.regions().unwrap();

        // First scan: Unknown (captures all)
        let mut session = ScanSession::first_scan(
            &process,
            ValueType::I32,
            ScanCondition::Unknown,
            None,
            None,
            &regions,
            4,
        )
        .unwrap();

        // Should have captured all aligned i32 addresses
        let initial_count = session.results.len();
        assert!(initial_count > 0);

        // Change one value
        process.write(4, &99i32.to_le_bytes()).unwrap();

        // Next scan: Changed
        let new_count = session
            .next_scan(&process, ScanCondition::Changed, None, None)
            .unwrap();

        // Only the changed address should remain
        assert_eq!(new_count, 1);
    }

    #[test]
    fn test_undo_scan() {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&42i32.to_le_bytes());
        data[4..8].copy_from_slice(&42i32.to_le_bytes());
        data[8..12].copy_from_slice(&42i32.to_le_bytes());

        let process = MockProcess::new(data);
        let regions = process.regions().unwrap();

        let mut session = ScanSession::first_scan(
            &process,
            ValueType::I32,
            ScanCondition::Exact,
            Some(ScanValue::I32(42)),
            None,
            &regions,
            4,
        )
        .unwrap();

        assert_eq!(session.results.len(), 3);

        // Change one value and do next scan
        process.write(0, &99i32.to_le_bytes()).unwrap();
        session
            .next_scan(
                &process,
                ScanCondition::Exact,
                Some(ScanValue::I32(42)),
                None,
            )
            .unwrap();
        assert_eq!(session.results.len(), 2);

        // Undo should restore to 3
        assert!(session.undo());
        assert_eq!(session.results.len(), 3);
    }

    #[test]
    fn test_scan_greater_than() {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&5i32.to_le_bytes());
        data[4..8].copy_from_slice(&15i32.to_le_bytes());
        data[8..12].copy_from_slice(&25i32.to_le_bytes());
        data[12..16].copy_from_slice(&3i32.to_le_bytes());

        let process = MockProcess::new(data);
        let regions = process.regions().unwrap();

        let session = ScanSession::first_scan(
            &process,
            ValueType::I32,
            ScanCondition::GreaterThan,
            Some(ScanValue::I32(10)),
            None,
            &regions,
            4,
        )
        .unwrap();

        // Only 15 and 25 are > 10
        assert_eq!(session.results.len(), 2);
    }
}
