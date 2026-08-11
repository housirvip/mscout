use crate::scanner::{ScanValue, ValueType};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CheatTable {
    pub version: u32,
    pub process_name: String,
    pub entries: Vec<TableEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct TableEntry {
    pub label: String,
    pub address: AddressSpec,
    pub value_type: ValueType,
    pub frozen: bool,
    pub freeze_value: Option<ScanValue>,
}

#[derive(Serialize, Deserialize)]
pub enum AddressSpec {
    Static(usize),
    Pointer {
        module: String,
        base_offset: usize,
        offsets: Vec<isize>,
    },
}

impl CheatTable {
    pub fn new(process_name: String) -> Self {
        Self {
            version: 1,
            process_name,
            entries: Vec::new(),
        }
    }
}

impl CheatTable {
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let table: Self = serde_json::from_str(&json)?;
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_table() {
        let table = CheatTable {
            version: 1,
            process_name: "test.exe".into(),
            entries: vec![TableEntry {
                label: "Health".into(),
                address: AddressSpec::Static(0x12345),
                value_type: ValueType::I32,
                frozen: false,
                freeze_value: None,
            }],
        };

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.mst");

        table.save_to_file(&path).unwrap();
        let loaded = CheatTable::load_from_file(&path).unwrap();

        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.process_name, "test.exe");
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].label, "Health");
    }
}
