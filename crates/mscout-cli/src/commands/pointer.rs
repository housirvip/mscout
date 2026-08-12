use std::path::Path;

use anyhow::Result;
use mscout_core::{
    platform::{NativeProcess, ProcessMemory},
    pointer::{PointerChain, PointerScanner, resolve_pointer},
};
use serde_json::json;

use crate::commands::scan::parse_address;
use crate::output::Output;
use crate::session;
use crate::state::CliState;

pub fn pointer_scan(
    addr_str: &str,
    depth: usize,
    max_offset: usize,
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let target = parse_address(addr_str)?;
    let (session_file, _) = session::load_session(session_path)?;
    let process = NativeProcess::attach(session_file.pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let regions = process.regions().map_err(|e| anyhow::anyhow!("{e}"))?;
    let scanner = PointerScanner::new(depth, max_offset);
    let chains = scanner.scan(&process, target, &regions)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    output_chains(&chains, &process, target, out)
}

pub fn pointer_resolve(
    base_str: &str,
    offset_strs: &[String],
    session_path: Option<&Path>,
    out: &Output,
) -> Result<()> {
    let base = parse_address(base_str)?;
    let offsets: Vec<isize> = offset_strs.iter()
        .map(|s| {
            let s = s.trim().strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
            isize::from_str_radix(s, 16).map_err(|e| anyhow::anyhow!("Invalid offset '{s}': {e}"))
        })
        .collect::<Result<Vec<_>>>()?;

    let (session_file, _) = session::load_session(session_path)?;
    let process = NativeProcess::attach(session_file.pid)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let chain = PointerChain { base_address: base, offsets, module: None };
    let resolved = resolve_pointer(&process, &chain);

    if out.json {
        out.success("pointer-resolve", json!({
            "base": format!("0x{:X}", base),
            "offsets": chain.offsets.iter().map(|o| format!("0x{:X}", o)).collect::<Vec<_>>(),
            "resolved": resolved.map(|a| format!("0x{:X}", a)),
        }));
    } else {
        match resolved {
            Some(addr) => println!("Resolved: 0x{:X}", addr),
            None => println!("Could not resolve pointer chain."),
        }
    }
    Ok(())
}

// ─── REPL variants ─────────────────────────────────────────────────────────

pub fn pointer_scan_repl(
    state: &CliState,
    addr_str: &str,
    depth: usize,
    max_offset: usize,
    out: &Output,
) -> Result<()> {
    let process = state.process.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not attached."))?;
    let target = parse_address(addr_str)?;

    let regions = process.regions().map_err(|e| anyhow::anyhow!("{e}"))?;
    let scanner = PointerScanner::new(depth, max_offset);
    let chains = scanner.scan(process.as_ref(), target, &regions)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    output_chains(&chains, process.as_ref(), target, out)
}

pub fn pointer_resolve_repl(
    state: &CliState,
    base_str: &str,
    offset_strs: &[String],
    out: &Output,
) -> Result<()> {
    let process = state.process.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Not attached."))?;
    let base = parse_address(base_str)?;
    let offsets: Vec<isize> = offset_strs.iter()
        .map(|s| {
            let s = s.trim().strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
            isize::from_str_radix(s, 16).map_err(|e| anyhow::anyhow!("Invalid offset '{s}': {e}"))
        })
        .collect::<Result<Vec<_>>>()?;

    let chain = PointerChain { base_address: base, offsets, module: None };
    let resolved = resolve_pointer(process.as_ref(), &chain);

    if out.json {
        out.success("pointer-resolve", json!({
            "base": format!("0x{:X}", base),
            "offsets": chain.offsets.iter().map(|o| format!("0x{:X}", o)).collect::<Vec<_>>(),
            "resolved": resolved.map(|a| format!("0x{:X}", a)),
        }));
    } else {
        match resolved {
            Some(addr) => println!("Resolved: 0x{:X}", addr),
            None => println!("Could not resolve pointer chain."),
        }
    }
    Ok(())
}

// ─── Helper ────────────────────────────────────────────────────────────────

fn output_chains(
    chains: &[PointerChain],
    process: &dyn ProcessMemory,
    target: usize,
    out: &Output,
) -> Result<()> {
    if out.json {
        let data: Vec<_> = chains.iter().map(|c| {
            let resolved = resolve_pointer(process, c);
            json!({
                "base": format!("0x{:X}", c.base_address),
                "module": c.module,
                "offsets": c.offsets.iter().map(|o| format!("0x{:X}", o)).collect::<Vec<_>>(),
                "resolved": resolved.map(|a| format!("0x{:X}", a)),
            })
        }).collect();
        out.success("pointer-scan", json!({"target": format!("0x{:X}", target), "chains": data}));
    } else {
        println!("Found {} pointer chain(s) for 0x{:X}:", chains.len(), target);
        let rows: Vec<Vec<String>> = chains.iter().map(|c| {
            let resolved = resolve_pointer(process, c);
            vec![
                format!("0x{:X}", c.base_address),
                c.module.clone().unwrap_or_default(),
                c.offsets.iter().map(|o| format!("0x{:X}", o)).collect::<Vec<_>>().join(" → "),
                resolved.map(|a| format!("0x{:X}", a)).unwrap_or_else(|| "—".into()),
            ]
        }).collect();
        out.print_table(&["Base", "Module", "Offsets", "Resolved"], rows);
    }
    Ok(())
}
