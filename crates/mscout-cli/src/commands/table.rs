use std::path::Path;

use anyhow::Result;
use mscout_core::table::{AddressSpec, CheatTable, TableEntry};
use serde_json::json;

use crate::commands::scan::{parse_address, parse_value_type};
use crate::output::Output;
use crate::session;

pub fn save_table(path: &Path, session_path: Option<&Path>, out: &Output) -> Result<()> {
    let (session_file, _) = session::load_session(session_path)?;
    let table = session_file.table.unwrap_or_else(|| CheatTable::new(session_file.process_name.clone()));
    table.save_to_file(path).map_err(|e| anyhow::anyhow!("{e}"))?;

    if out.json {
        out.success("table-save", json!({"path": path.display().to_string(), "entries": table.entries.len()}));
    } else {
        println!("Saved {} entries to {}", table.entries.len(), path.display());
    }
    Ok(())
}

pub fn load_table(path: &Path, session_path: Option<&Path>, out: &Output) -> Result<()> {
    let table = CheatTable::load_from_file(path).map_err(|e| anyhow::anyhow!("{e}"))?;

    // Store in session if we have one
    if let Ok((mut session_file, spath)) = session::load_session(session_path) {
        session_file.table = Some(table.clone());
        session::save_session(&session_file, Some(&spath))?;
    }

    if out.json {
        let entries: Vec<_> = table.entries.iter().map(|e| {
            json!({
                "label": e.label,
                "address": format!("{:?}", e.address),
                "type": format!("{:?}", e.value_type),
                "frozen": e.frozen,
            })
        }).collect();
        out.success("table-load", json!({"path": path.display().to_string(), "process": table.process_name, "entries": entries}));
    } else {
        println!("Loaded table for '{}' ({} entries)", table.process_name, table.entries.len());
        let rows: Vec<Vec<String>> = table.entries.iter().enumerate().map(|(i, e)| {
            vec![
                i.to_string(),
                e.label.clone(),
                format!("{:?}", e.address),
                format!("{:?}", e.value_type),
                if e.frozen { "✓" } else { "" }.to_string(),
            ]
        }).collect();
        out.print_table(&["#", "Label", "Address", "Type", "Frozen"], rows);
    }
    Ok(())
}

pub fn list_table(session_path: Option<&Path>, out: &Output) -> Result<()> {
    let (session_file, _) = session::load_session(session_path)?;
    let table = session_file.table.unwrap_or_else(|| CheatTable::new(session_file.process_name.clone()));

    if out.json {
        let entries: Vec<_> = table.entries.iter().map(|e| {
            json!({
                "label": e.label,
                "address": format!("{:?}", e.address),
                "type": format!("{:?}", e.value_type),
                "frozen": e.frozen,
            })
        }).collect();
        out.success("table-list", json!({"entries": entries}));
    } else {
        if table.entries.is_empty() {
            println!("Table is empty.");
        } else {
            let rows: Vec<Vec<String>> = table.entries.iter().enumerate().map(|(i, e)| {
                vec![
                    i.to_string(),
                    e.label.clone(),
                    format!("{:?}", e.address),
                    format!("{:?}", e.value_type),
                    if e.frozen { "✓" } else { "" }.to_string(),
                ]
            }).collect();
            out.print_table(&["#", "Label", "Address", "Type", "Frozen"], rows);
        }
    }
    Ok(())
}

pub fn add_entry(
    addr_str: &str,
    type_str: &str,
    label: Option<&str>,
    _pointer_spec: Option<&str>,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let address = parse_address(addr_str)?;
    let value_type = parse_value_type(type_str)?;
    let label_str = label.unwrap_or("").to_string();

    let (mut session_file, spath) = session::load_session(session_path)?;
    let process_name = session_file.process_name.clone();
    let table = session_file.table.get_or_insert_with(|| CheatTable::new(process_name));

    let entry = TableEntry {
        label: label_str.clone(),
        address: AddressSpec::Static(address),
        value_type,
        frozen: false,
        freeze_value: None,
    };
    table.entries.push(entry);
    session::save_session(&session_file, Some(&spath))?;

    if out.json {
        out.success("table-add", json!({"address": format!("0x{:X}", address), "type": type_str, "label": label_str}));
    } else {
        println!("Added 0x{:X} ({}) to table", address, label_str);
    }
    Ok(())
}

pub fn remove_entry(
    address: Option<&str>,
    index: Option<usize>,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let (mut session_file, spath) = session::load_session(session_path)?;
    let process_name = session_file.process_name.clone();
    let table = session_file.table.get_or_insert_with(|| CheatTable::new(process_name));

    if let Some(idx) = index {
        if idx >= table.entries.len() {
            anyhow::bail!("Index {} out of range (table has {} entries)", idx, table.entries.len());
        }
        table.entries.remove(idx);
    } else if let Some(addr_str) = address {
        let addr = parse_address(addr_str)?;
        let before = table.entries.len();
        table.entries.retain(|e| {
            match &e.address {
                AddressSpec::Static(a) => *a != addr,
                _ => true,
            }
        });
        if table.entries.len() == before {
            anyhow::bail!("No entry found with address 0x{:X}", addr);
        }
    } else {
        anyhow::bail!("Provide --address or --index");
    }

    let remaining = session_file.table.as_ref().map(|t| t.entries.len()).unwrap_or(0);
    session::save_session(&session_file, Some(&spath))?;

    if out.json {
        out.success("table-rm", json!({"entries": remaining}));
    } else {
        println!("Entry removed. {} entries remaining.", remaining);
    }
    Ok(())
}
