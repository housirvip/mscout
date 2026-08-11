use std::path::PathBuf;

use mem_scanner_core::table::CheatTable;

#[tauri::command]
pub fn save_table(path: String, table: CheatTable) -> Result<(), String> {
    table
        .save_to_file(&PathBuf::from(path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_table(path: String) -> Result<CheatTable, String> {
    CheatTable::load_from_file(&PathBuf::from(path)).map_err(|e| e.to_string())
}
