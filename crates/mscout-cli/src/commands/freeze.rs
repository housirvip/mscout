use std::path::Path;

use anyhow::Result;
use mscout_core::{
    freeze::FreezeManager,
    platform::{NativeProcess, ProcessMemory},
};
use serde_json::json;

use crate::commands::scan::{parse_address, parse_scan_value, parse_value_type};
use crate::output::Output;
use crate::session;
use crate::state::CliState;

/// Freeze a value (REPL mode — uses in-process FreezeManager).
pub fn freeze_repl(
    state: &mut CliState,
    addr_str: &str,
    value_str: &str,
    type_str: &str,
    label: Option<&str>,
    out: &Output,
) -> Result<()> {
    let process = state.process.clone()
        .ok_or_else(|| anyhow::anyhow!("Not attached."))?;
    let address = parse_address(addr_str)?;
    let value_type = parse_value_type(type_str)?;
    let value = parse_scan_value(value_str, value_type)?;
    let label_str = label.unwrap_or("").to_string();

    // Create FreezeManager if needed
    if state.freeze_manager.is_none() {
        state.freeze_manager = Some(FreezeManager::start(process));
    }

    let fm = state.freeze_manager.as_ref().unwrap();
    fm.add(address, value, label_str.clone());

    if out.json {
        out.success("freeze", json!({"address": format!("0x{:X}", address), "value": value_str, "type": type_str, "label": label_str}));
    } else {
        println!("Frozen 0x{:X} = {} ({})", address, value_str, type_str);
    }
    Ok(())
}

/// Unfreeze (REPL mode).
pub fn unfreeze_repl(
    state: &mut CliState,
    addr_str: Option<&str>,
    all: bool,
    out: &Output,
) -> Result<()> {
    if all {
        if let Some(mut fm) = state.freeze_manager.take() {
            fm.stop();
        }
        if out.json {
            out.success("unfreeze", json!({"all": true}));
        } else {
            println!("All addresses unfrozen.");
        }
        return Ok(());
    }

    let addr_str = addr_str.ok_or_else(|| anyhow::anyhow!("Provide an address or --all"))?;
    let address = parse_address(addr_str)?;

    let fm = state.freeze_manager.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No frozen addresses."))?;
    fm.remove(address);

    if out.json {
        out.success("unfreeze", json!({"address": format!("0x{:X}", address)}));
    } else {
        println!("Unfrozen 0x{:X}", address);
    }
    Ok(())
}

/// List frozen entries (REPL mode).
pub fn frozen_repl(state: &CliState, out: &Output) -> Result<()> {
    let entries = match &state.freeze_manager {
        Some(fm) => fm.list(),
        None => Vec::new(),
    };

    if out.json {
        let data: Vec<_> = entries.iter().map(|e| {
            json!({
                "address": format!("0x{:X}", e.address),
                "value": format!("{:?}", e.value),
                "enabled": e.enabled,
                "label": e.label,
            })
        }).collect();
        out.success("frozen", data);
    } else {
        if entries.is_empty() {
            println!("No frozen addresses.");
        } else {
            let rows: Vec<Vec<String>> = entries.iter().map(|e| {
                vec![
                    format!("0x{:X}", e.address),
                    format!("{:?}", e.value),
                    if e.enabled { "✓" } else { "✗" }.to_string(),
                    e.label.clone(),
                ]
            }).collect();
            out.print_table(&["Address", "Value", "Enabled", "Label"], rows);
        }
    }
    Ok(())
}

// ─── Subcommand mode (simplified: no daemon for now, just write once) ──────

/// Freeze in subcommand mode. Spawns a background process that keeps writing.
/// For v1: we simply write the value once and note it. Full daemon is future work.
pub fn freeze_cmd(
    addr_str: &str,
    value_str: &str,
    type_str: &str,
    _interval: u64,
    label: Option<&str>,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let address = parse_address(addr_str)?;
    let value_type = parse_value_type(type_str)?;
    let value = parse_scan_value(value_str, value_type)?;
    let bytes = value.to_bytes();
    let label_str = label.unwrap_or("").to_string();

    let (session_file, _) = session::load_session(session_path)?;
    let process = NativeProcess::attach(session_file.pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Write the value immediately
    process.write(address, &bytes).map_err(|e| anyhow::anyhow!("{e}"))?;

    // TODO: In future, spawn a background daemon for continuous write loop.
    // For now, the freeze command writes once. Use REPL mode for continuous freeze.

    if out.json {
        out.success("freeze", json!({
            "address": format!("0x{:X}", address),
            "value": value_str,
            "type": type_str,
            "label": label_str,
            "note": "Value written once. Use REPL mode for continuous freeze."
        }));
    } else {
        println!("Wrote {} to 0x{:X} (type={})", value_str, address, type_str);
        println!("Note: Use `mscout repl` for continuous freeze (background write loop).");
    }
    Ok(())
}

pub fn unfreeze_cmd(
    _addr_str: Option<&str>,
    _all: bool,
    _session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    if out.json {
        out.success("unfreeze", json!({"note": "Freeze daemon not running in subcommand mode. Use REPL for continuous freeze."}));
    } else {
        println!("No freeze daemon running in subcommand mode.");
        println!("Use `mscout repl` for continuous freeze.");
    }
    Ok(())
}

pub fn frozen_cmd(_session_path: Option<&Path>, out: &Output) -> Result<()> {
    if out.json {
        out.success("frozen", json!([]));
    } else {
        println!("No freeze daemon running in subcommand mode. Use `mscout repl`.");
    }
    Ok(())
}
